extern crate piston_window;
extern crate rand;
extern crate nalgebra as na;
extern crate reactive;

use std::{cmp, f64};
use std::sync::{Arc, Mutex, Condvar};
use piston_window::*;
use rand::{Rng, XorShiftRng, weak_rng};

use reactive::process::{Process, ProcessMut};
use reactive::process::{value_proc, join_all};
use reactive::process::execute_process_parallel_with_main;
use reactive::process::LoopStatus::{Continue, Exit};
use reactive::signal::ValuedSignal;
use reactive::signal::parallel::{SpmcSignalPl, MpmcSignalPl};

const WIDTH: usize = 80;
const HEIGHT: usize = 60;
const NUM_AGENTS: usize = 50;
const MAX_SUGAR: usize = 6;

fn main() {

    let display_config = DisplayConfig {
        max_sugar: 6,
        sugar_color: [1.0, 0.7, 0.1, 1.0],
        agent_color: [0.2, 0.1, 1.0, 0.6],
    };

    let sugar_dist = gen_sugar_capcity(
        vec![(7, 10), (23, 46), (52, 33)], MAX_SUGAR, 
        vec![3., 7., 13., 20., 30., 50.], WIDTH, HEIGHT);

    let sim = Simulation {
        sugar_capacity: sugar_dist.clone(),
        sugar_grow_back_rate: 1,
        sugar_grow_back_interval: 1,
    };

    let grid = grid::Grid {
        cols: WIDTH as u32,
        rows: HEIGHT as u32,
        units: 10.0,
    };

    let mut possible_cells: Vec<_> = grid.cells().collect();
    weak_rng().shuffle(&mut possible_cells);
    let init_aps: Vec<_> = 
        possible_cells.iter().take(NUM_AGENTS)
        .map(|&(x, y)| (x as usize, y as usize)).collect();

    let gs = GlobalState {
        sugar_dist: sugar_dist,
        occupied: na::DMatrix::from_element(80, 60, 0),
        aps: init_aps.clone(),
        tick: 0,
    };

    let gs = Arc::new(Mutex::new(gs));

    let sd_signal = SpmcSignalPl::new();
    let sd_signal_rev = sd_signal.clone();

    let occupied = SpmcSignalPl::new();
    let occupied_rev = occupied.clone();

    let tick = SpmcSignalPl::new();
    let tick_rev = tick.clone();
    
    let aps_signal = MpmcSignalPl::default();
    let aps_signal_rev = aps_signal.clone();

    let mut confirm = Vec::new();
    for _ in 0..NUM_AGENTS {
        confirm.push(SpmcSignalPl::new());
    }

    let mut agents = Vec::new();

    for (i, &agent_pos) in init_aps.iter().enumerate() {
        let pos = (agent_pos.0 as usize, agent_pos.1 as usize);
        let ag = Agent {
            pos: pos,
            next_pos: pos,
            sugar: 10,
            sugar_metabolism: 3,
            vision: 4,
            rng: weak_rng(),
        };
        let ag = Arc::new(Mutex::new(ag));

        let occupied_rev = occupied_rev.clone();
        let confirm = confirm[i].clone();
        let aps_signal = aps_signal.clone();
        let ag_cl = ag.clone();
        
        let agent_alive = move |sugar_dist: Arc<na::DMatrix<usize>>| {
            let ag_cl2 = ag_cl.clone();
            let aps_signal = aps_signal.clone();
            let sd_cl = sugar_dist.clone();
            let seek_next_pos = move |occupied: Arc<na::DMatrix<usize>>| {
                let agent_pos_update = ag_cl2.lock().unwrap().move_to(&sd_cl, &occupied);
                aps_signal.emit((i, agent_pos_update))
            };
            let ag_cl2 = ag_cl.clone();
            let update_ag_pos = move |()| {
                ag_cl2.lock().unwrap().update_pos_sugar(&sugar_dist);
                Exit(Continue)
            };
            occupied_rev
            .await()
            .and_then(seek_next_pos)
            // Must wait next instant to get a confirmation.
            .pause()
            .then(confirm.await())
            .if_else(value_proc(()).map(update_ag_pos), value_proc(Continue))
            .while_proc()
        };

        let definitely_move = move |sugar_dist: Arc<na::DMatrix<usize>>| {
            let alive = ag.lock().unwrap().metabolise();
            value_proc(alive).if_else(agent_alive(sugar_dist), value_proc(Exit(())))
        };

        let tick_rev = tick_rev.clone();
        let await_tick = move |sugar_dist: Arc<na::DMatrix<usize>>| {
            tick_rev
            .await()
            .if_else(definitely_move(sugar_dist), value_proc(Exit(())))
        };
        
        let agent_proc =
            sd_signal_rev
            .await()
            .and_then(await_tick)
            .while_proc();

        agents.push(agent_proc);
    }

    let gs_cl = gs.clone();
    let broadcast_sugar_dist = move |()| {
        sim.update_global_state(&mut *gs_cl.lock().unwrap());
        let sugar_dist = gs_cl.lock().unwrap().sugar_dist.clone();
        sd_signal.emit(Arc::new(sugar_dist))
    };

    let gs_cl = gs.clone();
    let reset_aps = move |()| gs_cl.lock().unwrap().aps = Vec::new();

    let gs_cl = gs.clone();
    let collision_detect = move |aps_update| {
        let (confirmed, rejected) = gs_cl.lock().unwrap().update_aps(aps_update);
        let confirm_signals = confirmed.iter().map(|&i| confirm[i].emit(true));
        let reject_signals = rejected.iter().map(|&i| confirm[i].emit(false));
        let confirmations = join_all(confirm_signals.chain(reject_signals));
        let all_updated = rejected.is_empty();
        confirmations.then(
            value_proc(all_updated).if_else(value_proc(Exit(Continue)), value_proc(Continue)))
    };

    let gs_cl = gs.clone();
    let occupied = occupied.clone();
    let broadcast_occupied = move |()| {
        let occup = gs_cl.lock().unwrap().occupied.clone();
        occupied.emit(Arc::new(occup))
    };

    let update_aps =
        value_proc(())
        .and_then(broadcast_occupied)
        .then(aps_signal_rev.await())
        .and_then(collision_detect)
        .while_proc();

    let update_patches = 
        value_proc(())
        .and_then(broadcast_sugar_dist)
        .then(tick_rev.await())
        .if_else(value_proc(()).map(reset_aps).then(update_aps), value_proc(Exit(())))
        .while_proc();

    let simulation_updated = Arc::new((Mutex::new(false), Condvar::new()));
    let simulation_updated_cl = simulation_updated.clone();
    let window_updated = Arc::new((Mutex::new(WindowUpdated::NotYet), Condvar::new()));
    let window_updated_cl = window_updated.clone();

    let inform_simulation_updated = move |_| {
        let (ref updated, ref cvar) = *simulation_updated_cl;
        *updated.lock().unwrap() = true;
        cvar.notify_one();
        let (ref lock, ref cvar) = *window_updated_cl;
        let mut updated = lock.lock().unwrap();
        while *updated == WindowUpdated::NotYet {
            updated = cvar.wait(updated).unwrap();
        }
        match *updated {
            WindowUpdated::Done => {
                *updated = WindowUpdated::NotYet;
                return tick.emit(true).then(value_proc(Continue));
            },
            WindowUpdated::End => {
                return tick.emit(false).then(value_proc(Exit(())));
            },
            WindowUpdated::NotYet => panic!("This shouldn't happen!!!"),
        }
    };

    let inform_updated = 
        sd_signal_rev
        .await()
        .and_then(inform_simulation_updated)
        .pause()
        .while_proc();
    
    let mut window: PistonWindow = 
        WindowSettings::new("Hello", ((WIDTH as u32)*10, (HEIGHT as u32)*10))
        .exit_on_esc(true)
        .build()
        .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));
    window.set_max_fps(8);

    let update_window_clos = || {
        let mut window_updated_local = true;
        while let Some(e) = window.next() {
            if window_updated_local {
                let (ref lock, ref cvar) = *simulation_updated;
                let mut updated = lock.lock().unwrap();
                while !*updated {
                    updated = cvar.wait(updated).unwrap();
                }
                *updated = false;
                window_updated_local = false;
            }
            window.draw_2d(&e, |c, g| {
                clear(color::WHITE, g);
                update_window(c, g, &gs.lock().unwrap(), &display_config, &grid);
                window_updated_local = true;
            });
            if window_updated_local {
                let (ref updated, ref cvar) = *window_updated;
                *updated.lock().unwrap() = WindowUpdated::Done;
                cvar.notify_one();
            }
        }
        let (ref updated, ref cvar) = *window_updated;
        *updated.lock().unwrap() = WindowUpdated::End;
        cvar.notify_one();
    };
    
    let agents_proc = join_all(agents);

    execute_process_parallel_with_main(
        update_window_clos, update_patches.join(inform_updated).join(agents_proc), 4);
}

#[derive(PartialEq)]
enum WindowUpdated {
    Done,
    NotYet,
    End,
}

struct GlobalState {
    sugar_dist: na::DMatrix<usize>,
    occupied: na::DMatrix<usize>,
    aps: Vec<(usize, usize)>,
    tick: usize,
}

impl GlobalState {
    fn update_aps(
        &mut self, aps_update: Vec<(usize, ((usize, usize), (usize, usize)))>)
        -> (Vec<usize>, Vec<usize>)
    {
        let mut confirmed = Vec::new();
        let mut rejected = Vec::new();
        for &(id, (old_pos, new_pos)) in aps_update.iter() {
            if id == NUM_AGENTS {
                continue
            } else if self.occupied[new_pos] == 0 || self.occupied[new_pos] == id + 1 {
                self.occupied[new_pos] = id + 1;
                self.occupied[old_pos] = 0;
                self.aps.push(new_pos);
                confirmed.push(id);
            } else {
                rejected.push(id);
            }
        }
        (confirmed, rejected)
    }
}


struct Simulation {
    sugar_capacity: na::DMatrix<usize>,
    sugar_grow_back_rate: usize,
    sugar_grow_back_interval: usize,
}

impl Simulation {
    fn update_global_state(&self, gs: &mut GlobalState) {
        for &agent_pos in gs.aps.iter() {
            gs.sugar_dist[agent_pos] = 0;
        }
        gs.tick = (gs.tick+1) % self.sugar_grow_back_interval;
        if gs.tick == 0 {
            gs.sugar_dist = gs.sugar_dist.zip_map(
                &self.sugar_capacity,
                |sugar, sugar_capacity|
                    cmp::min(sugar+self.sugar_grow_back_rate, sugar_capacity))
        }
    }
}

fn distance(pos1: (usize, usize), pos2: (usize, usize)) -> f64 {
    let x_dist = pos1.0 as f64 - pos2.0 as f64;
    let y_dist = pos1.1 as f64 - pos2.1 as f64;
    (x_dist.powi(2) + y_dist.powi(2)).sqrt()
}

/// `rayons` is supposed to be sorted and have length `max_sugar`.
fn gen_sugar_capcity(
    centers: Vec<(usize, usize)>,
    max_sugar: usize,
    rayons: Vec<f64>,
    width: usize,
    height: usize) -> na::DMatrix<usize>
{
    let decide_sugar_amount = |x, y| {
        let min_dist = 
            centers
            .iter()
            .map(|&center| distance(center, (x, y)))
            .fold(f64::INFINITY, f64::min);
        let nth_circle = rayons.iter().position(|&x| x > min_dist);
        nth_circle.map_or(0, |n| max_sugar - n)
    };
    na::DMatrix::from_fn(width, height, decide_sugar_amount)
}

struct Agent {
    pos: (usize, usize),
    next_pos: (usize, usize),
    sugar: usize,
    sugar_metabolism: usize,
    vision: usize,
    rng: XorShiftRng,
}

fn l1_distance(pos1: (usize, usize), pos2: (usize, usize)) -> usize {
    let x_dist = cmp::max(pos1.0, pos2.0) - cmp::min(pos1.0, pos2.0);
    let y_dist = cmp::max(pos1.1, pos2.1) - cmp::min(pos1.1, pos2.1);
    x_dist + y_dist
}

impl Agent {
    fn can_see(&self, max_width: usize, max_height: usize) -> Vec<(usize, usize)> {
        let (x, y) = self.pos;
        let y_min = y.saturating_sub(self.vision);
        let y_max = cmp::min(y + self.vision, max_height);
        let x_min = x.saturating_sub(self.vision);
        let x_max = cmp::min(x + self.vision, max_width);
        let move_along_x = (x_min..x).chain(x+1..x_max+1).map(|x| (x, y));
        let move_along_y = (y_min..y).chain(y+1..y_max+1).map(|y| (x, y));
        move_along_x.chain(move_along_y).collect()
    }

    /// Returns a boolean to indiacate if the agent is still alive.
    fn metabolise(&mut self) -> bool {
        self.sugar = self.sugar.saturating_sub(self.sugar_metabolism);
        self.sugar > 0
    }

    fn move_to(&mut self,
               sugar_dist: &na::DMatrix<usize>,
               occupied: &na::DMatrix<usize>) -> ((usize, usize), (usize, usize))
    {
        let self_pos = self.pos.clone();
        let mut current_most = sugar_dist[self_pos];
        let mut min_d = 0;
        let mut possible_next_pos = vec![self_pos];
        for &pos in self.can_see(sugar_dist.nrows()-1, sugar_dist.ncols()-1).iter() {
            if occupied[pos] == 0 {
                let d = l1_distance(pos, self.pos);
                if sugar_dist[pos] > current_most {
                    current_most = sugar_dist[pos];
                    min_d = d;
                    possible_next_pos = vec![pos];
                } else if sugar_dist[pos] == current_most {
                    if d < min_d {
                        min_d = d;
                        possible_next_pos = vec![pos];
                    } else if d == min_d {
                        possible_next_pos.push(pos);
                    }
                }
            }
        }
        self.next_pos = self.rng.choose(&possible_next_pos).unwrap().clone();
        (self.pos, self.next_pos)
    }

    fn update_pos_sugar(&mut self, sugar_dist: &na::DMatrix<usize>) {
        self.pos = self.next_pos;
        self.sugar += sugar_dist[self.next_pos];
    }
}

struct DisplayConfig {
    max_sugar: usize,
    sugar_color: types::Color,
    agent_color: types::Color,
}

impl DisplayConfig {
    fn compute_cell_color(&self, sugar_amount: usize) -> types::Color {
        assert!(sugar_amount <= self.max_sugar,
                "The maximum sugar capacity is depassed!");
        let fraction = (sugar_amount as f32)/(self.max_sugar as f32);
        self.sugar_color.mul_rgba(1.0, 1.0, 1.0, fraction)
    }
}

fn update_window(
    c: Context,
    g: &mut G2d,
    gs: &GlobalState,
    config: &DisplayConfig,
    grid: &grid::Grid)
{
    for cell in grid.cells() {
        let color = config.compute_cell_color(
            gs.sugar_dist[(cell.0 as usize, cell.1 as usize)]);
        let coord = grid.cell_position(cell);
        rectangle(color, [coord[0], coord[1], grid.units, grid.units], c.transform, g);
    }
    for &agent_pos in gs.aps.iter() {
        let coord = grid.cell_position((agent_pos.0 as u32, agent_pos.1 as u32));
        ellipse(config.agent_color, 
                [coord[0], coord[1], grid.units, grid.units], c.transform, g);
    }
}

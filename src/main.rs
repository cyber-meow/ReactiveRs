#![type_length_limit="4194304"]

extern crate piston_window;
extern crate find_folder;
extern crate gfx_device_gl;
extern crate rand;
extern crate nalgebra as na;
extern crate reactive;

use std::{cmp, f64};
use std::sync::{Arc, Mutex, Condvar};
use piston_window::*;
use rand::{Rng, weak_rng};
use rand::distributions::{Normal, IndependentSample};

use reactive::process::{Process, ProcessMut};
use reactive::process::{value_proc, join_all};
use reactive::process::execute_process_parallel_with_main;
use reactive::process::LoopStatus::{Continue, Exit};
use reactive::signal::ValuedSignal;
use reactive::signal::parallel::{SpmcSignalPl, MpmcSignalPl};

const WIDTH: usize = 80;
const HEIGHT: usize = 60;
const NUM_AGENTS: usize = 80;
const MAX_SUGAR: usize = 6;
const FETILITY_AGE: (usize, usize) = (15, 60);

fn main() {

    let sugar_dist = gen_sugar_capcity(
        vec![(7, 10), (23, 46), (52, 33)], MAX_SUGAR, 
        vec![3., 7., 13., 20., 30., 50.], WIDTH, HEIGHT);

    let sim = Simulation {
        sugar_capacity: sugar_dist.clone(),
        sugar_grow_back_rate: 1,
        sugar_grow_back_interval: 1,
    };

    let agent_init = AgentInit {
        init_sugar_range: (40, 61),
        sugar_metabolism_range: (3, 7),
        max_age_range: (21, 101),
        vision_range: (2, 6),
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
        occupied: na::DMatrix::from_element(WIDTH, HEIGHT, None),
        agents: Vec::new(),
        tick: 0,
    };

    let gs = Arc::new(Mutex::new(gs));

    let broadcast = SpmcSignalPl::new();
    let broadcast_rev = broadcast.clone();

    let occupied = SpmcSignalPl::new();
    let occupied_rev = occupied.clone();

    let tick = SpmcSignalPl::new();
    let tick_rev = tick.clone();
    
    let agents_signal = MpmcSignalPl::default();
    let agents_signal_rev = agents_signal.clone();
    let agents_signal_cl = agents_signal.clone();

    for &agent_pos in init_aps.iter() {
        let pos = (agent_pos.0 as usize, agent_pos.1 as usize);
        let mut gs_lock = gs.lock().unwrap();
        let ag = agent_init.gen_agent(pos, &gs_lock.sugar_dist);
        gs_lock.agents.push((ag, SpmcSignalPl::new()));
    }
        
    let agent_alive =
        move |(ag, confirm): AgentComplete, sugar_dist: Arc<na::DMatrix<usize>>|
    {
        let agents_signal = agents_signal_cl.clone();
        let confirm_cl = confirm.clone();
        let seek_next_pos = move |occupied: Arc<na::DMatrix<Option<Agent>>>| {
            let confirm_cl = confirm_cl.clone();
            let (old_pos, new_ag) = ag.move_to(&sugar_dist, &occupied);
            agents_signal
            .emit((old_pos, Some((new_ag.clone(), confirm_cl))))
            .then(value_proc(ag))
        };
        occupied_rev
        .await()
        .and_then(seek_next_pos)
        // Must wait next instant to get a confirmation.
        .pause()
        .then(confirm.await())
        .if_else(value_proc(Exit(true)), value_proc(Continue))
        .while_proc()
    };

    let agent_update_proc = 
        move |(ag, confirm): AgentComplete, sugar_dist: Arc<na::DMatrix<usize>>|
    {
        let (alive, ag) = ag.metabolise();
        value_proc(alive).if_else(agent_alive((ag, confirm), sugar_dist), value_proc(false))
    };
        
    let definitely_move = 
        move |sugar_dist: Arc<na::DMatrix<usize>>, agents: Arc<Vec<AgentComplete>>|
    {
        let agent_procs = 
            agents.iter().map(|ag| agent_update_proc(ag.clone(), sugar_dist.clone()));
        let agents_left = |agents: Vec<bool>| {
            if agents.iter().all(|x| !x) {
                Exit(())
            } else {
                Continue
            }
        };
        join_all(agent_procs).map(agents_left)
    };

    let tick_rev_cl = tick_rev.clone();
    let await_tick = move |(sd, agents): (Arc<na::DMatrix<usize>>, Arc<Vec<AgentComplete>>)| {
        tick_rev_cl
        .await()
        .if_else(definitely_move(sd, agents), value_proc(Exit(())))
    };
 
    let agents_proc =
        broadcast_rev
        .await()
        .and_then(await_tick)
        .while_proc();

    // To ensure that the program doesn't hangs when all the agents die.
    let tick_rev_cl = tick_rev.clone();
    let emit_on_agents_signal = 
        agents_signal
        .emit(((0, 0), None))
        .pause()
        .then(tick_rev_cl.await())
        .if_else(value_proc(Continue), value_proc(Exit(())))
        .while_proc();

    let gs_cl = gs.clone();
    let broadcast_info = move |()| {
        sim.update_global_state(&mut *gs_cl.lock().unwrap());
        let gs_lock = gs_cl.lock().unwrap();
        let sugar_dist = gs_lock.sugar_dist.clone();
        let agents = gs_lock.agents.clone();
        broadcast.emit((Arc::new(sugar_dist), Arc::new(agents)))
    };

    let gs_cl = gs.clone();
    let reset_agents = move |()| gs_cl.lock().unwrap().instant_reset();

    let gs_cl = gs.clone();
    let collision_detect = move |aps_update| {
        let (confirmed, rejected) = gs_cl.lock().unwrap().update_agents(aps_update);
        let confirm_signals = confirmed.iter().map(|s| s.emit(true));
        let reject_signals = rejected.iter().map(|s| s.emit(false));
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
        .then(agents_signal_rev.await())
        .and_then(collision_detect)
        .while_proc();

    let update_patches = 
        value_proc(())
        .and_then(broadcast_info)
        .then(tick_rev.await())
        .if_else(value_proc(()).map(reset_agents).then(update_aps), value_proc(Exit(())))
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
        broadcast_rev
        .await()
        .and_then(inform_simulation_updated)
        .pause()
        .while_proc();
    
    let mut window: PistonWindow = 
        WindowSettings::new("Sugarscape", ((WIDTH as u32 + 27) * 10, HEIGHT as u32 * 10))
        .exit_on_esc(true)
        .build()
        .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));
    window.set_max_fps(8);

    let assets = find_folder::Search::ParentsThenKids(3, 3).for_folder("assets").unwrap();
    let ref font = assets.join("FiraSans-Regular.ttf");
    let factory = window.factory.clone();
    let glyphs = Glyphs::new(font, factory, TextureSettings::new()).unwrap();
    
    let mut display_config = DisplayConfig {
        max_sugar: MAX_SUGAR,
        sugar_color: [1.0, 0.7, 0.1, 1.0],
        agent_male_color: [0.2, 0.1, 1.0, 0.9],
        agent_female_color: [1.0, 0.2, 0.0, 0.9],
        cache: glyphs,
    };

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
                update_window(c, g, &gs.lock().unwrap(), &mut display_config, &grid);
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

    execute_process_parallel_with_main(
        update_window_clos,
        update_patches
        .join(inform_updated)
        .join(agents_proc)
        .join(emit_on_agents_signal), 2);
}

#[derive(PartialEq)]
enum WindowUpdated {
    Done,
    NotYet,
    End,
}

type Pos = (usize, usize);
type AgentComplete = (Agent, SpmcSignalPl<bool>);

struct GlobalState {
    sugar_dist: na::DMatrix<usize>,
    occupied: na::DMatrix<Option<Agent>>,
    agents: Vec<AgentComplete>,
    tick: usize,
}

impl GlobalState {
    fn update_agents(
        &mut self, agents_update: Vec<(Pos, Option<AgentComplete>)>
        ) -> (Vec<SpmcSignalPl<bool>>, Vec<SpmcSignalPl<bool>>)
    {
        let mut confirmed = Vec::new();
        let mut rejected = Vec::new();
        for (old_pos, agent_opt) in agents_update.into_iter() {
            if let Some((agent, confirm)) = agent_opt {
                if self.occupied[agent.pos].is_none() || agent.pos == old_pos {
                    self.occupied[agent.pos] = Some(agent.clone());
                    self.occupied[old_pos] = None;
                    self.agents.push((agent, confirm.clone()));
                    confirmed.push(confirm);
                } else {
                    rejected.push(confirm);
                }
            }
        }
        (confirmed, rejected)
    }

    fn instant_reset(&mut self) {
        self.agents = Vec::new();
    }
}


struct Simulation {
    sugar_capacity: na::DMatrix<usize>,
    sugar_grow_back_rate: usize,
    sugar_grow_back_interval: usize,
}

impl Simulation {
    fn update_global_state(&self, gs: &mut GlobalState) {
        for &(agent, _) in gs.agents.iter() {
            gs.sugar_dist[agent.pos] = 0;
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

#[derive(Copy, Clone)]
enum Growth {
    Child,
    Fertile,
    Aged,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Gender {
    Male,
    Female,
}

type AgentStatus = (Gender, Growth);

impl Gender {
    fn rand_gender() -> Gender {
        if weak_rng().gen_weighted_bool(2) {
            Gender::Male
        } else {
            Gender::Female
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct Agent {
    pos: Pos,
    init_sugar: usize,
    sugar: usize,
    sugar_metabolism: usize,
    vision: usize,
    age: usize,
    max_age: usize,
    gender: Gender,
}

fn l1_distance(pos1: (usize, usize), pos2: (usize, usize)) -> usize {
    let x_dist = cmp::max(pos1.0, pos2.0) - cmp::min(pos1.0, pos2.0);
    let y_dist = cmp::max(pos1.1, pos2.1) - cmp::min(pos1.1, pos2.1);
    x_dist + y_dist
}

/* Defines the agents. */

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
    fn metabolise(self) -> (bool, Agent) {
        let new_sugar = self.sugar.saturating_sub(self.sugar_metabolism);
        let new_age = self.age + 1;
        let alive = new_sugar > 0 && new_age <= self.max_age;
        let new_agent = Agent { sugar: new_sugar, age: new_age, ..self };
        (alive, new_agent)
    }

    fn status(&self) -> AgentStatus {
        if self.age < FETILITY_AGE.0 {
            (self.gender, Growth::Child)
        } else if self.age >= FETILITY_AGE.1 {
            (self.gender, Growth::Aged)
        } else {
            (self.gender, Growth::Fertile)
        }
    }

    fn move_to(self,
               sugar_dist: &na::DMatrix<usize>,
               occupied: &na::DMatrix<Option<Agent>>) -> (Pos, Agent)
    {
        let self_pos = self.pos.clone();
        let mut current_most = sugar_dist[self_pos];
        let mut min_d = 0;
        let mut possible_next_pos = vec![self_pos];
        for &pos in self.can_see(sugar_dist.nrows()-1, sugar_dist.ncols()-1).iter() {
            if occupied[pos].is_none() {
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
        let new_pos = weak_rng().choose(&possible_next_pos).unwrap().clone();
        let new_sugar = self.sugar + sugar_dist[new_pos];
        let new_agent = Agent { pos: new_pos, sugar: new_sugar, .. self };
        (self.pos, new_agent)
    }
}

struct AgentInit {
    init_sugar_range: (usize, usize),
    sugar_metabolism_range: (usize, usize),
    max_age_range: (usize, usize),
    vision_range: (usize, usize),
}

fn gen_normal(range: (usize, usize)) -> usize {
    let (low, high) = (range.0 as f64, range.1 as f64);
    let mean = (low + high) / 2.0;
    let std = (high - low) / 4.0;
    let normal = Normal::new(mean, std);
    let mut rng = weak_rng();
    let mut v = normal.ind_sample(&mut rng);
    while v >= high || v < low {
        v = normal.ind_sample(&mut rng);
    }
    v.floor() as usize
}

impl AgentInit {
    fn gen_agent(&self, pos: (usize, usize), sugar_dist: &na::DMatrix<usize>) -> Agent {
        let init_sugar = gen_normal(self.init_sugar_range);
        Agent {
            pos: pos,
            init_sugar: init_sugar,
            sugar: init_sugar + sugar_dist[pos],
            sugar_metabolism: gen_normal(self.sugar_metabolism_range),
            vision: gen_normal(self.vision_range),
            age: 0,
            max_age: gen_normal(self.max_age_range),
            gender: Gender::rand_gender(),
        }
    }
}

/* Shows the window */

struct DisplayConfig<C: character::CharacterCache> {
    max_sugar: usize,
    sugar_color: types::Color,
    agent_male_color: types::Color,
    agent_female_color: types::Color,
    cache: C,
}

impl<C> DisplayConfig<C> where C: character::CharacterCache {
    fn compute_cell_color(&self, sugar_amount: usize) -> types::Color {
        assert!(sugar_amount <= self.max_sugar,
                "The maximum sugar capacity is depassed!");
        let fraction = (sugar_amount as f32)/(self.max_sugar as f32);
        self.sugar_color.mul_rgba(1.0, 1.0, 1.0, fraction)
    }
}

fn update_window<C>(
    c: Context,
    g: &mut G2d,
    gs: &GlobalState,
    config: &mut DisplayConfig<C>,
    grid: &grid::Grid)
where 
    C: character::CharacterCache<Texture=Texture<gfx_device_gl::Resources>>,
    C::Error: std::fmt::Debug,
{
    clear(color::WHITE, g);
    for cell in grid.cells() {
        let color = config.compute_cell_color(
            gs.sugar_dist[(cell.0 as usize, cell.1 as usize)]);
        let coord = grid.cell_position(cell);
        rectangle(color, [coord[0], coord[1], grid.units, grid.units], c.transform, g);
    }
    for &(agent, _) in gs.agents.iter() {
        let coord = grid.cell_position((agent.pos.0 as u32, agent.pos.1 as u32));
        let color = match agent.status().0 {
            Gender::Male => config.agent_male_color,
            Gender::Female => config.agent_female_color,
        };
        let color = match agent.status().1 {
            Growth::Child => color.mul_rgba(1.0, 1.0, 1.0, 0.7),
            Growth::Fertile => color,
            Growth::Aged => color.shade(0.3),
        };
        ellipse(color, [coord[0], coord[1], grid.units, grid.units], c.transform, g);
    }
    let transform = c.transform.trans(WIDTH as f64 * 10.0, 0.0);
    rectangle([0.0, 0.0, 0.0, 0.85], [0.0, 0.0, 270.0, HEIGHT as f64 * 10.0], transform, g);
    line([0.0, 0.0, 0.0, 1.0], 2.0, [0.0, 0.0, 0.0, HEIGHT as f64 * 10.0], transform, g);
    let transform = transform.trans(25.0, 50.0);
    text(color::WHITE, 28, "Number of Agents", &mut config.cache, transform, g).unwrap();
    let transform = transform.trans(0.0, 35.0);
    let num_of_agents = format!("{}", gs.agents.len());
    text(color::WHITE, 28, &num_of_agents, &mut config.cache, transform, g).unwrap();
    let transform = transform.trans(0.0, 40.0);
    text(color::WHITE, 28, "Board Sugar", &mut config.cache, transform, g).unwrap();
    let transform = transform.trans(0.0, 35.0);
    let total_sugar_board = format!("{}", gs.sugar_dist.iter().fold(0, |sum, &i| sum+i));
    text(color::WHITE, 28, &total_sugar_board, &mut config.cache, transform, g).unwrap();
    let transform = transform.trans(0.0, 40.0);
    text(color::WHITE, 28, "Agent Sugar", &mut config.cache, transform, g).unwrap();
    let transform = transform.trans(0.0, 35.0);
    let total_sugar_agents = format!("{}", gs.agents.iter().fold(0, |sum, &(a, _)| sum+a.sugar));
    text(color::WHITE, 28, &total_sugar_agents, &mut config.cache, transform, g).unwrap();
}

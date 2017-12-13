extern crate piston_window;
extern crate nalgebra as na;
extern crate reactive;

use std::{cmp, thread, time};
use std::sync::{Arc, Mutex, Condvar};
use piston_window::*;

use reactive::process::{Process, ProcessMut};
use reactive::process::{value_proc, execute_process_parallel_with_main};
use reactive::process::LoopStatus::{Continue, Exit};
use reactive::signal::ValuedSignal;
use reactive::signal::parallel::{SpmcSignalPl, MpmcSignalPl};

fn main() {

    let display_config = DisplayConfig {
        max_sugar: 6,
        sugar_color: [1.0, 0.7, 0.1, 1.0],
        agent_color: [0.2, 0.1, 1.0, 1.0],
    };
    let sim = Simulation {
        sugar_capacity: na::DMatrix::from_element(60, 60, 6),
        sugar_grow_back_rate: 1,
        sugar_grow_back_interval: 1,
    };
    let grid = grid::Grid {
        cols: 60,
        rows: 60,
        units: 10.0,
    };
    let gs = GlobalState {
        sugar: na::DMatrix::from_element(60, 60, 1),
        tick: 0,
    };
    let gs = Arc::new(Mutex::new(gs));
    let gs_cl = gs.clone();

    let gs_signal = SpmcSignalPl::new();
    let gs_signal_rev = gs_signal.clone();
    let tick = SpmcSignalPl::new();
    let tick_rev = tick.clone();
    //let aps_signal = MpmcSignalSt::default();
    
    let broadcast_sugar_dist = move |()| {
        let sugar_dist = gs_cl.lock().unwrap().sugar.clone();
        gs_signal.emit(Arc::new(sugar_dist))
    };
    let update_patches_clos = move |to_continue| {
        sim.update_game_state(&mut *gs.lock().unwrap(), &vec![]);
        if to_continue { Continue } else { Exit(()) }
    };

    let update_patches = 
        value_proc(())
        .and_then(broadcast_sugar_dist)
        .then(tick_rev.await())
        .pause()
        .map(update_patches_clos)
        .while_proc();
    
    let mut window: PistonWindow = 
        WindowSettings::new("Hello", (600, 600))
        .exit_on_esc(true)
        .build()
        .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));
    window.set_max_fps(4);

    let sugar_dist_main = Arc::new(Mutex::new(Arc::new(na::DMatrix::from_element(60, 60, 1))));
    let sugar_dist_main_cl = sugar_dist_main.clone();
    let simulation_updated = Arc::new((Mutex::new(false), Condvar::new()));
    let simulation_updated_cl = simulation_updated.clone();
    let window_updated = Arc::new((Mutex::new(WindowUpdated::NotYet), Condvar::new()));
    let window_updated_cl = window_updated.clone();

    let inform_simulation_updated = move |sugar_dist: Arc<na::DMatrix<usize>>| {
        *sugar_dist_main_cl.lock().unwrap() = sugar_dist.clone();
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
            WindowUpdated::NotYet => panic!("This shouldn't happen!"),
        }
    };

    let inform_updated = 
        gs_signal_rev
        .await()
        .and_then(inform_simulation_updated)
        .pause()
        .while_proc();

    let update_window_clos = || {
        let aps = vec![(0, 3), (10, 50)];
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
                update_window(c, g, &sugar_dist_main.lock().unwrap(),
                              &aps, &display_config, &grid);
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
        update_window_clos, update_patches.join(inform_updated), 2);
}

#[derive(PartialEq)]
enum WindowUpdated {
    Done,
    NotYet,
    End,
}

struct Simulation {
    sugar_capacity: na::DMatrix<usize>,
    sugar_grow_back_rate: usize,
    sugar_grow_back_interval: usize,
}

impl Simulation {
    fn update_game_state(&self, gs: &mut GlobalState, aps: &Vec<(usize, usize)>) {
        for &agent_pos in aps.iter() {
            gs.sugar[agent_pos] = 0;
        }
        gs.tick = (gs.tick+1) % self.sugar_grow_back_interval;
        if gs.tick == 0 {
            gs.sugar = gs.sugar.zip_map(
                &self.sugar_capacity,
                |sugar, sugar_capacity|
                    cmp::min(sugar+self.sugar_grow_back_rate, sugar_capacity))
        }
    }
}

struct GlobalState {
    sugar: na::DMatrix<usize>,
    tick: usize,
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
    sugar: &na::DMatrix<usize>,
    agent_poss: &Vec<(usize, usize)>,
    config: &DisplayConfig,
    grid: &grid::Grid)
{
    for cell in grid.cells() {
        let color = config.compute_cell_color(
            sugar[(cell.0 as usize, cell.1 as usize)]);
        let coord = grid.cell_position(cell);
        rectangle(color, [coord[0], coord[1], grid.units, grid.units], c.transform, g);
    }
    for &agent_pos in agent_poss.iter() {
        let coord = grid.cell_position((agent_pos.0 as u32, agent_pos.1 as u32));
        ellipse(config.agent_color, 
                [coord[0], coord[1], grid.units, grid.units], c.transform, g);
    }
}

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Barrier, Condvar};
use crossbeam::sync::chase_lev;
use rand::weak_rng;
use ordermap::OrderSet;

use parallel::continuation;
use parallel::ParallelRuntime;
use parallel::runtime::RuntimeStatus;

pub struct ParallelRuntimeCollection {
    runtimes: Vec<ParallelRuntime>,
}

impl ParallelRuntimeCollection {
    pub fn new(num_runtimes: usize) -> Self {
        let mut runtimes = Vec::new();
        let mut workers = VecDeque::new();
        let mut stealers = Vec::new();
        for _ in 0..num_runtimes {
            let (worker, stealer) = chase_lev::deque();
            workers.push_back(worker);
            stealers.push(stealer);
        }
        let barrier = Arc::new(Barrier::new(num_runtimes));
        let working_pool = Arc::new(
            Mutex::new((0..num_runtimes).collect::<OrderSet<_>>()));
        let whether_to_continue = Arc::new(
            (Mutex::new(RuntimeStatus::WorkRemained), Condvar::new()));
        for i in 0..num_runtimes {
            let worker = workers.pop_front().unwrap();
            let stealers: Vec<_> = stealers.iter().map(|stealer| stealer.clone()).collect();
            runtimes.push(ParallelRuntime {
                id: i,
                num_threads_total: num_runtimes,
                worker,
                stealers,
                barrier: barrier.clone(),
                rng: weak_rng(),
                working_pool: working_pool.clone(),
                whether_to_continue: whether_to_continue.clone(),
                next_instant_works: Vec::new(),
            })
        }
        ParallelRuntimeCollection { runtimes }
    }
}

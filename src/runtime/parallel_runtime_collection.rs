use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Barrier, Condvar};
use std::sync::atomic::AtomicUsize;

#[cfg(feature = "abort_if_panic")]
use std::panic;
#[cfg(feature = "abort_if_panic")]
use std::process;

use crossbeam;
use crossbeam::sync::chase_lev;
use rand::{weak_rng, Rng};
use ordermap::OrderSet;

use runtime::Runtime;
use runtime::parallel_runtime::{ParallelRuntime, RuntimeStatus};
use continuation::ContinuationPl;

pub struct ParallelRuntimeCollection {
    runtimes: Vec<ParallelRuntime>,
}

impl ParallelRuntimeCollection {
    pub fn new(num_runtimes: usize) -> Self {
        if num_runtimes == 0 {
            panic!("There should be at least one runtime!");
        }
        let mut runtimes = Vec::new();
        let mut workers = VecDeque::new();
        let mut stealers = Vec::new();
        for _ in 0..num_runtimes {
            let (worker, stealer) = chase_lev::deque();
            workers.push_back(worker);
            stealers.push(stealer);
        }
        let barrier = Arc::new(Barrier::new(num_runtimes));
        let await_counter = Arc::new(AtomicUsize::new(0));
        let working_pool = Arc::new(
            Mutex::new((0..num_runtimes).collect::<OrderSet<_>>()));
        let eoi_working_pool = Arc::new(
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
                end_of_instant_works: Vec::new(),
                eoi_working_pool: eoi_working_pool.clone(),
                emitted_signals: Vec::new(),
                await_counter: await_counter.clone(),
                test_presence_signals: Vec::new(),
                #[cfg(feature = "debug")]
                instant: 0,
            })
        }
        ParallelRuntimeCollection { runtimes }
    }

    /// Execute in parallel all the runtimes contained in the collection, with
    /// one thread for each runtime.  
    /// When the library is compiled with the feature `abort_if_panic`, the process
    /// that is executing the program is aborted whenever a child thread panics.
    /// Maybe there's another way to smartly deal with panics coming from child threads
    /// but I am not particularly working on it.
    pub fn execute<F>(&mut self, main_thread_function: F) where F: FnOnce() {
        #[cfg(feature = "abort_if_panic")] {
            // The function `take_handler` returns the default handler in case
            // when a custom one is not set.
            let orig_handler = panic::take_hook();
            panic::set_hook(Box::new(move |panic_info| {
                // Invokes the default handler and exit the process.
                orig_handler(panic_info);
                process::exit(1);
            }));
        }
        crossbeam::scope(|scope| {
            for runtime in self.runtimes.iter_mut() {
                scope.spawn(move || runtime.execute());
            }
            main_thread_function();
        });
    }

    pub fn register_work(&mut self, c: Box<ContinuationPl<()>>) {
        let runtime = weak_rng().choose_mut(&mut self.runtimes).unwrap();
        runtime.on_current_instant(c);
    }
}

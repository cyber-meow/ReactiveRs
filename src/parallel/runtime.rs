use std::sync::{Arc, Mutex, Barrier, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use crossbeam::sync::chase_lev;
use rand::{Rng, XorShiftRng};
use ordermap::OrderSet;

use parallel::Continuation;
use parallel::signal::signal_runtime::SignalRuntimeRefBase;

pub struct Runtime {
    pub(crate) id: usize,
    pub(crate) num_threads_total: usize,
    pub(crate) worker: chase_lev::Worker<Box<Continuation<()>>>,
    pub(crate) stealers: Vec<chase_lev::Stealer<Box<Continuation<()>>>>,
    pub(crate) barrier: Arc<Barrier>,
    pub(crate) rng: XorShiftRng,
    pub(crate) working_pool: Arc<Mutex<OrderSet<usize>>>,
    pub(crate) whether_to_continue: Arc<(Mutex<RuntimeStatus>, Condvar)>,
    pub(crate) next_instant_works: Vec<Box<Continuation<()>>>,
    pub(crate) end_of_instant_works: Vec<Box<Continuation<()>>>,
    pub(crate) eoi_working_pool: Arc<Mutex<OrderSet<usize>>>,
    pub(crate) emitted_signals: Vec<Box<SignalRuntimeRefBase>>,
    pub(crate) await_counter: Arc<AtomicUsize>,
    pub(crate) test_presence_signals: Vec<Box<SignalRuntimeRefBase>>,
    #[cfg(feature = "debug")]
    pub(crate) instant: usize,
}

pub(crate) enum RuntimeStatus {
    Undetermined(usize),
    WorkRemained,
    Finished,
}

impl Runtime {
    /// Executes instants until all work is completed.
    pub fn execute(&mut self) {
        while self.instant() {};
    }

    /// Executes a single instant to completion. Indicates if more work remains to be done.
    pub fn instant(&mut self) -> bool {
        #[cfg(feature = "debug")] {
            println!("Thread {}: instant {}.", self.id, self.instant);
            self.instant += 1;
        }
        self.consume_current_works(false);
        self.end_of_instant()
    }

    /// When there are some works in `worker`, finishes them.
    fn consume_current_works(&mut self, is_eoi: bool) {
        loop {
            if let Some(work) = self.worker.try_pop() {
                if cfg!(feature = "debug") {
                    println!("Thread {}: work.", self.id);
                }
                work.call_box(self, ());
            } else if let Some(work) = self.try_steal(is_eoi) {
                if cfg!(feature = "debug") {
                    println!("Thread {}: work.", self.id);
                }
                work.call_box(self, ());
            } else {
                break;
            }
        }
        if self.id == 0 && is_eoi {
            let (ref lock, _) = *self.whether_to_continue;
            let mut runtime_status = lock.lock().unwrap();
            *runtime_status = RuntimeStatus::Undetermined(0);
        }
        if cfg!(feature = "debug") {
            println!("Thread {}: sleep.", self.id);
        }
        self.barrier.wait();
    }

    /// Tries to steal work from other workers.
    /// Returns `None` only when there is no longer anyone who is working.
    fn try_steal(&mut self, is_eoi: bool) -> Option<Box<Continuation<()>>> {
        let wp = if is_eoi { &self.eoi_working_pool } else { &self.working_pool };
        {
            let mut working_pool = wp.lock().unwrap();
            assert!(working_pool.remove(&self.id));
            if cfg!(feature = "debug") {
                println!("Thread {}: try to steal.", self.id);
            }
        }
        loop {
            let working_pool = wp.lock().unwrap();
            if working_pool.is_empty() {
                return None;
            }
            let index = self.rng.gen_range(0, working_pool.len());
            let to_steal = *working_pool.get_index(index).unwrap();
            // Explicit unlock for efficiency.
            // Problem: after succeding in stealing some work, `working_pool` can be
            // empty for a while before the stealer adds itself to the pool (if the
            // original owner of the task doesn't have any more work to do and removes
            // itself from the pool before this). Some threads can then believe that
            // the current instant is terminated and go to sleep.
            drop(working_pool);
            match self.stealers[to_steal].steal() {
                chase_lev::Steal::Data(work) => {
                    let mut working_pool = wp.lock().unwrap();
                    assert!(working_pool.insert(self.id));
                    if cfg!(feature = "debug") {
                        println!("Thread {}: steal success.", self.id);
                    }
                    return Some(work);
                },
                _ => {
                    thread::yield_now();
                    continue;
                },
            };
        }
    }

    /// Terminates/Starts the instant properly after the first synchronization.
    /// One important point is to know if there is still work to be done somewhere
    /// (knowing that the work can come from another runtime).
    fn end_of_instant(&mut self) -> bool {
        while let Some(work) = self.end_of_instant_works.pop() {
            self.on_current_instant(work);
        }
        self.consume_current_works(true);
        while let Some(s) = self.test_presence_signals.pop() {
            s.execute_present_works_box(self);
        }
        while let Some(s) = self.emitted_signals.pop() {
            s.reset_box();
        }
        self.barrier.wait();
        self.deal_with_next_instant_works()
    }

    /// Moves works from `next_instant_works` to `worker` if there is any
    /// and decides if the program should be terminate (`true` means shouldn't).
    fn deal_with_next_instant_works(&mut self) -> bool {
        if self.next_instant_works.is_empty() {
            if self.await_counter.load(Ordering::SeqCst) != 0 {
                return true;
            }
            let (ref lock, ref cvar) = *self.whether_to_continue;
            {
                let mut runtime_status = lock.lock().unwrap();
                match *runtime_status {
                    RuntimeStatus::Undetermined(k)
                        if k == self.num_threads_total - 1 =>
                    {
                        *runtime_status = RuntimeStatus::Finished;
                        cvar.notify_all();
                        return false;
                    },
                    RuntimeStatus::Undetermined(k) => {
                        *runtime_status = RuntimeStatus::Undetermined(k+1);
                    },
                    RuntimeStatus::WorkRemained => return true,
                    RuntimeStatus::Finished => assert!(false),
                };
            }
            let mut runtime_status = lock.lock().unwrap();
            loop {
                match *runtime_status {
                    RuntimeStatus::Undetermined(_) => {
                        runtime_status = cvar.wait(runtime_status).unwrap();
                    },
                    RuntimeStatus::WorkRemained => return true,
                    RuntimeStatus::Finished => return false,
                };
            }
        } else {
            {
                let (ref lock, ref cvar) = *self.whether_to_continue;
                let mut runtime_status = lock.lock().unwrap();
                match *runtime_status {
                    RuntimeStatus::Undetermined(_) => {
                        *runtime_status = RuntimeStatus::WorkRemained;
                        let mut working_pool = self.working_pool.lock().unwrap();
                        debug_assert!(working_pool.is_empty());
                        *working_pool = (0..self.num_threads_total).collect();
                        let mut eoi_working_pool = self.eoi_working_pool.lock().unwrap();
                        debug_assert!(eoi_working_pool.is_empty());
                        *eoi_working_pool = (0..self.num_threads_total).collect();
                        cvar.notify_all();
                    },
                    RuntimeStatus::WorkRemained => (),
                    RuntimeStatus::Finished => assert!(false),
                };
            }
            while let Some(work) = self.next_instant_works.pop() {
                self.on_current_instant(work);
            }
            return true;
        }
    }
    
    /// Registers a continuation to execute on the current instant.
    pub(crate) fn on_current_instant(&mut self, c: Box<Continuation<()>>) {
        self.worker.push(c);
    }

    /// Registers a continuation to execute at the next instant.
    pub(crate) fn on_next_instant(&mut self, c: Box<Continuation<()>>) {
        self.next_instant_works.push(c);
    }
    
    /// Registers a continuation to execute at the end of the instant. Runtime calls for `c`
    /// behave as if they where executed during the next instant.
    pub(crate) fn on_end_of_instant(&mut self, c: Box<Continuation<()>>) {
        self.end_of_instant_works.push(c);
    }
    
    /// Increases the await counter by 1 when some process await a signal to continue.
    pub(crate) fn incr_await_counter(&mut self) {
        self.await_counter.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrease the await counter by 1 when some signal is emitted and
    /// the corresponding process is thus executed.
    pub(crate) fn decr_await_counter(&mut self) {
        self.await_counter.fetch_sub(1, Ordering::SeqCst);
    }

    /// Registers a emitted signal for the current instant.
    pub(crate) fn emit_signal(&mut self, s: Box<SignalRuntimeRefBase>) {
        self.emitted_signals.push(s);
    }
    
    /// Registers a signal for which we need to test its presence on the current instant.
    pub(crate) fn add_test_signal(&mut self, s: Box<SignalRuntimeRefBase>) {
        self.test_presence_signals.push(s);
    }
}

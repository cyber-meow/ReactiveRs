use std::sync::{Arc, Mutex, Barrier, Condvar};
use crossbeam::sync::chase_lev;
use rand::{Rng, XorShiftRng};
use ordermap::OrderSet;

use parallel::Continuation;

pub struct ParallelRuntime {
    pub(crate) id: usize,
    pub(crate) num_threads_total: usize,
    pub(crate) worker: chase_lev::Worker<Box<Continuation<()>>>,
    pub(crate) stealers: Vec<chase_lev::Stealer<Box<Continuation<()>>>>,
    pub(crate) barrier: Arc<Barrier>,
    pub(crate) rng: XorShiftRng,
    pub(crate) working_pool: Arc<Mutex<OrderSet<usize>>>,
    pub(crate) whether_to_continue: Arc<(Mutex<RuntimeStatus>, Condvar)>,
    pub(crate) next_instant_works: Vec<Box<Continuation<()>>>,
}

pub(crate) enum RuntimeStatus {
    Undetermined(usize),
    WorkRemained,
    Finished,
}

impl ParallelRuntime {
    /// Executes instants until all work is completed.
    pub fn execute(&mut self) {
        while self.instant() {};
    }

    /// Executes a single instant to completion. Indicates if more work remains to be done.
    pub fn instant(&mut self) -> bool {
        loop {
            if let Some(work) = self.worker.try_pop() {
                work.call_box(self, ());
            } else if let Some(work) = self.try_steal() {
                work.call_box(self, ());
            } else {
                break;
            }
        }
        if self.id == 0 {
            let (ref lock, _) = *self.whether_to_continue;
            let mut runtime_status = lock.lock().unwrap();
            *runtime_status = RuntimeStatus::Undetermined(0);
        }
        self.barrier.wait();
        self.end_of_instant()
    }

    /// Tries to steal some work from other workers.
    /// Returns `None` only when there is no longer anyone who is working.
    fn try_steal(&mut self) -> Option<Box<Continuation<()>>> {
        {
            let mut working_pool = self.working_pool.lock().unwrap();
            assert!(working_pool.remove(&self.id));
        }
        loop {
            let working_pool = self.working_pool.lock().unwrap();
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
                    let mut working_pool = self.working_pool.lock().unwrap();
                    assert!(working_pool.insert(self.id));
                    return Some(work);
                },
                _ => continue,
            };
        }
    }

    /// Terminates/Starts the instant properly after the synchronization.
    /// One important point is to know if there is still work to be done somewhere
    /// (knowing that the work can come from another runtime).
    fn end_of_instant(&mut self) -> bool {
        if self.next_instant_works.is_empty() {
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
                        assert!(working_pool.is_empty());
                        *working_pool = (0..self.num_threads_total).collect();
                        cvar.notify_all();
                    },
                    RuntimeStatus::WorkRemained => (),
                    RuntimeStatus::Finished => assert!(false),
                };
            }
            while let Some(work) = self.next_instant_works.pop() {
                self.worker.push(work);
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
}

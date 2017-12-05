/*mod process_mut;
pub use self::process_mut::{ProcessMut, ProcessMutSt, ProcessMutPl};
*/
/*mod value;
mod pause;
pub use self::value::{value, Value};
pub use self::pause::Pause;*/

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use runtime::{Runtime, SingleThreadRuntime};
use runtime::{ParallelRuntime, ParallelRuntimeCollection};
use continuation::{Continuation, ContinuationPl};

/// A reactive process.
pub trait Process<R, C>: 'static where R: Runtime, C: Continuation<R, Self::Value> {
    /// The value created by the process.
    type Value;

    /// Executes the reactive process in the runtime, calls `next` with the resulting value.
    fn call(self, runtime: &mut R, next: C);
}

/// Reactive process to be executed in a single thread.
pub trait ProcessSt<C>: Process<SingleThreadRuntime, C>
    where C: Continuation<SingleThreadRuntime, Self::Value> {}

impl<P, C> ProcessSt<C> for P
    where P: Process<SingleThreadRuntime, C>,
          C: Continuation<SingleThreadRuntime, Self::Value> {}

/// Reactive process that can be safely passed and shared between threads.
pub trait ProcessPl<C>:
        Process<ParallelRuntime, C, Value=<Self as ConstraintOnValue>::T> 
        + ConstraintOnValue + Send + Sync
    where C: ContinuationPl<Self::Value> {}

impl<P, C> ProcessPl<C> for P where
    P: Process<ParallelRuntime, C, Value=<Self as ConstraintOnValue>::T> 
       + ConstraintOnValue + Send + Sync,
    C: ContinuationPl<Self::Value> {}

/// This is a workaround to have constraints on associated types.
pub trait ConstraintOnValue {
    type T: Send + Sync;
}

/*
/// Execute a process in a newly created runtime and return its value.
pub fn execute_process<P>(p: P) -> P::Value where P: ProcessSt {
    let mut runtime = SingleThreadRuntime::new();
    let res: Rc<RefCell<Option<P::Value>>> = Rc::new(RefCell::new(None));
    let res2 = res.clone();
    let c = move |_: &mut SingleThreadRuntime, v| *res2.borrow_mut() = Some(v);
    runtime.on_current_instant(Box::new(|r: &mut SingleThreadRuntime, _| p.call(r, c)));
    runtime.execute();
    let mut res = res.borrow_mut();
    res.take().unwrap()
}

/// Execute a process in a newly created runtime and return its value.
pub fn execute_process_parallel<P>(p: P, num_runtimes: usize) -> P::Value where P: ProcessPl
{
    if num_runtimes == 0 {
        panic!("There should be at least one runtime!");
    }
    let mut runtime_col = ParallelRuntimeCollection::new(num_runtimes);
    let res: Arc<Mutex<Option<P::Value>>> = Arc::new(Mutex::new(None));
    let res2 = res.clone();
    let c = move |_: &mut ParallelRuntime, v| *res2.lock().unwrap() = Some(v);
    runtime_col.register_work(Box::new(|r: &mut ParallelRuntime, _| p.call(r, c)));
    runtime_col.execute();
    let mut res = res.lock().unwrap();
    res.take().unwrap()
}*/

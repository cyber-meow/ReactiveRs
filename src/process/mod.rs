mod value;
pub use self::value::{value, Value};

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use runtime::{Runtime, SingleThreadRuntime};
use runtime::{ParallelRuntime, ParallelRuntimeCollection};
use continuation::Continuation;

/// A reactive process.
pub trait Process<R>: 'static where R: Runtime {
    /// The value created by the process.
    type Value;

    /// Executes the reactive process in the runtime, calls `next` with the resulting value.
    fn call<C>(self, runtime: &mut R, next: C) where C: Continuation<R, Self::Value>;
}

/// Reactive process to be executed in a single thread.
pub trait ProcessSt: Process<SingleThreadRuntime> {}

impl<P> ProcessSt for P where P: Process<SingleThreadRuntime> {}

/// Reactive process that can be safely passed and shared between threads.
pub trait ProcessPl:
    Process<ParallelRuntime, Value=<Self as ConstraintOnType>::T> 
    + ConstraintOnType + Send + Sync {}

impl<P> ProcessPl for P where P:
    Process<ParallelRuntime, Value=<Self as ConstraintOnType>::T> 
    + ConstraintOnType + Send + Sync {}

/// This is a workaround to have constraints on associated types.
pub trait ConstraintOnType {
    type T: Send + Sync;
}

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
}

mod process_mut;
pub use self::process_mut::{ProcessMutSt, ProcessMutPl};

mod value;
mod pause;
pub use self::value::{value, Value};
pub use self::pause::Pause;

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use runtime::{Runtime, SingleThreadRuntime};
use runtime::{ParallelRuntime, ParallelRuntimeCollection};
use continuation::{Continuation, ContinuationSt, ContinuationPl};

/// A abstract reactive process. A method `call` is in fact also necessary.
/// Please see `ProcessSt` and `ProcessPl` for more information.
pub trait Process: 'static {
    /// The value created by the process.
    type Value;  

    /// Suspends the execution of a process until next instant.
    fn pause(self) -> Pause<Self> where Self: Sized {
        Pause(self)
    }
}

// The codes for the two versions of the library are almost the same.
// However, I'm not able to figure out a way to design a common trait so that
// the method definitions and trait implementations can be put together.
//
// The problem is that the method `call` has a generic type parameter `C` and in the
// two cases it must implement the trait `ContinuationSt` or `ContinuationPl`.
// I didn't find a way to integrate this information into a same trait.
// For example, a trait cannot be parametrized by another trait, which can be helpful here.
//
/// A reactive process to be executed in a single thread.
pub trait ProcessSt: Process {
    /// Executes the reactive process in the runtime, calls `next` with the resulting value.
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>;
}

/// A reactive process that can be safely passed and shared between threads.
pub trait ProcessPl: 
        Process<Value = <Self as ConstraintOnValue>::T>
        + ConstraintOnValue + Send + Sync {
    /// Executes the reactive process in the runtime, calls `next` with the resulting value.
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>;
}

/// This is a workaround to have constraints on associated types.
pub trait ConstraintOnValue {
    type T: Send + Sync;
}

/// Execute a process in a newly created runtime and return its value.
pub fn execute_process<P>(p: P) -> P::Value where P: ProcessSt
{
    let mut runtime = SingleThreadRuntime::new();
    let res: Rc<RefCell<Option<P::Value>>> = Rc::new(RefCell::new(None));
    let res2 = res.clone();
    let c = move |_: &mut SingleThreadRuntime, v| *res2.borrow_mut() = Some(v);
    runtime.on_current_instant(Box::new(|r: &mut SingleThreadRuntime, _| p.call(r, c)));
    runtime.execute();
    let mut res = res.borrow_mut();
    res.take().unwrap()
}

/// Execute a process in newly created runtimes and return its value.
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

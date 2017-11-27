use std::sync::{Arc, Mutex};

mod process_mut;
pub use self::process_mut::ProcessMut;

mod value;
mod pause;
mod map;
pub use self::value::{value, Value};
pub use self::pause::Pause;
pub use self::map::Map;

use parallel::Continuation;
use parallel::{Runtime, RuntimeCollection};

/// A reactive process.
pub trait Process: Send + 'static {
    /// The value created by the process.
    type Value: Send;

    /// Executes the reactive process in the runtime, calls `next` with the resulting value.
    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value>;
    
    /// Suspends the execution of a process until next instant.
    fn pause(self) -> Pause<Self> where Self: Sized {
        Pause(self)
    }
    
    /// Applies a function to the value returned by the process before passing it to
    /// its continuation.
    fn map<F, V>(self, map: F) -> Map<Self, F>
        where Self: Sized, F: FnOnce(Self::Value) -> V + Send + 'static, V: Send + 'static
    {
        Map { process: self, map }
    }
}

/// Execute a process in a newly created runtime and return its value.
pub fn execute_process<P>(p: P, num_runtimes: usize) -> P::Value where P: Process {
    if num_runtimes == 0 {
        panic!("There should be at least one runtime!");
    }
    let mut runtime_col = RuntimeCollection::new(num_runtimes);
    let res: Arc<Mutex<Option<P::Value>>> = Arc::new(Mutex::new(None));
    let res2 = res.clone();
    let c = move |_: &mut Runtime, v| *res2.lock().unwrap() = Some(v);
    runtime_col.register_work(Box::new(|r: &mut Runtime, _| p.call(r, c)));
    runtime_col.execute();
    let mut res = res.lock().unwrap();
    res.take().unwrap()
}

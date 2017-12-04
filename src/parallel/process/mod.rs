use std::sync::{Arc, Mutex};

mod process_mut;
pub use self::process_mut::{ProcessMut, While, LoopStatus};

mod value;
mod pause;
mod map;
mod flatten;
mod and_then;
mod then;
mod if_else;
mod join;
pub use self::value::{value, Value};
pub use self::pause::Pause;
pub use self::map::Map;
pub use self::flatten::Flatten;
pub use self::and_then::AndThen;
pub use self::then::Then;
pub use self::if_else::IfElse;
pub use self::join::Join;

use parallel::Continuation;
use parallel::{Runtime, RuntimeCollection};

/// A reactive process.
pub trait Process: Send + Sync + 'static {
    /// The value created by the process.
    type Value: Send + Sync;

    /// Executes the reactive process in the runtime, calls `next` with the resulting value.
    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value>;
    
    /// Suspends the execution of a process until next instant.
    fn pause(self) -> Pause<Self> where Self: Sized {
        Pause(self)
    }
    
    /// Applies a function to the value returned by the process before passing it to
    /// its continuation.
    fn map<F, V>(self, map: F) -> Map<Self, F>
        where Self: Sized, F: FnOnce(Self::Value) -> V + Send + Sync + 'static
    {
        Map { process: self, map }
    }
    
    /// Flattens the execution of a process when its returned value is itself another process.
    fn flatten(self) -> Flatten<Self> where Self: Sized, Self::Value: Process {
        Flatten(self)
    }
    
    /// Chains another process after the exectution of one process (like the `bind` for a monad).
    fn and_then<F, P>(self, chain: F) -> AndThen<Self, F>
        where Self: Sized, F: FnOnce(Self::Value) -> P + Send + Sync + 'static, P: Process
    {
        AndThen { process: self, chain }
    }
    
    /// Executes a second process after one process terminates.
    /// The returned value of the first process is ignored.
    fn then<P>(self, successor: P) -> Then<Self, P> where Self: Sized, P: Process {
        Then { process: self, successor }
    }
    
    /// Decides whether to execute `if_branch` or `else_branch` according to
    /// the returned value of `self`, which must be of type `bool`.  
    /// The combinator `and_then` defined earlier together with the built-in
    /// `if`-`else` branching in Rust cannot allow us to achieve the same purpose
    /// since `if` branch and `else` branch in Rust must result in the same type.
    fn if_else<P1, P2, V>(self, if_branch: P1, else_branch: P2) -> IfElse<Self, P1, P2>
        where Self: Process<Value=bool> + Sized,
              P1: Process<Value=V>,
              P2: Process<Value=V>,
              V: Send + Sync,
    {
        IfElse {
            process: self,
            if_branch,
            else_branch,
        }
    }
    
    /// Executes two processes in parallel. 
    fn join<P>(self, proc2: P) -> Join<Self, P> where Self: Sized, P: Process {
        Join(self, proc2)
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

use std::rc::Rc;
use std::cell::RefCell;

mod process_mut;
pub use self::process_mut::{ProcessMut, While};

mod value;
mod pause;
mod map;
mod flatten;
mod and_then;
mod join;
pub use self::value::{value, Value};
pub use self::pause::Pause;
pub use self::map::Map;
pub use self::flatten::Flatten;
pub use self::and_then::AndThen;
pub use self::join::Join;

use {Runtime, Continuation};

/// A reactive process.
pub trait Process: 'static {
    /// The value created by the process.
    type Value;

    /// Executes the reactive process in the runtime, calls `next` with the resulting value.
    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value>;

    /// Suspends the execution of a process until next instant.
    fn pause(self) -> Pause<Self> where Self: Sized {
        Pause(self)
    }

    /// Apply a function to the value returned by the process before passing it to
    /// its continuation.
    fn map<F, V>(self, map: F) -> 
        Map<Self, F> where Self: Sized, F: FnOnce(Self::Value) -> V + 'static
    {
        Map { process: self, map }
    }

    /// Flatten the execution of a process when its returned value is itself another process.
    fn flatten(self) -> Flatten<Self> where Self: Sized, Self::Value: Process {
        Flatten(self)
    }

    /// Chain another process after the exectution of one process (like the `bind` for a monad).
    fn and_then<F, P>(self, chain: F) ->
        AndThen<Self, F> where Self: Sized, F: FnOnce(Self::Value) -> P + 'static, P: Process
    {
        AndThen { process: self, chain }
    }

    /// Execute two processes in parallel. 
    fn join<P>(self, proc2: P) -> Join<Self, P> where Self: Sized, P: Process {
        Join(self, proc2)
    }

    // TODO: add combinators
}

/// Execute a process in a newly created runtime and return its value.
pub fn execute_process<P>(p: P) -> P::Value where P: Process {
    let mut runtime = Runtime::new();
    let res: Rc<RefCell<Option<P::Value>>> = Rc::new(RefCell::new(None));
    let res2 = res.clone();
    let c = move |_: &mut Runtime, v| { *res2.borrow_mut() = Some(v); drop(res2) };
    runtime.on_current_instant(Box::new(|r: &mut Runtime, _| p.call(r, c)));
    runtime.execute();
    Rc::try_unwrap(res).map_err(|_| ()).unwrap().into_inner().unwrap()
}

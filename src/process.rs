use std::rc::Rc;
use std::cell::RefCell;
use runtime::Runtime;
use continuation::Continuation;

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
        //Flatten<Map<Self, F>> where Self: Sized, F: FnOnce(Self::Value) -> P + 'static, P: Process
        AndThen<Self, F> where Self: Sized, F: FnOnce(Self::Value) -> P + 'static, P: Process
    {
        AndThen { process: self, chain }
        //self.map(chain).flatten()
    }

    // TODO: add combinators
}

pub fn execute_process<P>(p: P) -> Rc<RefCell<Option<P::Value>>> where P: Process {
    let mut runtime = Runtime::new();
    let res: Rc<RefCell<Option<P::Value>>> = Rc::new(RefCell::new(None));
    let res2 = res.clone();
    let c = move |_: &mut Runtime, v| *res2.borrow_mut() = Some(v);
    p.call(&mut runtime, c);
    runtime.execute();
    res
}

/// A process that return a value of type V.
pub struct Value<V>(V);

impl<V> Process for Value<V> where V: 'static {
    type Value = V;
    
    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<V> {
        next.call(runtime, self.0);
    }
}

/// Create a new process that returns the value v immediately.
pub fn value<V>(v: V) -> Value<V> {
    Value(v)
}

/// A process that applies a function to the returned value of another process.
pub struct Map<P, F> { process: P, map: F }

impl<P, F, V> Process for Map<P, F> 
    where P: Process, F: FnOnce(P::Value) -> V + 'static
{
    type Value = V;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.process.call(runtime, next.map(self.map));
    }
}

/// The process is suspended until next instant.
pub struct Pause<P>(P);

impl<P> Process for Pause<P> where P: Process {
    type Value = P::Value;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.0.call(runtime, next.pause());
    }
}

/// Flatten the process when it returns another process to get only the final process.
pub struct Flatten<P>(P);

impl<P> Process for Flatten<P> where P: Process, P::Value: Process {
    type Value = <<P as Process>::Value as Process>::Value;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let c = |r: &mut Runtime, p: P::Value| p.call(r, next);
        self.0.call(runtime, c);
    }
}

/// Chain a computation onto the end of another process.
pub struct AndThen<P, F> { process: P, chain: F }

impl<P1, P2, F> Process for AndThen<P1, F>
    where P1: Process, P2: Process, F: FnOnce(P1::Value) -> P2 + 'static
{
    type Value = P2::Value;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let chain = self.chain;
        let c = |r: &mut Runtime, v: P1::Value| chain(v).call(r, next);
        self.process.call(runtime, c);
    }
}

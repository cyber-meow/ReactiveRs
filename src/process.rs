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

    // TODO: add combinators
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

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<V> {
        self.process.call(runtime, next.map(self.map));
    }
}

/// The process is suspended until next instant.
pub struct Pause<P>(P);

impl<P> Process for Pause<P> where P: Process {
    type Value = P::Value;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<P::Value> {
        self.0.call(runtime, next.pause());
    }
}

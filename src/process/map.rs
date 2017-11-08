use {Runtime, Continuation};
use process::{Process, ProcessMut};

/// A process that applies a function to the returned value of another process.
pub struct Map<P, F> { pub(crate) process: P, pub(crate) map: F }

impl<P, F, V> Process for Map<P, F> 
    where P: Process, F: FnOnce(P::Value) -> V + 'static
{
    type Value = V;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.process.call(runtime, next.map(self.map));
    }
}

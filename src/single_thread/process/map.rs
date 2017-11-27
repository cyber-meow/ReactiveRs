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

impl<P, F, V> ProcessMut for Map<P,F>
    where P: ProcessMut, F: FnMut(P::Value) -> V + 'static
{
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        let process = self.process;
        let mut f = self.map;
        process.call_mut(
            runtime,
            next.map(move |(process, v): (P, P::Value)| {
                let new_v = f(v);
                (process.map(f), new_v)
            })
        )
    }
}

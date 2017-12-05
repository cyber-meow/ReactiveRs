use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

/// A process that applies a function to the returned value of another process.
pub struct Map<P, F> { pub(crate) process: P, pub(crate) map: F }

impl<P, F, V> Process for Map<P, F>
    where P: Process, F: FnOnce(P::Value) -> V + 'static
{
    type Value = V;
}

// Implements the traits for the single thread version of the library.

impl<P, F, V> ProcessSt for Map<P, F> 
    where P: ProcessSt, F: FnOnce(P::Value) -> V + 'static
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        self.process.call(runtime, next.map(self.map));
    }
}

impl<P, F, V> ProcessMutSt for Map<P,F>
    where P: ProcessMutSt, F: FnMut(P::Value) -> V + 'static
{
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
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

// Implements the traits for the parallel version of the library.

impl<P, F, V> ConstraintOnValue for Map<P, F>
    where P: Process, F: FnOnce(P::Value) -> V + 'static, V: Send + Sync
{
    type T = V;
}

// Note that we must use `P::T` instead of `P::Value` because in the definition of 
// `ProcessPl` we have <Value = <Self::ConstraintOnValue>::T>.

impl<P, F, V> ProcessPl for Map<P, F> 
    where P: ProcessPl, F: FnOnce(P::T) -> V + Send + Sync + 'static, V: Send + Sync
{
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        self.process.call(runtime, next.map(self.map));
    }
}

impl<P, F, V> ProcessMutPl for Map<P,F>
    where P: ProcessMutPl, F: FnMut(P::T) -> V + Send + Sync + 'static, V: Send + Sync
{
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
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

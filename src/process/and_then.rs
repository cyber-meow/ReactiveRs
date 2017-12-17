use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

/// Chains a computation onto the end of another process.
pub struct AndThen<P, F> { pub(crate) process: P, pub(crate) chain: F }

impl<P1, P2, F> Process for AndThen<P1, F>
    where P1: Process, P2: Process, F: FnOnce(P1::Value) -> P2 + 'static
{
    type Value = P2::Value;
}

impl<P1, P2, F> ProcessMut for AndThen<P1, F>
    where P1: ProcessMut, P2: Process, F: FnMut(P1::Value) -> P2 + 'static {}

// Implements the traits for the single thread version of the library.

impl<P1, P2, F> ProcessSt for AndThen<P1, F>
    where P1: ProcessSt, P2: ProcessSt, F: FnOnce(P1::Value) -> P2 + 'static
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let chain = self.chain;
        let c = |r: &mut SingleThreadRuntime, v: P1::Value| chain(v).call(r, next);
        self.process.call(runtime, c);
    }
}

impl<P1, P2, F> ProcessMutSt for AndThen<P1, F>
    where P1: ProcessMutSt, P2: ProcessSt, F: FnMut(P1::Value) -> P2 + 'static
{
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let mut chain = self.chain;
        let c = |r: &mut SingleThreadRuntime, (process, v): (P1, P1::Value)| {
            chain(v).call(r, next.map(|v2| (process.and_then(chain), v2)));
        };
        self.process.call_mut(runtime, c);
    }
}

// Implements the traits for the parallel version of the library.

impl<P1, P2, F> ConstraintOnValue for AndThen<P1, F>
    where P1: Process, P2: ProcessPl, F: FnOnce(P1::Value) -> P2 + 'static
{
    type T = P2::Value;
}

// Note that we must use `P::T` instead of `P::Value` because in the definition of 
// `ProcessPl` we have <Value = <Self::ConstraintOnValue>::T>.

impl<P1, P2, F> ProcessPl for AndThen<P1, F>
    where P1: ProcessPl, P2: ProcessPl, F: FnOnce(P1::T) -> P2 + Send + Sync + 'static
{
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        let chain = self.chain;
        let c = |r: &mut ParallelRuntime, v: P1::Value| chain(v).call(r, next);
        self.process.call(runtime, c);
    }
}

impl<P1, P2, F> ProcessMutPl for AndThen<P1, F>
    where P1: ProcessMutPl, P2: ProcessPl, F: FnMut(P1::T) -> P2 + Send + Sync + 'static
{
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let mut chain = self.chain;
        let c = |r: &mut ParallelRuntime, (process, v): (P1, P1::Value)| {
            chain(v).call(r, next.map(|v2| (process.and_then(chain), v2)));
        };
        self.process.call_mut(runtime, c);
    }
}

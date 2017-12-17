use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

/// Flattens the process when it returns another process to get only the
/// result of the final process.
pub struct Flatten<P>(pub(crate) P);

impl<P> Process for Flatten<P> where P: Process, P::Value: Process {
    type Value = <P::Value as Process>::Value;
}

impl<P> ProcessMut for Flatten<P> where P: ProcessMut, P::Value: Process {}

// Implements the traits for the single thread version of the library.

impl<P> ProcessSt for Flatten<P> where P: ProcessSt, P::Value: ProcessSt {
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let c = |r: &mut SingleThreadRuntime, p: P::Value| p.call(r, next);
        self.0.call(runtime, c);
    }
}

impl<P> ProcessMutSt for Flatten<P> where P: ProcessMutSt, P::Value: ProcessSt {
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let c = |r: &mut SingleThreadRuntime, (p_old, p_new): (P, P::Value)| {
            p_new.call(r, next.map(|v| (p_old.flatten(), v)));
        };
        self.0.call_mut(runtime, c);
    }
}

// Implements the traits for the parallel version of the library.

impl<P> ConstraintOnValue for Flatten<P> where P: Process, P::Value: ProcessPl {
    type T = <P::Value as Process>::Value;
}

// Note that we must use `P::T` instead of `P::Value` because in the definition of 
// `ProcessPl` we have <Value = <Self::ConstraintOnValue>::T>.

impl<P> ProcessPl for Flatten<P> where P: ProcessPl, P::T: ProcessPl {
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        let c = |r: &mut ParallelRuntime, p: P::Value| p.call(r, next);
        self.0.call(runtime, c);
    }
}

impl<P> ProcessMutPl for Flatten<P> where P: ProcessMutPl, P::T: ProcessPl {
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let c = |r: &mut ParallelRuntime, (p_old, p_new): (P, P::Value)| {
            p_new.call(r, next.map(|v| (p_old.flatten(), v)));
        };
        self.0.call_mut(runtime, c);
    }
}

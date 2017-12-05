use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{Continuation, ContinuationSt, ContinuationPl};
use process::{Process, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

/// The process is suspended until next instant.
pub struct Pause<P>(pub(crate) P);

impl<P> Process for Pause<P> where P: Process {
    type Value = P::Value;
}

impl<P> ProcessSt for Pause<P> where P: ProcessSt {
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        self.0.call(runtime, next.pause());
    }
}

impl<P> ProcessMutSt for Pause<P> where P: ProcessMutSt {
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        self.0.call_mut(
            runtime,
            next.pause().map(|(process, v): (P, P::Value)| (process.pause(), v)));
    }
}

impl<P> ConstraintOnValue for Pause<P> where P: ProcessPl {
    type T = P::Value;
}

impl<P> ProcessPl for Pause<P> where P: ProcessPl {
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        self.0.call(runtime, next.pause());
    }
}

impl<P> ProcessMutPl for Pause<P> where P: ProcessMutPl {
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        self.0.call_mut(
            runtime,
            next.pause().map(|(process, v): (P, P::Value)| (process.pause(), v)));
    }
}

use runtime::{Runtime, SingleThreadRuntime, ParallelRuntime};
use continuation::{Continuation, ContinuationSt};
use process::{Process, ProcessSt, ProcessPl, ConstraintOnValue};
use process::{ProcessMut, ProcessMutSt, ProcessMutPl};

/// The process is suspended until next instant.
pub struct Pause<P>(pub(crate) P);

// Since the implementation of `pause` for `Continuation` depends on the
// type of runtime, this must also be the case here.
// And I wasn't able to figure out a method to merge them all together.
//
/// The single thread process implementation for `Pause`.
impl<P> Process<SingleThreadRuntime> for Pause<P> where P: ProcessSt {
    type Value = P::Value;

    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        self.0.call(runtime, next.pause());
    }
}

impl<P> ProcessMut<SingleThreadRuntime> for Pause<P> where P: ProcessMutSt {
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        self.0.call_mut(
            runtime,
            next.pause().map(|(process, v): (P, P::Value)| (process.pause(), v)));
    }
}

/// The parallel process implementation for `Pause`.
impl<P> Process<ParallelRuntime> for Pause<P> where P: ProcessPl {
    type Value = P::Value;

    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: Continuation<ParallelRuntime, Self::Value>
    {
        self.0.call(runtime, next.pause());
    }
}

impl<P> ProcessMut<ParallelRuntime> for Pause<P> where P: ProcessMutPl {
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: Continuation<ParallelRuntime, (Self, Self::Value)>
    {
        self.0.call_mut(
            runtime,
            next.pause().map(|(process, v): (P, P::Value)| (process.pause(), v)));
    }
}

impl<P> ConstraintOnValue for Pause<P> where P: ProcessPl {
    type T = P::Value;
}

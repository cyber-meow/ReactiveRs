use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{Continuation, ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

/// Repeats a process forever.
pub struct Loop<P>(pub(crate) P);

impl<P> Process for Loop<P> where P: ProcessMut {
    type Value = ();
}

// TODO: This is useless without other control structures.
impl<P> ProcessMut for Loop<P> where P: ProcessMut {}

// Implements the traits for the single thread version of the library.

impl<P> ProcessSt for Loop<P> where P: ProcessMutSt {
    fn call<C>(self, runtime: &mut SingleThreadRuntime, _: C)
        where C: ContinuationSt<Self::Value>
    {
        self.0.call_mut(runtime, LoopContinuation);
    }
}

// TODO: This is useless without other control structures.
impl<P> ProcessMutSt for Loop<P> where P: ProcessMutSt {
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, _: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        self.0.call_mut(runtime, LoopContinuation);
    }
}

/// The continuation to call when using the `loop_proc` combinator.
pub struct LoopContinuation;

impl<P> Continuation<SingleThreadRuntime, (P, P::Value)> for LoopContinuation
    where P: ProcessMutSt
{
    fn call(self, runtime: &mut SingleThreadRuntime, (process, _): (P, P::Value)) {
        process.call_mut(runtime, self);
    }

    fn call_box(self: Box<Self>, runtime: &mut SingleThreadRuntime, value: (P, P::Value)) {
        (*self).call(runtime, value);
    }
}

// Implements the traits for the parallel version of the library.

impl<P> ConstraintOnValue for Loop<P> where P: ProcessMut {
    type T = ();
}

impl<P> ProcessPl for Loop<P> where P: ProcessMutPl {
    fn call<C>(self, runtime: &mut ParallelRuntime, _: C)
        where C: ContinuationPl<Self::Value>
    {
        self.0.call_mut(runtime, LoopContinuation);
    }
}

// TODO: This is useless without other control structures.
impl<P> ProcessMutPl for Loop<P> where P: ProcessMutPl {
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, _: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        self.0.call_mut(runtime, LoopContinuation);
    }
}

impl<P> Continuation<ParallelRuntime, (P, P::Value)> for LoopContinuation
    where P: ProcessMutPl
{
    fn call(self, runtime: &mut ParallelRuntime, (process, _): (P, P::Value)) {
        process.call_mut(runtime, self);
    }

    fn call_box(self: Box<Self>, runtime: &mut ParallelRuntime, value: (P, P::Value)) {
        (*self).call(runtime, value);
    }
}

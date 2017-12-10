use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{Continuation, ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

/// Repeats a process a several times and the produced value is returned at the end.
pub struct Repeat<P> { pub(crate) process: P, pub(crate) times: usize }

impl<P> Process for Repeat<P> where P: ProcessMut {
    type Value = P::Value;
}

impl<P> ProcessMut for Repeat<P> where P: ProcessMut {}

// Implements the traits for the single thread version of the library.

impl<P> ProcessSt for Repeat<P> where P: ProcessMutSt {
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let c = RepeatContinuation {
            repeated_times: self.times,
            counter: self.times,
            continuation: next.map(|(_, v)| v),
        };
        self.process.call_mut(runtime, c);
    }
}

impl<P> ProcessMutSt for Repeat<P> where P: ProcessMutSt {
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let c = RepeatContinuation {
            repeated_times: self.times,
            counter: self.times,
            continuation: next,
        };
        self.process.call_mut(runtime, c);
    }
}

/// The continuation to call when using `repeat` combinator.
/// The field `counter` is decreased by one each time the struct is called
/// and the field `continuation` is called once `counter` gets zero.
pub struct RepeatContinuation<C> {
    repeated_times: usize,
    counter: usize,
    continuation: C,
}

impl<P, C> Continuation<SingleThreadRuntime, (P, P::Value)> for RepeatContinuation<C>
    where P: ProcessMutSt, C: ContinuationSt<(Repeat<P>, P::Value)>
{
    fn call(mut self, runtime: &mut SingleThreadRuntime, (p, v): (P, P::Value)) {
        self.counter -= 1;
        if self.counter == 0 {
            self.continuation.call(runtime, (p.repeat(self.repeated_times), v));
        } else {
            p.call_mut(runtime, self)
        }
    }
    
    fn call_box(self: Box<Self>, runtime: &mut SingleThreadRuntime, value: (P, P::Value)) {
        (*self).call(runtime, value);
    }
}

// Implements the traits for the parallel version of the library.

impl <P> ConstraintOnValue for Repeat<P> where P: ProcessMut, P::Value: Send + Sync {
    type T = P::Value;
}

impl<P> ProcessPl for Repeat<P> where P: ProcessMutPl {
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        let c = RepeatContinuation {
            repeated_times: self.times,
            counter: self.times,
            continuation: next.map(|(_, v)| v),
        };
        self.process.call_mut(runtime, c);
    }
}

impl<P> ProcessMutPl for Repeat<P> where P: ProcessMutPl {
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let c = RepeatContinuation {
            repeated_times: self.times,
            counter: self.times,
            continuation: next,
        };
        self.process.call_mut(runtime, c);
    }
}

impl<P, C> Continuation<ParallelRuntime, (P, P::Value)> for RepeatContinuation<C>
    where P: ProcessMutPl, C: ContinuationPl<(Repeat<P>, P::Value)>
{
    fn call(mut self, runtime: &mut ParallelRuntime, (p, v): (P, P::Value)) {
        self.counter -= 1;
        if self.counter == 0 {
            self.continuation.call(runtime, (p.repeat(self.repeated_times), v));
        } else {
            p.call_mut(runtime, self)
        }
    }
    
    fn call_box(self: Box<Self>, runtime: &mut ParallelRuntime, value: (P, P::Value)) {
        (*self).call(runtime, value);
    }
}

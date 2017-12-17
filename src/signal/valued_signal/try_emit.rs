use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};
use signal::valued_signal::{ValuedSignal, CanEmit};

/// Process that represents an emission of a signal with some value.
pub struct TryEmitValue<S, A> {
    pub(crate) signal: S,
    pub(crate) emitted: A,
}

impl<S, A> Process for TryEmitValue<S, A> where S: ValuedSignal, A: 'static {
    type Value = bool;
}

impl<S, A> ProcessMut for TryEmitValue<S, A> where S: ValuedSignal, A: 'static {}

// Non-parallel

impl<S, A> ProcessSt for TryEmitValue<S, A>
    where S: ValuedSignal, S::RuntimeRef: CanEmit<SingleThreadRuntime, A>, A: 'static
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let res = self.signal.runtime().try_emit(runtime, self.emitted);
        next.call(runtime, res);
    }
}

impl<S, A> ProcessMutSt for TryEmitValue<S, A>
    where S: ValuedSignal, S::RuntimeRef: CanEmit<SingleThreadRuntime, A>, A: Clone + 'static
{
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let res = self.signal.runtime().try_emit(runtime, self.emitted.clone());
        next.call(runtime, (self, res));
    }
}

// Parallel

impl<S, A> ConstraintOnValue for TryEmitValue<S, A> {
    type T = bool;
}

impl<S, A> ProcessPl for TryEmitValue<S, A>
    where S: ValuedSignal + Send + Sync,
          S::RuntimeRef: CanEmit<ParallelRuntime, A>,
          A: Send + Sync + 'static,
{
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        let res = self.signal.runtime().try_emit(runtime, self.emitted);
        next.call(runtime, res);
    }
}

impl<S, A> ProcessMutPl for TryEmitValue<S, A>
    where S: ValuedSignal + Send + Sync,
          S::RuntimeRef: CanEmit<ParallelRuntime, A>,
          A: Clone + Send + Sync + 'static,
{
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let res = self.signal.runtime().try_emit(runtime, self.emitted.clone());
        next.call(runtime, (self, res));
    }
}

use runtime::{Runtime, SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};
use signal::signal_runtime::SignalRuntimeRefBase;
use signal::valued_signal::ValuedSignal;

/// Attemps to emit some value. Created by the method `try_emit`.
pub struct TryEmitValue<S, A> {
    pub(crate) signal: S,
    pub(crate) emitted: A,
}

impl<S, A> Process for TryEmitValue<S, A> where S: ValuedSignal, A: 'static {
    type Value = bool;
}

impl<S, A> ProcessMut for TryEmitValue<S, A> where S: ValuedSignal, A: 'static {}

pub trait CanTryEmit<R, A>: SignalRuntimeRefBase<R> where R: Runtime {
    /// Tries to emit a signal and indicates if the emission is successful.
    fn try_emit(&mut self, runtime: &mut R, emitted: A) -> bool;
}

// Non-parallel

impl<S, A> ProcessSt for TryEmitValue<S, A>
    where S: ValuedSignal, S::RuntimeRef: CanTryEmit<SingleThreadRuntime, A>, A: 'static
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let res = self.signal.runtime().try_emit(runtime, self.emitted);
        next.call(runtime, res);
    }
}

impl<S, A> ProcessMutSt for TryEmitValue<S, A>
    where S: ValuedSignal, S::RuntimeRef: CanTryEmit<SingleThreadRuntime, A>, A: Clone + 'static
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
          S::RuntimeRef: CanTryEmit<ParallelRuntime, A>,
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
          S::RuntimeRef: CanTryEmit<ParallelRuntime, A>,
          A: Clone + Send + Sync + 'static,
{
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let res = self.signal.runtime().try_emit(runtime, self.emitted.clone());
        next.call(runtime, (self, res));
    }
}

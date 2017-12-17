use runtime::{Runtime, SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};
use signal::signal_runtime::SignalRuntimeRefBase;
use signal::ValuedSignal;

/// Process that represents an emission of a signal with some value.
pub struct EmitValue<S, A> {
    pub(crate) signal: S,
    pub(crate) emitted: A,
}

impl<S, A> Process for EmitValue<S, A> where S: ValuedSignal, A: 'static {
    type Value = ();
}

impl<S, A> ProcessMut for EmitValue<S, A> where S: ValuedSignal, A: 'static {}

pub trait CanEmit<R, A>: SignalRuntimeRefBase<R> where R: Runtime {
    /// Emits the value `emitted` to the signal.
    fn emit(&mut self, runtime: &mut R, emitted: A);
    
    /// Tries to emit a signal and indicates the emission is successful.
    fn try_emit(&mut self, runtime: &mut R, emitted: A) -> bool {
        if self.is_emitted() {
            false
        } else {
            self.emit(runtime, emitted);
            true
        }
    }
}

// Non-parallel

impl<S, A> ProcessSt for EmitValue<S, A>
    where S: ValuedSignal, S::RuntimeRef: CanEmit<SingleThreadRuntime, A>, A: 'static
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        self.signal.runtime().emit(runtime, self.emitted);
        next.call(runtime, ());
    }
}

impl<S, A> ProcessMutSt for EmitValue<S, A>
    where S: ValuedSignal, S::RuntimeRef: CanEmit<SingleThreadRuntime, A>, A: Clone + 'static
{
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        self.signal.runtime().emit(runtime, self.emitted.clone());
        next.call(runtime, (self, ()));
    }
}

// Parallel

impl<S, A> ConstraintOnValue for EmitValue<S, A> {
    type T = ();
}

impl<S, A> ProcessPl for EmitValue<S, A>
    where S: ValuedSignal + Send + Sync,
          S::RuntimeRef: CanEmit<ParallelRuntime, A>,
          A: Send + Sync + 'static,
{
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        self.signal.runtime().emit(runtime, self.emitted);
        next.call(runtime, ());
    }
}

impl<S, A> ProcessMutPl for EmitValue<S, A>
    where S: ValuedSignal + Send + Sync,
          S::RuntimeRef: CanEmit<ParallelRuntime, A>,
          A: Clone + Send + Sync + 'static,
{
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        self.signal.runtime().emit(runtime, self.emitted.clone());
        next.call(runtime, (self, ()));
    }
}

use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

use signal::Signal;
use signal::signal_runtime::{SignalRuntimeRefSt, SignalRuntimeRefPl};

/// Process that awaits the emission of a signal.
pub struct AwaitImmediate<S>(pub(crate) S);

impl<S> Process for AwaitImmediate<S> where S: Signal {
    type Value = ();
}

impl<S> ProcessMut for AwaitImmediate<S> where S: Signal {}

// Implements the traits for the single thread version of the library.

impl<S> ProcessSt for AwaitImmediate<S>
    where S: Signal, S::RuntimeRef: SignalRuntimeRefSt
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        self.0.runtime().on_signal(runtime, next);
    }
}

impl<S> ProcessMutSt for AwaitImmediate<S>
    where S: Signal, S::RuntimeRef: SignalRuntimeRefSt
{
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        self.0.runtime().on_signal(
            runtime,
            next.map(|()| (self, ()))
        );
    }
}

// Implements the traits for the parallel version of the library.

impl<S> ConstraintOnValue for AwaitImmediate<S> where S: Signal {
    type T = ();
}

impl<S> ProcessPl for AwaitImmediate<S>
    where S: Signal + Send + Sync, S::RuntimeRef: SignalRuntimeRefPl
{
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        self.0.runtime().on_signal(runtime, next);
    }
}

impl<S> ProcessMutPl for AwaitImmediate<S>
    where S: Signal + Send + Sync, S::RuntimeRef: SignalRuntimeRefPl
{
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        self.0.runtime().on_signal(
            runtime,
            next.map(|()| (self, ()))
        );
    }
}

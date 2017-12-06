pub mod signal_runtime;

/*mod pure_signal;
pub use self::pure_signal::{PureSignal, new_pure_signal};
*/
use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

use self::signal_runtime::{SignalRuntimeRefSt, SignalRuntimeRefPl};

/// A reactive signal.  
/// The signal implement the trait `Clone` to assure that it can be used multiple times
/// in the program. However, note that for most of the constructions `clone` is used
/// implicitly so one can pass the signal directly.
/// The user needs to call `clone` explicitly only in scenarios where the signal's
/// ownership must be shared in different places, ex: closure.
pub trait Signal: Clone + 'static {
    /// The runtime reference type associated with the signal.
    type RuntimeRef;
    
    /// Returns a reference to the signal's runtime.
    fn runtime(&self) -> Self::RuntimeRef;
}/*    
    /// Returns a process that waits for the next emission of the signal, current instant
    /// included.
    fn await_immediate(&self) -> AwaitImmediate<Self> where Self: Sized {
        AwaitImmediate(self.clone())
    }
}*/
/*
pub trait SignalSt<RuntimeRef>: Signal<RuntimeRef> where RuntimeRef: SignalRuntimeRefSt {}

impl<RuntimeRef> SignalSt<RuntimeRef> for Signal<RuntimeRef>
    where RuntimeRef: SignalRuntimeRefSt {}
*/
/*/// Process that awaits the emission of a signal.
pub struct AwaitImmediate<S>(S);

impl<S> Process for AwaitImmediate<S> where S: Signal {
    type Value = ();
}

impl<S> ProcessMut for AwaitImmediate<S> where S: Signal {}

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
}*/

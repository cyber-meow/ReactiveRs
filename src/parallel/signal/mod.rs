use parallel::{Runtime, Continuation};
use parallel::process::{Process, ProcessMut};

pub mod signal_runtime;
use self::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRef};

mod pure_signal;
pub use self::pure_signal::PureSignal;
mod mpmc_signal;
pub use self::mpmc_signal::MpmcSignal;

/// A reactive signal.  
/// The signal implement the trait `Clone` to assure that it can be used multiple times
/// in the program. However, note that for most of the constructions `clone` is used
/// implicitly so one can pass the signal directly.
/// The user needs to call `clone` explicitly only in scenarios where the signal's
/// ownership must be shared in different places, ex: closure.
pub trait Signal: Clone + Send + Sync + 'static {
    /// The runtime reference type associated with the signal.
    type RuntimeRef: SignalRuntimeRef;

    /// Returns a reference to the signal's runtime.
    fn runtime(&self) -> Self::RuntimeRef;

    /// Returns a process that waits for the next emission of the signal, current instant
    /// included.
    fn await_immediate(&self) -> AwaitImmediate<Self> where Self: Sized {
        AwaitImmediate(self.clone())
    }

    /// Test the status of a signal `s`. If the signal is present, the process `p1`
    /// is executed instantaneously, other wise `p2` is executed at the following instant.
    fn present_else<P1, P2, V>(&self, p1: P1, p2: P2) -> PresentElse<Self, P1, P2>
        where Self: Sized, P1: Process<Value=V>, P2: Process<Value=V>, V: Send + Sync
    {
        PresentElse {
            signal: self.clone(),
            present_proc: p1,
            else_proc: p2,
        }
    }
}

/// Process that awaits the emission of a signal.
pub struct AwaitImmediate<S>(S);

impl<S> Process for AwaitImmediate<S> where S: Signal {
    type Value = ();
    
    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.0.runtime().on_signal(runtime, next);
    }
}

impl<S> ProcessMut for AwaitImmediate<S> where S: Signal {
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        self.0.runtime().on_signal(
            runtime,
            next.map(|()| (self, ()))
        );
    }
}

/// Process that tests the status of a signal and chooses the branch to execute
/// according to the result.
pub struct PresentElse<S, P1, P2> {
    signal: S,
    present_proc: P1,
    else_proc: P2,
}

impl<S, P1, P2, V> Process for PresentElse<S, P1, P2>
    where S: Signal, P1: Process<Value=V>, P2: Process<Value=V>, V: Send + Sync
{
    type Value = V;
    
    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let mut signal_runtime = self.signal.runtime();
        let c = |r: &mut Runtime, ()| {
            if self.signal.runtime().is_emitted() {
                let present_continuation =
                    move |r: &mut Runtime, ()| { self.present_proc.call(r, next) };
                r.on_current_instant(Box::new(present_continuation));
            } else {
                let else_continuation =
                    move |r: &mut Runtime, ()| { self.else_proc.call(r, next) };
                r.on_next_instant(Box::new(else_continuation));
            }
        };
        signal_runtime.on_signal_present(runtime, c);
        runtime.add_test_signal(Box::new(signal_runtime));
    }
}

impl <S, P1, P2, V> ProcessMut for PresentElse<S, P1, P2>
    where S: Signal, P1: ProcessMut<Value=V>, P2: ProcessMut<Value=V>, V: Send + Sync
{
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        let mut signal_runtime = self.signal.runtime();
        let c = move |r: &mut Runtime, ()| {
            let signal = self.signal;
            let (else_proc, present_proc) = (self.else_proc, self.present_proc);
            if signal.runtime().is_emitted() {
                let present_continuation = move |r: &mut Runtime, ()| {
                    present_proc.call_mut(
                        r, next.map(move |(p, v)| (signal.present_else(p, else_proc), v)))
                };
                r.on_current_instant(Box::new(present_continuation));
            } else {
                let else_continuation = |r: &mut Runtime, ()| {
                    else_proc.call_mut(r, next.map(
                        move |(p, v)| (signal.present_else(present_proc, p), v)))
                };
                r.on_next_instant(Box::new(else_continuation));
            }
        };
        signal_runtime.on_signal_present(runtime, c);
        runtime.add_test_signal(Box::new(signal_runtime));
    }
}

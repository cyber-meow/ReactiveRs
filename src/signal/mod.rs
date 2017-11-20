use {Runtime, Continuation};
use process::{Process, ProcessMut};

pub mod signal_runtime;
use self::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRef};

mod pure_signal;
pub use self::pure_signal::PureSignal;

/// A reactive signal.
pub trait Signal: Clone + 'static {
    /// The runtime reference type associated with the signal.
    type RuntimeReference: SignalRuntimeRef;

    /// Returns a reference to the signal's runtime.
    fn runtime(&mut self) -> Self::RuntimeReference;

    /// Returns a process that waits for the next emission of the signal, current instant
    /// included.
    fn await_immediate(&mut self) -> AwaitImmediate<Self> where Self: Sized {
        AwaitImmediate(self.clone())
    }

    fn present_else<P1, P2>(&mut self, p1: P1, p2: P2) -> PresentElse<Self, P1, P2>
        where Self: Sized, P1: Process, P2: Process
    {
        PresentElse {
            signal: self.clone(),
            present_proc: p1,
            else_proc: p2,
        }
    }
}

pub struct AwaitImmediate<S>(S);

impl<S> Process for AwaitImmediate<S> where S: Signal {
    type Value = ();
    
    fn call<C>(mut self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.0.runtime().on_signal(runtime, next);
    }
}

impl<S> ProcessMut for AwaitImmediate<S> where S: Signal {
    fn call_mut<C>(mut self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        self.0.runtime().on_signal(
            runtime,
            next.map(|()| (self, ()))
        );
    }
}

pub struct PresentElse<S, P1, P2> {
    signal: S,
    present_proc: P1,
    else_proc: P2,
}

impl<S, P1, P2, V> Process for PresentElse<S, P1, P2>
    where S: Signal, P1: Process<Value=V>, P2: Process<Value=V>
{
    type Value = V;
    
    fn call<C>(mut self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let mut signal_runtime = self.signal.runtime();
        let c = |r: &mut Runtime, ()| {
            if self.signal.runtime().is_emitted() {
                self.present_proc.call(r, next);
            } else {
                r.on_next_instant(Box::new(
                    move |r: &mut Runtime, ()| self.else_proc.call(r, next)));
            }
        };
        signal_runtime.on_signal_present(runtime, c);
        runtime.add_test_signal(signal_runtime);
    }
}

impl <S, P1, P2, V> ProcessMut for PresentElse<S, P1, P2>
    where S: Signal, P1: ProcessMut<Value=V>, P2: ProcessMut<Value=V>
{
    fn call_mut<C>(mut self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        let mut signal_runtime = self.signal.runtime();
        let c = move |r: &mut Runtime, ()| {
            let mut signal = self.signal;
            let (else_proc, present_proc) = (self.else_proc, self.present_proc);
            if signal.runtime().is_emitted() {
                present_proc.call_mut(
                    r, next.map(move |(p, v)| (signal.present_else(p, else_proc), v)))
            } else {
                r.on_next_instant(Box::new(
                    move |r: &mut Runtime, ()| {
                        else_proc.call_mut(
                            r,
                            next.map(move |(p, v)| (signal.present_else(present_proc, p), v))
                        )
                    }
                ));
            }
        };
        signal_runtime.on_signal_present(runtime, c);
        runtime.add_test_signal(signal_runtime);
    }   
}

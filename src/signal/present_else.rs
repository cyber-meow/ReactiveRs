use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

use signal::Signal;
use signal::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRefSt, SignalRuntimeRefPl};

/// Process that tests the status of a signal and chooses the branch to execute
/// according to the result.
pub struct PresentElse<S, P1, P2> {
    pub(crate) signal: S,
    pub(crate) present_proc: P1,
    pub(crate) else_proc: P2,
}

impl<S, P1, P2, V> Process for PresentElse<S, P1, P2>
    where S: Signal, P1: Process<Value=V>, P2: Process<Value=V>
{
    type Value = V;
}

impl<S, P1, P2, V> ProcessMut for PresentElse<S, P1, P2>
    where S: Signal, P1: ProcessMut<Value=V>, P2: ProcessMut<Value=V> {}

// Implements the traits for the single thread version of the library.

impl<S, P1, P2, V> ProcessSt for PresentElse<S, P1, P2>
    where S: Signal,
          S::RuntimeRef: SignalRuntimeRefSt,
          P1: ProcessSt<Value=V>,
          P2: ProcessSt<Value=V>,
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let mut signal_runtime = self.signal.runtime();
        let c = |r: &mut SingleThreadRuntime, ()| {
            if self.signal.runtime().is_emitted() {
                self.present_proc.call(r, next);
            } else {
                let else_continuation = move |r: &mut SingleThreadRuntime, ()| {
                    self.else_proc.call(r, next)
                };
                r.on_next_instant(Box::new(else_continuation));
            }
        };
        signal_runtime.on_signal_present(runtime, c);
        runtime.add_test_signal(Box::new(signal_runtime));
    }
}

impl <S, P1, P2, V> ProcessMutSt for PresentElse<S, P1, P2>
    where S: Signal,
          S::RuntimeRef: SignalRuntimeRefSt,
          P1: ProcessMutSt<Value=V>,
          P2: ProcessMutSt<Value=V>,
{
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let mut signal_runtime = self.signal.runtime();
        let c = move |r: &mut SingleThreadRuntime, ()| {
            let signal = self.signal;
            let (else_proc, present_proc) = (self.else_proc, self.present_proc);
            if signal.runtime().is_emitted() {
                present_proc.call_mut(
                    r, next.map(move |(p, v)| (signal.present_else(p, else_proc), v)))
            } else {
                let else_continuation = |r: &mut SingleThreadRuntime, ()| {
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

// Implements the traits for the parallel version of the library.

impl<S, P1, P2, V> ConstraintOnValue for PresentElse<S, P1, P2>
    where S: Signal, P1: Process<Value=V>, P2: Process<Value=V>, V: Send + Sync
{
    type T = V;
}

impl<S, P1, P2, V> ProcessPl for PresentElse<S, P1, P2>
    where S: Signal + Send + Sync,
          S::RuntimeRef: SignalRuntimeRefPl,
          P1: ProcessPl<T=V>,
          P2: ProcessPl<T=V>,
          V: Send + Sync,
{
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        let mut signal_runtime = self.signal.runtime();
        let c = |r: &mut ParallelRuntime, ()| {
            if self.signal.runtime().is_emitted() {
                let present_continuation = 
                    move |r: &mut ParallelRuntime, ()| {
                    self.present_proc.call(r, next)
                };
                // It's important to use `on_current_instant` instead of calling the
                // continuation directly because we may have a lot of continuations
                // bound a same signal.
                r.on_current_instant(Box::new(present_continuation));
            } else {
                let else_continuation =
                    move |r: &mut ParallelRuntime, ()| { self.else_proc.call(r, next) };
                r.on_next_instant(Box::new(else_continuation));
            }
        };
        signal_runtime.on_signal_present(runtime, c);
        runtime.add_test_signal(Box::new(signal_runtime));
    }
}

impl <S, P1, P2, V> ProcessMutPl for PresentElse<S, P1, P2>
    where S: Signal + Send + Sync,
          S::RuntimeRef: SignalRuntimeRefPl,
          P1: ProcessMutPl<T=V>,
          P2: ProcessMutPl<T=V>,
          V: Send + Sync,
{
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let mut signal_runtime = self.signal.runtime();
        let c = move |r: &mut ParallelRuntime, ()| {
            let signal = self.signal;
            let (else_proc, present_proc) = (self.else_proc, self.present_proc);
            if signal.runtime().is_emitted() {
                let present_continuation = move |r: &mut ParallelRuntime, ()| {
                    present_proc.call_mut(
                        r, next.map(move |(p, v)| (signal.present_else(p, else_proc), v)))
                };
                r.on_current_instant(Box::new(present_continuation));
            } else {
                let else_continuation = |r: &mut ParallelRuntime, ()| {
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

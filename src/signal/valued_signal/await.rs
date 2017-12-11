use std::marker::PhantomData;

use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};
use signal::valued_signal::{ValuedSignal, SignalType, MpSignal, SpSignal};
use signal::signal_runtime::{SignalRuntimeRefSt, SignalRuntimeRefPl};

/// Process awaiting a signal to be emitted to fetch its value.
pub struct AwaitValue<S, T>{
    pub(crate) signal: S,
    pub(crate) signal_type: PhantomData<T>,
}

impl<S, T> Process for AwaitValue<S, T> where S: ValuedSignal<SigType=T>, T: SignalType {
    type Value = S::Stored;
}

impl<S, T> ProcessMut for AwaitValue<S, T> where S: ValuedSignal<SigType=T>, T: SignalType {}

impl<S, T> ConstraintOnValue for AwaitValue<S, T>
    where S: ValuedSignal<SigType=T>, S::Stored: Send + Sync, T: SignalType
{
    type T = S::Stored;
}

pub trait GetValue<V> {
    /// Returns the value of the signal for the current instant.
    fn get_value(&self) -> V;
}

/* Multi-producer */

// Non-parallel

impl<S> ProcessSt for AwaitValue<S, MpSignal>
    where S: ValuedSignal<SigType=MpSignal>,
          S::RuntimeRef: GetValue<S::Stored> + SignalRuntimeRefSt,
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let signal_runtime = self.signal.runtime();
        let eoi_continuation = move |r: &mut SingleThreadRuntime, ()| {
            let stored = signal_runtime.get_value();
            r.on_next_instant(
                Box::new(|r: &mut SingleThreadRuntime, ()| next.call(r, stored)));
        };
        self.signal.runtime().on_signal(
            runtime,
            |r: &mut SingleThreadRuntime, ()| r.on_end_of_instant(Box::new(eoi_continuation)));
    }
}

impl<S> ProcessMutSt for AwaitValue<S, MpSignal>
    where S: ValuedSignal<SigType=MpSignal>,
          S::RuntimeRef: GetValue<S::Stored> + SignalRuntimeRefSt,
{
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let signal_runtime = self.signal.runtime();
        let mut signal_runtime2 = self.signal.runtime();
        let eoi_continuation = move |r: &mut SingleThreadRuntime, ()| {
            let stored = signal_runtime.get_value();
            r.on_next_instant(
                Box::new(|r: &mut SingleThreadRuntime, ()| next.call(r, (self, stored))));
        };
        signal_runtime2.on_signal(
            runtime,
            |r: &mut SingleThreadRuntime, ()| r.on_end_of_instant(Box::new(eoi_continuation)));
    }
}

// Parallel

impl<S> ProcessPl for AwaitValue<S, MpSignal>
    where S: ValuedSignal<SigType=MpSignal> + Send + Sync, 
          S::Stored: Send + Sync,
          S::RuntimeRef: GetValue<S::Stored> + SignalRuntimeRefPl + Send + Sync,
{
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        let signal_runtime = self.signal.runtime();
        let eoi_continuation = move |r: &mut ParallelRuntime, ()| {
            let stored = signal_runtime.get_value();
            r.on_next_instant(Box::new(|r: &mut ParallelRuntime, ()| next.call(r, stored)));
        };
        self.signal.runtime().on_signal(
            runtime,
            |r: &mut ParallelRuntime, ()| r.on_end_of_instant(Box::new(eoi_continuation)));
    }
}

impl<S> ProcessMutPl for AwaitValue<S, MpSignal>
    where S: ValuedSignal<SigType=MpSignal> + Send + Sync, 
          S::Stored: Send + Sync,
          S::RuntimeRef: GetValue<S::Stored> + SignalRuntimeRefPl + Send + Sync,
{
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let signal_runtime = self.signal.runtime();
        let mut signal_runtime2 = self.signal.runtime();
        let eoi_continuation = move |r: &mut ParallelRuntime, ()| {
            let stored = signal_runtime.get_value();
            r.on_next_instant(
                Box::new(|r: &mut ParallelRuntime, ()| next.call(r, (self, stored))));
        };
        signal_runtime2.on_signal(
            runtime,
            |r: &mut ParallelRuntime, ()| r.on_end_of_instant(Box::new(eoi_continuation)));
    }
}

/* Single-producer */

// Non-parallel

impl<S> ProcessSt for AwaitValue<S, SpSignal>
    where S: ValuedSignal<SigType=SpSignal>,
          S::RuntimeRef: GetValue<S::Stored> + SignalRuntimeRefSt,
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let signal_runtime = self.signal.runtime();
        self.signal.runtime().on_signal(
            runtime,
            move |r: &mut SingleThreadRuntime, ()| next.call(r, signal_runtime.get_value()));
    }
}

impl<S> ProcessMutSt for AwaitValue<S, SpSignal>
    where S: ValuedSignal<SigType=SpSignal>,
          S::RuntimeRef: GetValue<S::Stored> + SignalRuntimeRefSt,
{
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let signal_runtime = self.signal.runtime();
        let mut signal_runtime2 = self.signal.runtime();
        signal_runtime2.on_signal(
            runtime,
            move |r: &mut SingleThreadRuntime, ()|
                next.call(r, (self, signal_runtime.get_value())));
    }
}

// Parallel

impl<S> ProcessPl for AwaitValue<S, SpSignal>
    where S: ValuedSignal<SigType=SpSignal> + Send + Sync, 
          S::Stored: Send + Sync,
          S::RuntimeRef: GetValue<S::Stored> + SignalRuntimeRefPl + Send + Sync,
{
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        let signal_runtime = self.signal.runtime();
        self.signal.runtime().on_signal(
            runtime,
            move |r: &mut ParallelRuntime, ()| next.call(r, signal_runtime.get_value()));
    }
}

impl<S> ProcessMutPl for AwaitValue<S, SpSignal>
    where S: ValuedSignal<SigType=SpSignal> + Send + Sync, 
          S::Stored: Send + Sync,
          S::RuntimeRef: GetValue<S::Stored> + SignalRuntimeRefPl + Send + Sync,
{
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let signal_runtime = self.signal.runtime();
        let mut signal_runtime2 = self.signal.runtime();
        signal_runtime2.on_signal(
            runtime,
            move |r: &mut ParallelRuntime, ()|
                next.call(r, (self, signal_runtime.get_value())));
    }
}

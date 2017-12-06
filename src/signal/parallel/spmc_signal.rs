use std::sync::{Arc, Mutex};
use crossbeam::sync::TreiberStack;

use runtime::ParallelRuntime;
use continuation::ContinuationPl;
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

use signal::Signal;
use signal::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRefPl};
use signal::valued_signal::{ValuedSignal, CanEmit, Await};

/// A shared pointer to a signal runtime.
pub struct SpmcSignalRuntimeRef<V> {
    runtime: Arc<SpmcSignalRuntime<V>>,
}

impl<V> Clone for SpmcSignalRuntimeRef<V> {
    fn clone(&self) -> Self { 
        SpmcSignalRuntimeRef { runtime: self.runtime.clone() }
    }
}

/// Runtime for multi-producer, multi-consumer signals.
struct SpmcSignalRuntime<V> {
    value: Mutex<Option<B>>,
    last_value: Mutex<Option<B>>,
    last_value_updated: Mutex<bool>,
    await_works: TreiberStack<Box<ContinuationPl<()>>>,
    present_works: TreiberStack<Box<ContinuationPl<()>>>,
}

impl<V> SpmcSignalRuntime<V> where B: Clone {
    /// Returns a new instance of SignalRuntime.
    fn new<A>(default: B, gather: F) -> Self where F: FnMut(A, &mut B) {
        SpmcSignalRuntime {
            value: Mutex::new(None),
            last_value: Mutex::new(None),
            last_value_updated: Mutex::new(false),
            await_works: TreiberStack::new(),
            present_works: TreiberStack::new(),
        }
    }
}

impl<V> SignalRuntimeRefBase<ParallelRuntime> for SpmcSignalRuntimeRef<V>
    where B: Clone + 'static, F: 'static
{
    /// Returns a bool to indicate if the signal was emitted or not on the current instant.
    fn is_emitted(&self) -> bool {
        self.runtime.value.lock().unwrap().is_some()
    }

    /// Resets the signal at the beginning of each instant.
    fn reset(&mut self) {
        *self.runtime.value.lock().unwrap() = false;
        *self.runtime.value.lock().unwrap() = self.runtime.default_value.clone();
    }

    /// Exececutes all the continuations found in the vector `self.present_works`.
    fn execute_present_works(&mut self, runtime: &mut ParallelRuntime) {
        while let Some(c) = self.runtime.present_works.try_pop() {
            // If something is to be executed, the work is only to register a
            // task on current or next instant. It doen't mean to take a long time
            // so it's not primary to parallize here.
            // And for some technical problem we shouldn't use `on_current_instant` here.
            c.call_box(runtime, ());
        }
    }
}

impl<V> SignalRuntimeRefPl for SpmcSignalRuntimeRef<V>
    where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static
{
    /// Calls `c` at the first cycle where the signal is present.
    fn on_signal<C>(&mut self, runtime: &mut ParallelRuntime, c: C)
        where C: ContinuationPl<()>
    {
        // Important: the mutex must be unlocked after the task is added
        // in the stack if this is the case. Similar for `on_signal_present`.
        let emitted_guard = self.runtime.emitted.lock().unwrap();
        if *emitted_guard {
            drop(emitted_guard);
            c.call(runtime, ());
        } else {
            runtime.incr_await_counter();
            self.runtime.await_works.push(Box::new(c));
        }
    }
    
    /// Calls `c` only if the signal is present during this cycle.
    fn on_signal_present<C>(&mut self, runtime: &mut ParallelRuntime, c: C)
        where C: ContinuationPl<()>
    {
        let emitted_guard = self.runtime.emitted.lock().unwrap();
        if *emitted_guard {
            drop(emitted_guard);
            c.call(runtime, ());
        } else {
            self.runtime.present_works.push(Box::new(c));
        }
    }
}

impl<A, V> CanEmit<ParallelRuntime, A> for SpmcSignalRuntimeRef<V>
    where A: Send + Sync + 'static,
          B: Clone + Send + Sync + 'static,
          F: FnMut(A, &mut B) + Send + Sync + 'static
{
    fn emit(&mut self, runtime: &mut ParallelRuntime, emitted: A) {
        *self.runtime.emitted.lock().unwrap() = true;
        {
            let mut v = self.runtime.value.lock().unwrap();
            let gather = &mut *self.runtime.gather.lock().unwrap();
            gather(emitted, &mut v);
        }
        while let Some(c) = self.runtime.await_works.try_pop() {
            runtime.decr_await_counter();
            runtime.on_current_instant(c);
        }
        self.execute_present_works(runtime);
        runtime.emit_signal(Box::new(self.clone()));
        let signal_ref = self.clone();
        let update_last_value = move |_: &mut ParallelRuntime, ()| {
            let mut updated = signal_ref.runtime.last_value_updated.lock().unwrap();
            if !*updated {
                *updated = true;
                drop(updated);
                *signal_ref.runtime.last_value.lock().unwrap() = Some(signal_ref.get_value());
            }
        };
        runtime.on_end_of_instant(Box::new(update_last_value));
    }
}

impl<V> SpmcSignalRuntimeRef<V>
    where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static
{
    /// Returns a new instance of SignalRuntimeRef.
    fn new<A>(default: B, gather: F) -> Self where F: FnMut(A, &mut B) {
        SpmcSignalRuntimeRef {
            runtime: Arc::new(SpmcSignalRuntime::new(default, gather)),
        }
    }

    /// Returns the value of the signal for the current instant.
    /// The returned value is cloned and can thus be used directly.
    fn get_value(&self) -> B {
        self.runtime.value.lock().unwrap().clone()
    }
}

/// Interface of mpmc signal. This is what is directly exposed to users.
pub struct SpmcSignal<V>(SpmcSignalRuntimeRef<V>);

impl<V> Clone for SpmcSignal<V> {
    fn clone(&self) -> Self {
        SpmcSignal(self.0.clone())
    }
}

impl<V> Signal for SpmcSignal<V>
    where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static
{
    type RuntimeRef = SpmcSignalRuntimeRef<V>;
    
    fn runtime(&self) -> SpmcSignalRuntimeRef<V> {
        self.0.clone()
    }
}

impl<V> ValuedSignal for SpmcSignal<V>
    where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static
{
    type Stored = B;

    fn last_value(&self) -> Option<B> {
        let r = self.runtime();
        let last_v = r.runtime.last_value.lock().unwrap();
        last_v.clone()
    }
}

impl <V> SpmcSignal<V> where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static {
    /// Creates a new mpmc signal.
    pub fn new<A>(default: B, gather: F) -> Self
        where A: Send + Sync + 'static, F: FnMut(A, &mut B)
    {
        SpmcSignal(SpmcSignalRuntimeRef::new(default, gather))
    }
}

/* Await */

impl<V> ConstraintOnValue for Await<SpmcSignal<V>> where B: Send + Sync {
    type T = B;
}

impl<V> ProcessPl for Await<SpmcSignal<V>>
    where B: Clone + Send + Sync + 'static, F: 'static + Send + Sync
{
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        let signal_runtime = self.0.runtime();
        let eoi_continuation = move |r: &mut ParallelRuntime, ()| {
            let stored = signal_runtime.get_value();
            r.on_next_instant(Box::new(|r: &mut ParallelRuntime, ()| next.call(r, stored)));
        };
        self.0.runtime().on_signal(
            runtime,
            |r: &mut ParallelRuntime, ()| r.on_end_of_instant(Box::new(eoi_continuation)));
    }
}

impl<V> ProcessMutPl for Await<SpmcSignal<V>>
    where B: Clone + Send + Sync + 'static, F: 'static + Send + Sync
{
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let signal_runtime = self.0.runtime();
        let mut signal_runtime2 = self.0.runtime();
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

use std::sync::{Arc, Mutex};
use crossbeam::sync::TreiberStack;

use runtime::ParallelRuntime;
use continuation::ContinuationPl;
use signal::Signal;
use signal::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRefPl};
use signal::valued_signal::{ValuedSignal, MpSignal, CanEmit, GetValue};

/// A shared pointer to a signal runtime.
pub struct MpmcSignalRuntimeRef<B, F> {
    runtime: Arc<MpmcSignalRuntime<B, F>>,
}

impl<B, F> Clone for MpmcSignalRuntimeRef<B, F> {
    fn clone(&self) -> Self { 
        MpmcSignalRuntimeRef { runtime: self.runtime.clone() }
    }
}

/// Runtime for multi-producer, multi-consumer signals.
struct MpmcSignalRuntime<B, F> {
    emitted: Mutex<bool>,
    default_value: B,
    gather: Mutex<F>,
    value: Mutex<B>,
    last_value: Mutex<B>,
    last_value_updated: Mutex<bool>,
    await_works: TreiberStack<Box<ContinuationPl<()>>>,
    present_works: TreiberStack<Box<ContinuationPl<()>>>,
}

impl<B, F> MpmcSignalRuntime<B, F> where B: Clone {
    /// Returns a new instance of SignalRuntime.
    fn new<A>(default: B, gather: F) -> Self where F: FnMut(A, &mut B) {
        MpmcSignalRuntime {
            emitted: Mutex::new(false),
            default_value: default.clone(),
            gather: Mutex::new(gather),
            value: Mutex::new(default.clone()),
            last_value: Mutex::new(default),
            last_value_updated: Mutex::new(false),
            await_works: TreiberStack::new(),
            present_works: TreiberStack::new(),
        }
    }
}

impl<B, F> SignalRuntimeRefBase<ParallelRuntime> for MpmcSignalRuntimeRef<B, F>
    where B: Clone + 'static, F: 'static
{
    /// Returns a bool to indicate if the signal was emitted or not on the current instant.
    fn is_emitted(&self) -> bool {
        *self.runtime.emitted.lock().unwrap()
    }

    /// Resets the signal at the beginning of each instant.
    fn reset(&mut self) {
        let mut is_emitted = self.runtime.emitted.lock().unwrap();
        if *is_emitted {
            *is_emitted = false;
            drop(is_emitted);
            *self.runtime.value.lock().unwrap() = self.runtime.default_value.clone();
            *self.runtime.last_value_updated.lock().unwrap() = false;
        }
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

impl<B, F> SignalRuntimeRefPl for MpmcSignalRuntimeRef<B, F>
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

impl<A, B, F> CanEmit<ParallelRuntime, A> for MpmcSignalRuntimeRef<B, F>
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
                *signal_ref.runtime.last_value.lock().unwrap() = signal_ref.get_value();
            }
        };
        runtime.on_end_of_instant(Box::new(update_last_value));
    }
}

impl<B, F> GetValue<B> for MpmcSignalRuntimeRef<B, F> where B: Clone {
    /// Returns the value of the signal for the current instant.
    /// The returned value is cloned and can thus be used directly.
    fn get_value(&self) -> B {
        self.runtime.value.lock().unwrap().clone()
    }
}

impl<B, F> MpmcSignalRuntimeRef<B, F>
    where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static
{
    /// Returns a new instance of SignalRuntimeRef.
    fn new<A>(default: B, gather: F) -> Self where F: FnMut(A, &mut B) {
        MpmcSignalRuntimeRef {
            runtime: Arc::new(MpmcSignalRuntime::new(default, gather)),
        }
    }
}

/// Interface of mpmc signal. This is what is directly exposed to users.
pub struct MpmcSignalPl<B, F>(MpmcSignalRuntimeRef<B, F>);

impl<B, F> Clone for MpmcSignalPl<B, F> {
    fn clone(&self) -> Self {
        MpmcSignalPl(self.0.clone())
    }
}

impl<B, F> Signal for MpmcSignalPl<B, F>
    where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static
{
    type RuntimeRef = MpmcSignalRuntimeRef<B, F>;
    
    fn runtime(&self) -> MpmcSignalRuntimeRef<B, F> {
        self.0.clone()
    }
}

impl<B, F> ValuedSignal for MpmcSignalPl<B, F>
    where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static
{
    type Stored = B;
    type SigType = MpSignal;
}

impl<B, F> MpmcSignalPl<B, F> where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static {
    /// Creates a new mpmc signal.
    pub fn new<A>(default: B, gather: F) -> Self
        where A: Send + Sync + 'static, F: FnMut(A, &mut B)
    {
        MpmcSignalPl(MpmcSignalRuntimeRef::new(default, gather))
    }
    
    /// Returns the last value associated to the signal when it was emitted.
    /// Evaluates to the the default value before the first emission.
    pub fn last_value(&self) -> B {
        let r = self.runtime();
        let last_v = r.runtime.last_value.lock().unwrap();
        last_v.clone()
    }
}

impl MpmcSignalPl<(), ()> {
    /// Creates a new mpmc signal with the default combination function, which simply
    /// collects all emitted values in a vector.
    pub fn default<A>() -> MpmcSignalPl<Vec<A>, fn(A, &mut Vec<A>)>
        where A: Clone + Send + Sync + 'static
    {
        fn gather<A>(x: A, xs: &mut Vec<A>) {
            xs.push(x);
        }
        MpmcSignalPl::new(Vec::new(), gather)
    }
}

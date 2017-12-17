use std::sync::{Arc, Mutex};
use crossbeam::sync::TreiberStack;

use runtime::ParallelRuntime;
use continuation::ContinuationPl;
use signal::Signal;
use signal::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRefPl};
use signal::valued_signal::{ValuedSignal, MpSignal, CanEmit, GetValue};

/// A shared pointer to a signal runtime.
pub struct MpscSignalRuntimeRef<B, D, F> {
    runtime: Arc<MpscSignalRuntime<B, D, F>>,
}

impl<B, D, F> Clone for MpscSignalRuntimeRef<B, D, F> {
    fn clone(&self) -> Self { 
        MpscSignalRuntimeRef { runtime: self.runtime.clone() }
    }
}

/// Runtime for multi-producer, single-consumer signals.  
/// Since we're not always able to clone the default value, we need a function
/// that produces the default value each time when it's called.
struct MpscSignalRuntime<B, D, F> {
    emitted: Mutex<bool>,
    get_default: D,
    gather: Mutex<F>,
    value: Mutex<Option<B>>,
    await_works: TreiberStack<Box<ContinuationPl<()>>>,
    present_works: TreiberStack<Box<ContinuationPl<()>>>,
}

impl<B, D, F> MpscSignalRuntime<B, D, F> where D: Fn() -> B {
    /// Returns a new instance of SignalRuntime.
    fn new<A>(get_default: D, gather: F) -> Self where F: FnMut(A, &mut B) {
        MpscSignalRuntime {
            emitted: Mutex::new(false),
            value: Mutex::new(Some(get_default())),
            get_default: get_default,
            gather: Mutex::new(gather),
            await_works: TreiberStack::new(),
            present_works: TreiberStack::new(),
        }
    }
}

impl<B, D, F> SignalRuntimeRefBase<ParallelRuntime> for MpscSignalRuntimeRef<B, D, F>
    where B: 'static, D: Fn() -> B + 'static, F: 'static
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
            *self.runtime.value.lock().unwrap() = Some((self.runtime.get_default)());
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


impl<B, D, F> SignalRuntimeRefPl for MpscSignalRuntimeRef<B, D, F>
    where B: Send + Sync + 'static,
          D: Fn() -> B + Send + Sync + 'static,
          F: Send + Sync + 'static,
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

impl<A, B, D, F> CanEmit<ParallelRuntime, A> for MpscSignalRuntimeRef<B, D, F>
    where A: 'static,
          B: Send + Sync + 'static,
          D: Fn() -> B + Send + Sync + 'static,
          F: FnMut(A, &mut B) + Send + Sync + 'static,
{
    fn emit(&mut self, runtime: &mut ParallelRuntime, emitted: A) {
        *self.runtime.emitted.lock().unwrap() = true;
        {
            let gather = &mut *self.runtime.gather.lock().unwrap();
            match self.runtime.value.lock().unwrap().as_mut() {
                Some(v) => gather(emitted, v),
                None => assert!(false),
            }
        }
        while let Some(c) = self.runtime.await_works.try_pop() {
            runtime.decr_await_counter();
            runtime.on_current_instant(c);
        }
        self.execute_present_works(runtime);
        runtime.emit_signal(Box::new(self.clone()));
    }
}

impl<B, D, F> GetValue<B> for MpscSignalRuntimeRef<B, D, F> {
    /// Returns the value of the signal for the current instant.
    /// This function can only be called once at each instant.
    fn get_value(&self) -> B {
        self.runtime.value.lock().unwrap().take().expect(
            "Trying to get the value of a mpsc signal more than once inside an instant.")
    }
}

impl<B, D, F> MpscSignalRuntimeRef<B, D, F>
    where B: Send + Sync + 'static,
          D: Fn() -> B + Send + Sync + 'static,
          F: Send + Sync + 'static,
{
    /// Returns a new instance of SignalRuntimeRef.
    fn new<A>(get_default: D, gather: F) -> Self where F: FnMut(A, &mut B) {
        MpscSignalRuntimeRef {
            runtime: Arc::new(MpscSignalRuntime::new(get_default, gather)),
        }
    }
}

/// A parallel multi-producer, single-consumer signal.
pub struct MpscSignalPl<B, D, F>(MpscSignalRuntimeRef<B, D, F>);

impl<B, D, F> Clone for MpscSignalPl<B, D, F> {
    fn clone(&self) -> Self {
        MpscSignalPl(self.0.clone())
    }
}

impl<B, D, F> Signal for MpscSignalPl<B, D, F>
    where B: Send + Sync + 'static,
          D: Fn() -> B + Send + Sync + 'static,
          F: Send + Sync + 'static,
{
    type RuntimeRef = MpscSignalRuntimeRef<B, D, F>;
    
    fn runtime(&self) -> MpscSignalRuntimeRef<B, D, F> {
        self.0.clone()
    }
}

impl<B, D, F> ValuedSignal for MpscSignalPl<B, D, F>
    where B: Send + Sync + 'static,
          D: Fn() -> B + Send + Sync + 'static,
          F: Send + Sync + 'static,
{
    type Stored = B;
    type SigType = MpSignal;
}
    
impl<B, D, F> MpscSignalPl<B, D, F>
    where B: Send + Sync + 'static,
          D: Fn() -> B + Send + Sync + 'static,
          F: Send + Sync + 'static,
{
    /// Creates a new mpmc signal.
    pub fn new<A>(get_default: D, gather: F) -> Self where A: 'static, F: FnMut(A, &mut B) {
        MpscSignalPl(MpscSignalRuntimeRef::new(get_default, gather))
    }
}

impl MpscSignalPl<(), (), ()> {
    /// Creates a new mpsc signal with the default combination function, which simply
    /// collects all emitted values in a vector.
    pub fn default<A>() -> MpscSignalPl<Vec<A>, fn() -> Vec<A>, fn(A, &mut Vec<A>)>
        where A: Send + Sync + 'static
    {
        fn gather<A>(x: A, xs: &mut Vec<A>) {
            xs.push(x);
        }
        MpscSignalPl::new(Vec::new, gather)
    }
}

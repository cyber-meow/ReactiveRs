use std::sync::{Arc, Mutex};
use crossbeam::sync::TreiberStack;

use parallel::{Runtime, Continuation};
use parallel::process::{Process, ProcessMut};
use parallel::signal::Signal;
use parallel::signal::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRef};

/// A shared pointer to a signal runtime.
pub struct MpmcSignalRuntimeRef<B, F> {
    runtime: Arc<MpmcSignalRuntime<B, F>>,
}

impl<B, F> Clone for MpmcSignalRuntimeRef<B, F> {
    fn clone(&self) -> Self { 
        MpmcSignalRuntimeRef { runtime: self.runtime.clone() }
    }
}

/// Runtime for multi-produce, multi-consumer signals.
struct MpmcSignalRuntime<B, F> {
    emitted: Mutex<bool>,
    default_value: B,
    gather: Mutex<F>,
    value: Mutex<B>,
    last_value: Mutex<B>,
    await_works: TreiberStack<Box<Continuation<()>>>,
    present_works: TreiberStack<Box<Continuation<()>>>,
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
            await_works: TreiberStack::new(),
            present_works: TreiberStack::new(),
        }
    }
}

impl<B, F> SignalRuntimeRefBase for MpmcSignalRuntimeRef<B, F>
    where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static
{
    /// Returns a bool to indicate if the signal was emitted or not on the current instant.
    fn is_emitted(&self) -> bool {
        *self.runtime.emitted.lock().unwrap()
    }

    /// Resets the signal at the beginning of each instant.
    fn reset(&mut self) {
        *self.runtime.emitted.lock().unwrap() = false;
        *self.runtime.value.lock().unwrap() = self.runtime.default_value.clone();
    }

    /// Exececutes all the continuations found in the vector `self.present_works`.
    fn execute_present_works(&mut self, runtime: &mut Runtime) {
        while let Some(c) = self.runtime.present_works.try_pop() {
            // If something is to be executed, the work is only to register a
            // task on current or next instant. It doen't mean to take a long time
            // so it's not primary to parallize here.
            // And for some technical problem we shouldn't use `on_current_instant` here.
            c.call_box(runtime, ());
        }
    }
}


impl<B, F> SignalRuntimeRef for MpmcSignalRuntimeRef<B, F>
    where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static
{
    /// Calls `c` at the first cycle where the signal is present.
    fn on_signal<C>(&mut self, runtime: &mut Runtime, c: C) where C: Continuation<()> {
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
    fn on_signal_present<C>(&mut self, runtime: &mut Runtime, c: C)
        where C: Continuation<()>
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

impl<B, F> MpmcSignalRuntimeRef<B, F>
    where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static
{
    /// Returns a new instance of SignalRuntimeRef.
    fn new<A>(default: B, gather: F) -> Self where F: FnMut(A, &mut B) {
        MpmcSignalRuntimeRef {
            runtime: Arc::new(MpmcSignalRuntime::new(default, gather)),
        }
    }

    /// Emits the value `emitted` to the signal.
    fn emit<A>(&mut self, runtime: &mut Runtime, emitted: A) where F: FnMut(A, &mut B) {
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
        let update_last_value = move |_: &mut Runtime, ()| {
            *signal_ref.runtime.last_value.lock().unwrap() = signal_ref.get_value();
        };
        runtime.on_end_of_instant(Box::new(update_last_value));
    }

    /// Returns the value of the signal for the current instant.
    /// The returned value is cloned and can thus be used directly.
    fn get_value(&self) -> B {
        self.runtime.value.lock().unwrap().clone()
    }
}

/// Interface of mpmc signal. This is what user is directly exposed to.
pub struct MpmcSignal<B, F>(MpmcSignalRuntimeRef<B, F>);

impl<B, F> Clone for MpmcSignal<B, F> {
    fn clone(&self) -> Self {
        MpmcSignal(self.0.clone())
    }
}

impl<B, F> Signal for MpmcSignal<B, F>
    where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static
{
    type RuntimeRef = MpmcSignalRuntimeRef<B, F>;
    
    fn runtime(&self) -> MpmcSignalRuntimeRef<B, F> {
        self.0.clone()
    }
}

impl<B, F> MpmcSignal<B, F>
    where B: Clone + Send + Sync + 'static, F: Send + Sync + 'static
{
    /// Creates a new mpmc signal.
    pub fn new<A>(default: B, gather: F) -> Self
        where A: Send + Sync + 'static, F: FnMut(A, &mut B)
    {
        MpmcSignal(MpmcSignalRuntimeRef::new(default, gather))
    }

    /// Returns a process that emits the signal with value `emitted` when it is called.
    pub fn emit<A>(&self, emitted: A) -> Emit<A, B, F>
        where Self: Sized, A: Send + Sync + 'static, F: FnMut(A, &mut B)
    {
        Emit { signal: self.clone(), emitted }
    }

    /// Waits the  signal to be emitted, gets its content and
    /// terminates at the following instant.
    pub fn await(&self) -> Await<B, F> where Self: Sized {
        Await(self.clone())
    }

    /// Returns the last value associated to the signal when it was emitted.
    /// Evaluates to the default value before the first emission.
    pub fn last_value(&self) -> B {
        let r = self.runtime();
        let last_v = r.runtime.last_value.lock().unwrap();
        last_v.clone()
    }
}

pub struct Emit<A, B, F> {
    signal: MpmcSignal<B, F>,
    emitted: A,
}

impl<A, B, F> Process for Emit<A, B, F>
    where A: Send + Sync + 'static,
          B: Clone + Send + Sync + 'static,
          F: FnMut(A, &mut B) + Send + Sync + 'static,
{
    type Value = ();

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.signal.runtime().emit(runtime, self.emitted);
        next.call(runtime, ());
    }
}

impl<A, B, F> ProcessMut for Emit<A, B, F>
    where A: Clone + Send + Sync + 'static,
          B: Clone + Send + Sync + 'static,
          F: FnMut(A, &mut B) + Send + Sync + 'static
{
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        self.signal.runtime().emit(runtime, self.emitted.clone());
        next.call(runtime, (self, ()));
    }
}

pub struct Await<B, F>(MpmcSignal<B, F>);

impl<B, F> Process for Await<B, F>
    where B: Clone + Send + Sync + 'static, F: 'static + Send + Sync
{
    type Value = B;
    
    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let signal_runtime = self.0.runtime();
        let eoi_continuation = move |r: &mut Runtime, ()| {
            let stored = signal_runtime.get_value();
            r.on_next_instant(Box::new(|r: &mut Runtime, ()| next.call(r, stored)));
        };
        self.0.runtime().on_signal(
            runtime,
            |r: &mut Runtime, ()| r.on_end_of_instant(Box::new(eoi_continuation)));
    }
}

impl <B, F> ProcessMut for Await<B, F>
    where B: Clone + Send + Sync + 'static, F: 'static + Send + Sync
{
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        let signal_runtime = self.0.runtime();
        let mut signal_runtime2 = self.0.runtime();
        let eoi_continuation = move |r: &mut Runtime, ()| {
            let stored = signal_runtime.get_value();
            r.on_next_instant(Box::new(|r: &mut Runtime, ()| next.call(r, (self, stored))));
        };
        signal_runtime2.on_signal(
            runtime,
            |r: &mut Runtime, ()| r.on_end_of_instant(Box::new(eoi_continuation)));
    }
}

use std::sync::{Arc, Mutex};
use crossbeam::sync::TreiberStack;

use parallel::{Runtime, Continuation};
use parallel::process::{Process, ProcessMut};
use parallel::signal::Signal;
use parallel::signal::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRef};

/// A shared pointer to a signal runtime.
#[derive(Clone)]
pub struct PureSignalRuntimeRef {
    runtime: Arc<PureSignalRuntime>,
}

/// Runtime for pure signals.
struct PureSignalRuntime {
    emitted: Mutex<bool>,
    await_works: TreiberStack<Box<Continuation<()>>>,
    present_works: TreiberStack<Box<Continuation<()>>>,
}

impl PureSignalRuntime {
    /// Returns a new instance of SignalRuntime.
    fn new() -> Self {
        PureSignalRuntime {
            emitted: Mutex::new(false),
            await_works: TreiberStack::new(),
            present_works: TreiberStack::new(),
        }
    }
}

impl SignalRuntimeRefBase for PureSignalRuntimeRef {
    /// Returns a bool to indicate if the signal was emitted or not on the current instant.
    fn is_emitted(&self) -> bool {
        *self.runtime.emitted.lock().unwrap()
    }

    /// Resets the signal at the beginning of each instant.
    fn reset(&mut self) {
        *self.runtime.emitted.lock().unwrap() = false;
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


impl SignalRuntimeRef for PureSignalRuntimeRef {
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
            // Without explicit unlock we get some deadlock here.
            drop(emitted_guard);
            c.call(runtime, ());
        } else {
            self.runtime.present_works.push(Box::new(c));
        }
    }
}

impl PureSignalRuntimeRef {
    /// Returns a new instance of SignalRuntimeRef.
    fn new() -> Self {
        PureSignalRuntimeRef { runtime: Arc::new(PureSignalRuntime::new()) }
    }

    /// Sets the signal as emitted for the current instant.
    fn emit(&mut self, runtime: &mut Runtime) {
        *self.runtime.emitted.lock().unwrap() = true;
        while let Some(c) = self.runtime.await_works.try_pop() {
            runtime.decr_await_counter();
            runtime.on_current_instant(c);
        }
        self.execute_present_works(runtime);
        runtime.emit_signal(Box::new(self.clone()));
    }
}

/// Interface of pure signal, to be used by the user.
#[derive(Clone)]
pub struct PureSignal(PureSignalRuntimeRef);

impl Signal for PureSignal {
    type RuntimeRef = PureSignalRuntimeRef;
    
    fn runtime(&self) -> PureSignalRuntimeRef {
        self.0.clone()
    }
}

impl PureSignal {
    /// Creates a new pure signal.
    pub fn new() -> Self {
        PureSignal(PureSignalRuntimeRef::new())
    }
    
    /// Returns a process that emits the signal when it is called.
    pub fn emit(&self) -> Emit where Self: Sized {
        Emit(self.clone())
    }
}

pub struct Emit(PureSignal);

impl Process for Emit {
    type Value = ();

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.0.runtime().emit(runtime);
        next.call(runtime, ());
    }
}

impl ProcessMut for Emit {
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        self.0.runtime().emit(runtime);
        next.call(runtime, (self, ()));
    }
}

use std::rc::Rc;
use std::cell::RefCell;

use {Runtime, Continuation};
use process::{Process, ProcessMut};
use signal::Signal;
use signal::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRef};

/// A shared pointer to a signal runtime.
#[derive(Clone)]
pub struct PureSignalRuntimeRef {
    runtime: Rc<PureSignalRuntime>,
}

/// Runtime for pure signals.
struct PureSignalRuntime {
    emitted: RefCell<bool>,
    await_works: RefCell<Vec<Box<Continuation<()>>>>,
    present_works: RefCell<Vec<Box<Continuation<()>>>>,
}

impl PureSignalRuntime {
    /// Returns a new instance of SignalRuntime.
    fn new() -> Self {
        PureSignalRuntime {
            emitted: RefCell::new(false),
            await_works: RefCell::new(Vec::new()),
            present_works: RefCell::new(Vec::new()),
        }
    }
}

impl SignalRuntimeRefBase for PureSignalRuntimeRef {
    /// Returns a bool to indicate if the signal was emitted or not on the current instant.
    fn is_emitted(&self) -> bool {
        *self.runtime.emitted.borrow()
    }

    /// Resets the signal at the beginning of each instant.
    fn reset(&mut self) {
        *self.runtime.emitted.borrow_mut() = false;
    }

    /// Exececutes all the continuations found in the vector `self.present_works`.
    fn execute_present_works(&mut self, runtime: &mut Runtime) {
        while let Some(c) = self.runtime.present_works.borrow_mut().pop() {
            c.call_box(runtime, ());
        }
    }
}


impl SignalRuntimeRef for PureSignalRuntimeRef {
    /// Calls `c` at the first cycle where the signal is present.
    fn on_signal<C>(&mut self, runtime: &mut Runtime, c: C) where C: Continuation<()> {
        if *self.runtime.emitted.borrow() {
            c.call(runtime, ());
        } else {
            runtime.incr_await_counter();
            self.runtime.await_works.borrow_mut().push(Box::new(c));
        }
    }
    
    /// Calls `c` only if the signal is present during this cycle.
    fn on_signal_present<C>(&mut self, runtime: &mut Runtime, c: C)
        where C: Continuation<()>
    {
        if *self.runtime.emitted.borrow() {
            c.call(runtime, ());
        } else {
            self.runtime.present_works.borrow_mut().push(Box::new(c));
        }
    }
}

impl PureSignalRuntimeRef {
    /// Returns a new instance of SignalRuntimeRef.
    fn new() -> Self {
        PureSignalRuntimeRef { runtime: Rc::new(PureSignalRuntime::new()) }
    }

    /// Sets the signal as emitted for the current instant.
    fn emit(&mut self, runtime: &mut Runtime) {
        *self.runtime.emitted.borrow_mut() = true;
        while let Some(c) = self.runtime.await_works.borrow_mut().pop() {
            runtime.decr_await_counter();
            c.call_box(runtime, ());
        }
        self.execute_present_works(runtime);
        runtime.emit_signal(self.clone());
    }
}

#[derive(Clone)]
pub struct PureSignal(PureSignalRuntimeRef);

impl Signal for PureSignal {
    type RuntimeRef = PureSignalRuntimeRef;
    
    fn runtime(&mut self) -> PureSignalRuntimeRef {
        self.0.clone()
    }
}

impl PureSignal {
    /// Creates a new pure signal.
    pub fn new() -> Self {
        PureSignal(PureSignalRuntimeRef::new())
    }
    
    /// Returns a process that emits the signal when it is called.
    pub fn emit(&mut self) -> Emit<Self> where Self: Sized {
        Emit(self.clone())
    }
}

pub struct Emit<S>(S);

impl Process for Emit<PureSignal> {
    type Value = ();

    fn call<C>(mut self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.0.runtime().emit(runtime);
        next.call(runtime, ());
    }
}

impl ProcessMut for Emit<PureSignal> {
    fn call_mut<C>(mut self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        self.0.runtime().emit(runtime);
        next.call(runtime, (self, ()));
    }
}

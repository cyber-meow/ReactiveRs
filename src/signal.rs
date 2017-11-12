use std::rc::Rc;
use std::cell::RefCell;

use {Runtime, Continuation};
use process::{Process, ProcessMut};

/// A shared pointer to a signal runtime.
#[derive(Clone)]
pub struct SignalRuntimeRef {
    runtime: Rc<SignalRuntime>,
}

/// Runtime for pure signals.
struct SignalRuntime {
    emitted: RefCell<bool>,
    await_works: RefCell<Vec<Box<Continuation<()>>>>,
}

impl SignalRuntime {
    /// Returns a new instance of SignalRuntime.
    fn new() -> Self {
        SignalRuntime {
            emitted: RefCell::new(false),
            await_works: RefCell::new(Vec::new()),
        }
    }
}

impl SignalRuntimeRef {
    /// Returns a new instance of SignalRuntimeRef.
    fn new() -> Self {
        SignalRuntimeRef { runtime: Rc::new(SignalRuntime::new()) }
    }

    /// Sets the signal as emitted for the current instant.
    fn emit(self, runtime: &mut Runtime) {
        *self.runtime.emitted.borrow_mut() = true;
        while let Some(c) = self.runtime.await_works.borrow_mut().pop() {
            c.call_box(runtime, ());
        }
        runtime.emit_signal(self);
    }

    /// Resets the signal at the beginning of each instant.
    pub(crate) fn reset(self) {
        *self.runtime.emitted.borrow_mut() = false;
    }

    /// Calls `c` at the first cycle where the signal is present.
    fn on_signal<C>(self, runtime: &mut Runtime, c: C) where C: Continuation<()> {
        if *self.runtime.emitted.borrow() {
            c.call(runtime, ());
        } else {
            self.runtime.await_works.borrow_mut().push(Box::new(c));
        }
    }

    // TODO: add other methods when needed.
}

/// A reactive signal.
pub trait Signal: 'static {
    /// Returns a reference to the signal's runtime.
    fn runtime(&mut self) -> SignalRuntimeRef;

    /// Returns a process that emits the signal when it is called.
    fn emit(self) -> Emit<Self> where Self: Sized {
        Emit(self)
    }

    /// Returns a process that waits for the next emission of the signal, current instant
    /// included.
    fn await_immediate(self) -> AwaitImmediate<Self> where Self: Sized {
        AwaitImmediate(self)
    }

    // TODO: add other methods if needed.
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
        let singal_runtime_ref = self.0.runtime().clone();
        singal_runtime_ref.on_signal(
            runtime,
            next.map(|_| (self, ()))
        );
    }
}

pub struct Emit<S>(S);

impl<S> Process for Emit<S> where S: Signal {
    type Value = ();

    fn call<C>(mut self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.0.runtime().emit(runtime);
        next.call(runtime, ());
    }
}

impl<S> ProcessMut for Emit<S> where S: Signal {
    fn call_mut<C>(mut self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        self.0.runtime().emit(runtime);
        next.call(runtime, (self, ()));
    }
}

pub struct PureSignal(SignalRuntimeRef);

impl PureSignal {
    /// Creates a new pure signal.
    pub fn new() -> Self {
        PureSignal(SignalRuntimeRef::new())
    }
}

impl Signal for PureSignal {
    fn runtime(&mut self) -> SignalRuntimeRef {
        self.0.clone()
    }
}

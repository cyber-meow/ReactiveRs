use std::rc::Rc;
use std::cell::RefCell;

use runtime::SingleThreadRuntime;
use continuation::ContinuationSt;
use process::{ProcessSt, ProcessMutSt};

use signal::Signal;
use signal::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRefSt};
use signal::pure_signal::{PureSignal, Emit};

/// A shared pointer to a signal runtime.
#[derive(Clone)]
pub struct PureSignalRuntimeRef {
    runtime: Rc<PureSignalRuntime>,
}

/// Runtime for pure signals.
struct PureSignalRuntime {
    emitted: RefCell<bool>,
    await_works: RefCell<Vec<Box<ContinuationSt<()>>>>,
    present_works: RefCell<Vec<Box<ContinuationSt<()>>>>,
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

impl SignalRuntimeRefBase<SingleThreadRuntime> for PureSignalRuntimeRef {
    /// Returns a bool to indicate if the signal was emitted or not on the current instant.
    fn is_emitted(&self) -> bool {
        *self.runtime.emitted.borrow()
    }

    /// Resets the signal at the beginning of each instant.
    fn reset(&mut self) {
        *self.runtime.emitted.borrow_mut() = false;
    }

    /// Exececutes all the continuations found in the vector `self.present_works`.
    fn execute_present_works(&mut self, runtime: &mut SingleThreadRuntime) {
        while let Some(c) = self.runtime.present_works.borrow_mut().pop() {
            c.call_box(runtime, ());
        }
    }
}


impl SignalRuntimeRefSt for PureSignalRuntimeRef {
    /// Calls `c` at the first cycle where the signal is present.
    fn on_signal<C>(&mut self, runtime: &mut SingleThreadRuntime, c: C)
        where C: ContinuationSt<()>
    {
        if *self.runtime.emitted.borrow() {
            c.call(runtime, ());
        } else {
            runtime.incr_await_counter();
            self.runtime.await_works.borrow_mut().push(Box::new(c));
        }
    }
    
    /// Calls `c` only if the signal is present during this cycle.
    fn on_signal_present<C>(&mut self, runtime: &mut SingleThreadRuntime, c: C)
        where C: ContinuationSt<()>
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
    fn emit(&mut self, runtime: &mut SingleThreadRuntime) {
        *self.runtime.emitted.borrow_mut() = true;
        while let Some(c) = self.runtime.await_works.borrow_mut().pop() {
            runtime.decr_await_counter();
            c.call_box(runtime, ());
        }
        self.execute_present_works(runtime);
        runtime.emit_signal(Box::new(self.clone()));
    }
}

/// Interface of pure signal, to be used by the user.
#[derive(Clone)]
pub struct PureSignalImpl(PureSignalRuntimeRef);

impl Signal for PureSignalImpl {
    type RuntimeRef = PureSignalRuntimeRef;

    fn runtime(&self) -> PureSignalRuntimeRef {
        self.0.clone()
    }
}

impl PureSignal for PureSignalImpl {
    fn new() -> Self {
        PureSignalImpl(PureSignalRuntimeRef::new())
    }
}

/* Emit */

impl ProcessSt for Emit<PureSignalImpl> {
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        self.0.runtime().emit(runtime);
        next.call(runtime, ());
    }
}

impl ProcessMutSt for Emit<PureSignalImpl> {
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        self.0.runtime().emit(runtime);
        next.call(runtime, (self, ()));
    }
}

use std::rc::Rc;
use std::cell::RefCell;

use {Runtime, Continuation};
use process::{Process, ProcessMut};
use signal::Signal;
use signal::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRef};

/// A shared pointer to a signal runtime.
pub struct MpmcSignalRuntimeRef<B, F> {
    runtime: Rc<MpmcSignalRuntime<B, F>>,
}

impl<B, F> Clone for MpmcSignalRuntimeRef<B, F> {
    fn clone(&self) -> Self { 
        MpmcSignalRuntimeRef { runtime: self.runtime.clone() }
    }
}

/// Runtime for multi-produce, multi-consumer signals.
struct MpmcSignalRuntime<B, F> {
    emitted: RefCell<bool>,
    default_value: B,
    gather: RefCell<F>,
    value: RefCell<B>,
    await_works: RefCell<Vec<Box<Continuation<()>>>>,
    present_works: RefCell<Vec<Box<Continuation<()>>>>,
}

impl<B, F> MpmcSignalRuntime<B, F> where B: Clone {
    /// Returns a new instance of SignalRuntime.
    fn new<A>(default: B, gather: F) -> Self where F: FnMut(A, &mut B) {
        MpmcSignalRuntime {
            emitted: RefCell::new(false),
            default_value: default.clone(),
            gather: RefCell::new(gather),
            value: RefCell::new(default),
            await_works: RefCell::new(Vec::new()),
            present_works: RefCell::new(Vec::new()),
        }
    }
}

impl<B, F> SignalRuntimeRefBase for MpmcSignalRuntimeRef<B, F>
    where B: Clone + 'static, F: 'static
{
    /// Returns a bool to indicate if the signal was emitted or not on the current instant.
    fn is_emitted(&self) -> bool {
        *self.runtime.emitted.borrow()
    }

    /// Resets the signal at the beginning of each instant.
    fn reset(&mut self) {
        *self.runtime.emitted.borrow_mut() = false;
        *self.runtime.value.borrow_mut() = self.runtime.default_value.clone();
    }

    /// Exececutes all the continuations found in the vector `self.present_works`.
    fn execute_present_works(&mut self, runtime: &mut Runtime) {
        while let Some(c) = self.runtime.present_works.borrow_mut().pop() {
            c.call_box(runtime, ());
        }
    }
}


impl<B, F> SignalRuntimeRef for MpmcSignalRuntimeRef<B, F>
    where B: Clone + 'static, F: 'static
{
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

impl<B, F> MpmcSignalRuntimeRef<B, F> where B: Clone + 'static {
    /// Returns a new instance of SignalRuntimeRef.
    fn new<A>(default: B, gather: F) -> Self where F: FnMut(A, &mut B) {
        MpmcSignalRuntimeRef {
            runtime: Rc::new(MpmcSignalRuntime::new(default, gather)),
        }
    }

    /// Sets the signal as emitted for the current instant.
    fn emit<A>(&mut self, runtime: &mut Runtime, value: A) where F: FnMut(A, &mut B) + 'static {
        *self.runtime.emitted.borrow_mut() = true;
        {
            let v = &self.runtime.value;
            let gather = &mut *self.runtime.gather.borrow_mut();
            gather(value, &mut v.borrow_mut());
        }
        while let Some(c) = self.runtime.await_works.borrow_mut().pop() {
            runtime.decr_await_counter();
            c.call_box(runtime, ());
        }
        self.execute_present_works(runtime);
        runtime.emit_signal(self.clone());
    }
}

pub struct MpmcSignal<B, F>(MpmcSignalRuntimeRef<B, F>);

impl<B, F> Clone for MpmcSignal<B, F> {
    fn clone(&self) -> Self {
        MpmcSignal(self.0.clone())
    }
}

impl<B, F> Signal for MpmcSignal<B, F> where B: Clone + 'static, F: 'static {
    type RuntimeRef = MpmcSignalRuntimeRef<B, F>;
    
    fn runtime(&mut self) -> MpmcSignalRuntimeRef<B, F> {
        self.0.clone()
    }
}

/*
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
*/

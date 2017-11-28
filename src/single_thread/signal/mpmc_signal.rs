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
    last_value: RefCell<B>,
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
            value: RefCell::new(default.clone()),
            last_value: RefCell::new(default),
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

impl<B, F> MpmcSignalRuntimeRef<B, F> where B: Clone + 'static, F: 'static {
    /// Returns a new instance of SignalRuntimeRef.
    fn new<A>(default: B, gather: F) -> Self where F: FnMut(A, &mut B) {
        MpmcSignalRuntimeRef {
            runtime: Rc::new(MpmcSignalRuntime::new(default, gather)),
        }
    }

    /// Emits the value `emitted` to the signal.
    fn emit<A>(&mut self, runtime: &mut Runtime, emitted: A) where F: FnMut(A, &mut B) {
        *self.runtime.emitted.borrow_mut() = true;
        {
            let v = &self.runtime.value;
            let gather = &mut *self.runtime.gather.borrow_mut();
            gather(emitted, &mut v.borrow_mut());
        }
        while let Some(c) = self.runtime.await_works.borrow_mut().pop() {
            runtime.decr_await_counter();
            c.call_box(runtime, ());
        }
        self.execute_present_works(runtime);
        runtime.emit_signal(Box::new(self.clone()));
        let signal_ref = self.clone();
        let update_last_value = move |_: &mut Runtime, ()| {
            *signal_ref.runtime.last_value.borrow_mut() = signal_ref.get_value();
        };
        runtime.on_end_of_instant(Box::new(update_last_value));
    }

    /// Returns the value of the signal for the current instant.
    /// The returned value is cloned and can thus be used directly.
    fn get_value(&self) -> B {
        self.runtime.value.borrow().clone()
    }
}

/// Interface of mpmc signal. This is what user is directly exposed to.
pub struct MpmcSignal<B, F>(MpmcSignalRuntimeRef<B, F>);

impl<B, F> Clone for MpmcSignal<B, F> {
    fn clone(&self) -> Self {
        MpmcSignal(self.0.clone())
    }
}

impl<B, F> Signal for MpmcSignal<B, F> where B: Clone + 'static, F: 'static {
    type RuntimeRef = MpmcSignalRuntimeRef<B, F>;
    
    fn runtime(&self) -> MpmcSignalRuntimeRef<B, F> {
        self.0.clone()
    }
}

impl<B, F> MpmcSignal<B, F> where B: Clone + 'static, F: 'static {
    /// Creates a new mpmc signal.
    pub fn new<A>(default: B, gather: F) -> Self where F: FnMut(A, &mut B) {
        MpmcSignal(MpmcSignalRuntimeRef::new(default, gather))
    }

    /// Returns a process that emits the signal with value `emitted` when it is called.
    pub fn emit<A>(&self, emitted: A) -> Emit<A, B, F>
        where Self: Sized, F: FnMut(A, &mut B)
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
        let last_v = r.runtime.last_value.borrow();
        last_v.clone()
    }
}

pub struct Emit<A, B, F> {
    signal: MpmcSignal<B, F>,
    emitted: A,
}

impl<A, B, F> Process for Emit<A, B, F>
    where A: 'static, B: Clone + 'static, F: FnMut(A, &mut B) + 'static
{
    type Value = ();

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.signal.runtime().emit(runtime, self.emitted);
        next.call(runtime, ());
    }
}

impl<A, B, F> ProcessMut for Emit<A, B, F>
    where A: Clone + 'static, B: Clone + 'static, F: FnMut(A, &mut B) + 'static
{
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        self.signal.runtime().emit(runtime, self.emitted.clone());
        next.call(runtime, (self, ()));
    }
}

pub struct Await<B, F>(MpmcSignal<B, F>);

impl<B, F> Process for Await<B, F> where B: Clone + 'static, F: 'static {
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

impl <B, F> ProcessMut for Await<B, F> where B: Clone + 'static, F: 'static {
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

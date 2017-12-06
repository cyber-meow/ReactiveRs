use std::rc::Rc;
use std::cell::RefCell;

use runtime::SingleThreadRuntime;
use continuation::ContinuationSt;
use process::{ProcessSt, ProcessMutSt};

use signal::Signal;
use signal::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRefSt};
use signal::mpmc_signal::{MpmcSignal, Emit, Await};

/// A shared pointer to a signal runtime.
pub struct MpmcSignalRuntimeRef<B, F> {
    runtime: Rc<MpmcSignalRuntime<B, F>>,
}

impl<B, F> Clone for MpmcSignalRuntimeRef<B, F> {
    fn clone(&self) -> Self { 
        MpmcSignalRuntimeRef { runtime: self.runtime.clone() }
    }
}

/// Runtime for multi-producer, multi-consumer signals.
struct MpmcSignalRuntime<B, F> {
    emitted: RefCell<bool>,
    default_value: B,
    gather: RefCell<F>,
    value: RefCell<B>,
    last_value: RefCell<B>,
    last_value_updated: RefCell<bool>,
    await_works: RefCell<Vec<Box<ContinuationSt<()>>>>,
    present_works: RefCell<Vec<Box<ContinuationSt<()>>>>,
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
            last_value_updated: RefCell::new(false),
            await_works: RefCell::new(Vec::new()),
            present_works: RefCell::new(Vec::new()),
        }
    }
}

impl<B, F> SignalRuntimeRefBase<SingleThreadRuntime> for MpmcSignalRuntimeRef<B, F>
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
        *self.runtime.last_value_updated.borrow_mut() = false;
    }

    /// Exececutes all the continuations found in the vector `self.present_works`.
    fn execute_present_works(&mut self, runtime: &mut SingleThreadRuntime) {
        while let Some(c) = self.runtime.present_works.borrow_mut().pop() {
            c.call_box(runtime, ());
        }
    }
}


impl<B, F> SignalRuntimeRefSt for MpmcSignalRuntimeRef<B, F>
    where B: Clone + 'static, F: 'static
{
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

impl<B, F> MpmcSignalRuntimeRef<B, F> where B: Clone + 'static, F: 'static {
    /// Returns a new instance of SignalRuntimeRef.
    fn new<A>(default: B, gather: F) -> Self where F: FnMut(A, &mut B) {
        MpmcSignalRuntimeRef {
            runtime: Rc::new(MpmcSignalRuntime::new(default, gather)),
        }
    }

    /// Emits the value `emitted` to the signal.
    fn emit<A>(&mut self, runtime: &mut SingleThreadRuntime, emitted: A)
        where F: FnMut(A, &mut B)
    {
        *self.runtime.emitted.borrow_mut() = true;
        {
            let mut v = self.runtime.value.borrow_mut();
            let gather = &mut *self.runtime.gather.borrow_mut();
            gather(emitted, &mut v);
        }
        while let Some(c) = self.runtime.await_works.borrow_mut().pop() {
            runtime.decr_await_counter();
            c.call_box(runtime, ());
        }
        self.execute_present_works(runtime);
        runtime.emit_signal(Box::new(self.clone()));
        let signal_ref = self.clone();
        let update_last_value = move |_: &mut SingleThreadRuntime, ()| {
            if !*signal_ref.runtime.last_value_updated.borrow() {
                *signal_ref.runtime.last_value.borrow_mut() = signal_ref.get_value();
                *signal_ref.runtime.last_value_updated.borrow_mut() = true;
            }
        };
        runtime.on_end_of_instant(Box::new(update_last_value));
    }

    /// Returns the value of the signal for the current instant.
    /// The returned value is cloned and can thus be used directly.
    fn get_value(&self) -> B {
        self.runtime.value.borrow().clone()
    }
}

/// Interface of mpmc signal. This is what is directly exposed to users.
pub struct MpmcSignalImpl<B, F>(MpmcSignalRuntimeRef<B, F>);

impl<B, F> Clone for MpmcSignalImpl<B, F> {
    fn clone(&self) -> Self {
        MpmcSignalImpl(self.0.clone())
    }
}

impl<B, F> Signal for MpmcSignalImpl<B, F> where B: Clone + 'static, F: 'static {
    type RuntimeRef = MpmcSignalRuntimeRef<B, F>;
    
    fn runtime(&self) -> MpmcSignalRuntimeRef<B, F> {
        self.0.clone()
    }
}

impl<B, F> MpmcSignal for MpmcSignalImpl<B, F> where B: Clone + 'static, F: 'static {
    type Stored = B;
    type Gather = F;
    
    fn last_value(&self) -> B {
        let r = self.runtime();
        let last_v = r.runtime.last_value.borrow();
        last_v.clone()
    }
}
    
impl<B, F> MpmcSignalImpl<B, F> where B: Clone + 'static, F: 'static {
    /// Creates a new mpmc signal.
    pub fn new<A>(default: B, gather: F) -> Self where A: 'static, F: FnMut(A, &mut B) {
        MpmcSignalImpl(MpmcSignalRuntimeRef::new(default, gather))
    }
}

impl<A, B, F> ProcessSt for Emit<MpmcSignalImpl<B, F>, A>
    where A: 'static, B: Clone + 'static, F: FnMut(A, &mut B) + 'static
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        self.signal.runtime().emit(runtime, self.emitted);
        next.call(runtime, ());
    }
}

/* Emit */

impl<A, B, F> ProcessMutSt for Emit<MpmcSignalImpl<B, F>, A>
    where A: Clone + 'static, B: Clone + 'static, F: FnMut(A, &mut B) + 'static
{
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        self.signal.runtime().emit(runtime, self.emitted.clone());
        next.call(runtime, (self, ()));
    }
}

impl<B, F> ProcessSt for Await<MpmcSignalImpl<B, F>> 
    where B: Clone + 'static, F: 'static
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let signal_runtime = self.0.runtime();
        let eoi_continuation = move |r: &mut SingleThreadRuntime, ()| {
            let stored = signal_runtime.get_value();
            r.on_next_instant(
                Box::new(|r: &mut SingleThreadRuntime, ()| next.call(r, stored)));
        };
        self.0.runtime().on_signal(
            runtime,
            |r: &mut SingleThreadRuntime, ()|
                r.on_end_of_instant(Box::new(eoi_continuation)));
    }
}

/* Await */

impl <B, F> ProcessMutSt for Await<MpmcSignalImpl<B, F>>
    where B: Clone + 'static, F: 'static
{
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let signal_runtime = self.0.runtime();
        let mut signal_runtime2 = self.0.runtime();
        let eoi_continuation = move |r: &mut SingleThreadRuntime, ()| {
            let stored = signal_runtime.get_value();
            r.on_next_instant(
                Box::new(|r: &mut SingleThreadRuntime, ()| next.call(r, (self, stored))));
        };
        signal_runtime2.on_signal(
            runtime,
            |r: &mut SingleThreadRuntime, ()|
                r.on_end_of_instant(Box::new(eoi_continuation)));
    }
}

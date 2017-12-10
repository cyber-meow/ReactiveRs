use std::rc::Rc;
use std::cell::RefCell;

use runtime::SingleThreadRuntime;
use continuation::ContinuationSt;
use process::{ProcessSt, ProcessMutSt};

use signal::Signal;
use signal::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRefSt};
use signal::valued_signal::{ValuedSignal, CanEmit, Await};

/// A shared pointer to a signal runtime.
pub struct SpmcSignalRuntimeRef<V> {
    runtime: Rc<SpmcSignalRuntime<V>>,
}

impl<V> Clone for SpmcSignalRuntimeRef<V> {
    fn clone(&self) -> Self { 
        SpmcSignalRuntimeRef { runtime: self.runtime.clone() }
    }
}

/// Runtime for single-producer, multi-consumer signals.
struct SpmcSignalRuntime<V> {
    value: RefCell<Option<V>>,
    last_value: RefCell<Option<V>>,
    last_value_updated: RefCell<bool>,
    await_works: RefCell<Vec<Box<ContinuationSt<()>>>>,
    present_works: RefCell<Vec<Box<ContinuationSt<()>>>>,
}

impl<V> SpmcSignalRuntime<V> where V: Clone {
    /// Returns a new instance of SignalRuntime.
    fn new() -> Self {
        SpmcSignalRuntime {
            value: RefCell::new(None),
            last_value: RefCell::new(None),
            last_value_updated: RefCell::new(false),
            await_works: RefCell::new(Vec::new()),
            present_works: RefCell::new(Vec::new()),
        }
    }
}

impl<V> SignalRuntimeRefBase<SingleThreadRuntime> for SpmcSignalRuntimeRef<V>
    where V: Clone + 'static
{
    /// Returns a bool to indicate if the signal was emitted or not on the current instant.
    fn is_emitted(&self) -> bool {
        self.runtime.value.borrow().is_some()
    }

    /// Resets the signal at the beginning of each instant.
    fn reset(&mut self) {
        *self.runtime.value.borrow_mut() = None;
        *self.runtime.last_value_updated.borrow_mut() = false;
    }

    /// Exececutes all the continuations found in the vector `self.present_works`.
    fn execute_present_works(&mut self, runtime: &mut SingleThreadRuntime) {
        while let Some(c) = self.runtime.present_works.borrow_mut().pop() {
            c.call_box(runtime, ());
        }
    }
}


impl<V> SignalRuntimeRefSt for SpmcSignalRuntimeRef<V> where V: Clone + 'static {
    /// Calls `c` at the first cycle where the signal is present.
    fn on_signal<C>(&mut self, runtime: &mut SingleThreadRuntime, c: C)
        where C: ContinuationSt<()>
    {
        if self.is_emitted() {
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
        if self.is_emitted() {
            c.call(runtime, ());
        } else {
            self.runtime.present_works.borrow_mut().push(Box::new(c));
        }
    }
}

impl<V> CanEmit<SingleThreadRuntime, V> for SpmcSignalRuntimeRef<V> where V: Clone + 'static {
    fn emit(&mut self, runtime: &mut SingleThreadRuntime, emitted: V) {
        if self.is_emitted() {
            panic!("Multiple emissions of a single-producer signal inside an instant.");
        }
        *self.runtime.value.borrow_mut() = Some(emitted);
        while let Some(c) = self.runtime.await_works.borrow_mut().pop() {
            runtime.decr_await_counter();
            c.call_box(runtime, ());
        }
        self.execute_present_works(runtime);
        runtime.emit_signal(Box::new(self.clone()));
        let signal_ref = self.clone();
        let update_last_value = move |_: &mut SingleThreadRuntime, ()| {
            if !*signal_ref.runtime.last_value_updated.borrow() {
                *signal_ref.runtime.last_value.borrow_mut() = Some(signal_ref.get_value());
                *signal_ref.runtime.last_value_updated.borrow_mut() = true;
            }
        };
        runtime.on_end_of_instant(Box::new(update_last_value));
    }
}

impl<V> SpmcSignalRuntimeRef<V> where V: Clone + 'static {
    /// Returns a new instance of SignalRuntimeRef.
    fn new() -> Self {
        SpmcSignalRuntimeRef { runtime: Rc::new(SpmcSignalRuntime::new()) }
    }

    /// Returns the value of the signal for the current instant.
    /// The returned value is cloned and can thus be used directly.
    fn get_value(&self) -> V {
        self.runtime.value.borrow().clone().unwrap()
    }
}

/// Interface of spmc signal. This is what is directly exposed to users.
pub struct SpmcSignalSt<V>(SpmcSignalRuntimeRef<V>);

impl<V> Clone for SpmcSignalSt<V> {
    fn clone(&self) -> Self {
        SpmcSignalSt(self.0.clone())
    }
}

impl<V> Signal for SpmcSignalSt<V> where V: Clone + 'static {
    type RuntimeRef = SpmcSignalRuntimeRef<V>;
    
    fn runtime(&self) -> SpmcSignalRuntimeRef<V> {
        self.0.clone()
    }
}

impl<V> ValuedSignal for SpmcSignalSt<V> where V: Clone + 'static {
    type Stored = V;
 
    fn last_value(&self) -> Option<V> {
        let r = self.runtime();
        let last_v = r.runtime.last_value.borrow();
        last_v.clone()
    }
}

impl<V> SpmcSignalSt<V> where V: Clone + 'static {
    /// Creates a new spmc signal.
    pub fn new() -> Self {
        SpmcSignalSt(SpmcSignalRuntimeRef::new())
    }
}

/* Await */

impl<V> ProcessSt for Await<SpmcSignalSt<V>> where V: Clone + 'static {
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let signal_runtime = self.0.runtime();
        self.0.runtime().on_signal(
            runtime,
            move |r: &mut SingleThreadRuntime, ()| next.call(r, signal_runtime.get_value()));
    }
}

impl<V> ProcessMutSt for Await<SpmcSignalSt<V>> where V: Clone + 'static {
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let signal_runtime = self.0.runtime();
        let mut signal_runtime2 = self.0.runtime();
        signal_runtime2.on_signal(
            runtime,
            move |r: &mut SingleThreadRuntime, ()|
                next.call(r, (self, signal_runtime.get_value())));
    }
}

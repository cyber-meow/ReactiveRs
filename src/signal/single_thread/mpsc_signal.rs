use std::rc::Rc;
use std::cell::RefCell;

use runtime::SingleThreadRuntime;
use continuation::ContinuationSt;
use signal::Signal;
use signal::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRefSt};
use signal::valued_signal::{ValuedSignal, MpSignal, CanEmit, GetValue};

/// A shared pointer to a signal runtime.
pub struct MpscSignalRuntimeRef<B, D, F> {
    runtime: Rc<MpscSignalRuntime<B, D, F>>,
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
    emitted: RefCell<bool>,
    get_default: D,
    gather: RefCell<F>,
    value: RefCell<Option<B>>,
    await_works: RefCell<Vec<Box<ContinuationSt<()>>>>,
    present_works: RefCell<Vec<Box<ContinuationSt<()>>>>,
}

impl<B, D, F> MpscSignalRuntime<B, D, F> where D: Fn() -> B {
    /// Returns a new instance of SignalRuntime.
    fn new<A>(get_default: D, gather: F) -> Self where F: FnMut(A, &mut B) {
        MpscSignalRuntime {
            emitted: RefCell::new(false),
            value: RefCell::new(Some(get_default())),
            get_default: get_default,
            gather: RefCell::new(gather),
            await_works: RefCell::new(Vec::new()),
            present_works: RefCell::new(Vec::new()),
        }
    }
}

impl<B, D, F> SignalRuntimeRefBase<SingleThreadRuntime> for MpscSignalRuntimeRef<B, D, F>
    where B: 'static, D: Fn() -> B + 'static, F: 'static
{
    /// Returns a bool to indicate if the signal was emitted or not on the current instant.
    fn is_emitted(&self) -> bool {
        *self.runtime.emitted.borrow()
    }

    /// Resets the signal at the beginning of each instant.
    fn reset(&mut self) {
        if self.is_emitted() {
            *self.runtime.emitted.borrow_mut() = false;
            *self.runtime.value.borrow_mut() = Some((self.runtime.get_default)());
        }
    }

    /// Exececutes all the continuations found in the vector `self.present_works`.
    fn execute_present_works(&mut self, runtime: &mut SingleThreadRuntime) {
        while let Some(c) = self.runtime.present_works.borrow_mut().pop() {
            c.call_box(runtime, ());
        }
    }
}


impl<B, D, F> SignalRuntimeRefSt for MpscSignalRuntimeRef<B, D, F>
    where B: 'static, D: Fn() -> B + 'static, F: 'static
{
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

impl<A, B, D, F> CanEmit<SingleThreadRuntime, A> for MpscSignalRuntimeRef<B, D, F>
    where A: 'static,
          B: 'static,
          D: Fn() -> B + 'static,
          F: FnMut(A, &mut B) + 'static,
{
    fn emit(&mut self, runtime: &mut SingleThreadRuntime, emitted: A) {
        *self.runtime.emitted.borrow_mut() = true;
        {
            let gather = &mut *self.runtime.gather.borrow_mut();
            match self.runtime.value.borrow_mut().as_mut() {
                Some(v) => gather(emitted, v),
                None => assert!(false),
            }
        }
        while let Some(c) = self.runtime.await_works.borrow_mut().pop() {
            runtime.decr_await_counter();
            c.call_box(runtime, ());
        }
        self.execute_present_works(runtime);
        runtime.emit_signal(Box::new(self.clone()));
    }
}

impl<B, D, F> GetValue<B> for MpscSignalRuntimeRef<B, D, F> {
    /// Returns the value of the signal for the current instant.
    /// This function can only be called once at each instant.
    fn get_value(&self) -> B {
        self.runtime.value.borrow_mut().take().expect(
            "Trying to get the value of a mpsc signal more than once inside an instant.")
    }
}

impl<B, D, F> MpscSignalRuntimeRef<B, D, F>
    where B: 'static, D: Fn() -> B + 'static, F: 'static
{
    /// Returns a new instance of SignalRuntimeRef.
    fn new<A>(get_default: D, gather: F) -> Self where F: FnMut(A, &mut B) {
        MpscSignalRuntimeRef {
            runtime: Rc::new(MpscSignalRuntime::new(get_default, gather)),
        }
    }
}

/// Interface of mpsc signal. This is what is directly exposed to users.
pub struct MpscSignalSt<B, D, F>(MpscSignalRuntimeRef<B, D, F>);

impl<B, D, F> Clone for MpscSignalSt<B, D, F> {
    fn clone(&self) -> Self {
        MpscSignalSt(self.0.clone())
    }
}

impl<B, D, F> Signal for MpscSignalSt<B, D, F>
    where B: 'static, D: Fn() -> B + 'static, F: 'static
{
    type RuntimeRef = MpscSignalRuntimeRef<B, D, F>;
    
    fn runtime(&self) -> MpscSignalRuntimeRef<B, D, F> {
        self.0.clone()
    }
}

impl<B, D, F> ValuedSignal for MpscSignalSt<B, D, F>
    where B: 'static, D: Fn() -> B + 'static, F: 'static
{
    type Stored = B;
    type SigType = MpSignal;
}
    
impl<B, D, F> MpscSignalSt<B, D, F> where B: 'static, D: Fn() -> B + 'static, F: 'static {
    /// Creates a new mpmc signal.
    pub fn new<A>(get_default: D, gather: F) -> Self where A: 'static, F: FnMut(A, &mut B) {
        MpscSignalSt(MpscSignalRuntimeRef::new(get_default, gather))
    }
}

impl MpscSignalSt<(), (), ()> {
    /// Creates a new mpsc signal with the default combination function, which simply
    /// collects all emitted values in a vector.
    pub fn default<A>() -> MpscSignalSt<Vec<A>, fn() -> Vec<A>, fn(A, &mut Vec<A>)>
        where A: 'static
    {
        fn gather<A>(x: A, xs: &mut Vec<A>) {
            xs.push(x);
        }
        MpscSignalSt::new(Vec::new, gather)
    }
}

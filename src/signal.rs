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
    present_works: RefCell<Vec<Box<Continuation<()>>>>,
}

impl SignalRuntime {
    /// Returns a new instance of SignalRuntime.
    fn new() -> Self {
        SignalRuntime {
            emitted: RefCell::new(false),
            await_works: RefCell::new(Vec::new()),
            present_works: RefCell::new(Vec::new()),
        }
    }
}

impl SignalRuntimeRef {
    /// Returns a new instance of SignalRuntimeRef.
    fn new() -> Self {
        SignalRuntimeRef { runtime: Rc::new(SignalRuntime::new()) }
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

    /// Returns a bool to indicate if the signal was emitted or not on the current instant.
    fn is_emitted(&self) -> bool {
        *self.runtime.emitted.borrow()
    }

    /// Resets the signal at the beginning of each instant.
    pub(crate) fn reset(&mut self) {
        *self.runtime.emitted.borrow_mut() = false;
    }

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
    fn on_signal_present<C>(&mut self, runtime: &mut Runtime, c: C) where C: Continuation<()> {
        if *self.runtime.emitted.borrow() {
            c.call(runtime, ());
        } else {
            self.runtime.present_works.borrow_mut().push(Box::new(c));
        }
    }

    /// Exececutes all the continuations found in the vector `self.present_works`.
    pub(crate) fn execute_present_works(&mut self, runtime: &mut Runtime) {
        while let Some(c) = self.runtime.present_works.borrow_mut().pop() {
            c.call_box(runtime, ());
        }
    }

    // TODO: add other methods when needed.
}

/// A reactive signal.
pub trait Signal: Clone + 'static {
    /// Returns a reference to the signal's runtime.
    fn runtime(&mut self) -> SignalRuntimeRef;

    /// Returns a process that emits the signal when it is called.
    fn emit(&mut self) -> Emit<Self> where Self: Sized {
        Emit(self.clone())
    }

    /// Returns a process that waits for the next emission of the signal, current instant
    /// included.
    fn await_immediate(&mut self) -> AwaitImmediate<Self> where Self: Sized {
        AwaitImmediate(self.clone())
    }

    fn present_else<P1, P2>(&mut self, p1: P1, p2: P2) -> PresentElse<Self, P1, P2>
        where Self: Sized, P1: Process, P2: Process
    {
        PresentElse {
            signal: self.clone(),
            present_proc: p1,
            else_proc: p2,
        }
    }

    // TODO: add other methods if needed.
}

pub struct PresentElse<S, P1, P2> {
    signal: S,
    present_proc: P1,
    else_proc: P2,
}

impl<S, P1, P2, V> Process for PresentElse<S, P1, P2>
    where S: Signal, P1: Process<Value=V>, P2: Process<Value=V>
{
    type Value = V;
    
    fn call<C>(mut self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let mut signal_runtime = self.signal.runtime();
        let c = |r: &mut Runtime, ()| {
            if self.signal.runtime().is_emitted() {
                self.present_proc.call(r, next);
            } else {
                r.on_next_instant(Box::new(
                    move |r: &mut Runtime, ()| self.else_proc.call(r, next)));
            }
        };
        signal_runtime.on_signal_present(runtime, c);
        runtime.add_test_signal(signal_runtime);
    }
}

impl <S, P1, P2, V> ProcessMut for PresentElse<S, P1, P2>
    where S: Signal, P1: ProcessMut<Value=V>, P2: ProcessMut<Value=V>
{
    fn call_mut<C>(mut self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        let mut signal_runtime = self.signal.runtime();
        let c = move |r: &mut Runtime, ()| {
            let mut signal = self.signal;
            let (else_proc, present_proc) = (self.else_proc, self.present_proc);
            if signal.runtime().is_emitted() {
                present_proc.call_mut(
                    r, next.map(move |(p, v)| (signal.present_else(p, else_proc), v)))
            } else {
                r.on_next_instant(Box::new(
                    move |r: &mut Runtime, ()| {
                        else_proc.call_mut(
                            r,
                            next.map(move |(p, v)| (signal.present_else(present_proc, p), v))
                        )
                    }
                ));
            }
        };
        signal_runtime.on_signal_present(runtime, c);
        runtime.add_test_signal(signal_runtime);
    }   
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
        self.0.runtime().on_signal(
            runtime,
            next.map(|()| (self, ()))
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

#[derive(Clone)]
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

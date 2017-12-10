use runtime::{Runtime, SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};
use signal::Signal;

/// Defines the behavior of a pure signal. This is the interface exposed to users.
pub trait ValuedSignal: Signal {
    /// The type of value stored in the signal.
    type Stored;

    /// Returns a process that emits the signal with value `emitted` when it is called.
    fn emit<A>(&self, emitted: A) -> Emit<Self, A> where Self: Sized {
        Emit { signal: self.clone(), emitted }
    }

    /// Waits the signal to be emitted and gets its content.  
    /// For a single-producer signal the process terminates immediately and for a
    /// multi-producer signal it terminates at the following instant. 
    /// For example, when `s1` is a spmc signal and when `s2` is a mpmc signal,
    /// `s1.await().pause()` is semantically equivalent to `s2.await()`.
    fn await(&self) -> Await<Self> where Self: Sized {
        Await(self.clone())
    }

    /// Returns the last value associated to the signal when it was emitted.
    /// Evaluates to the `None` before the first emission.
    fn last_value(&self) -> Option<Self::Stored>;
}

/* Emit */

/// Process that represents an emission of a signal with some value.
pub struct Emit<S, A> {
    pub(crate) signal: S,
    pub(crate) emitted: A,
}

impl<S, A> Process for Emit<S, A> where S: ValuedSignal, A: 'static {
    type Value = ();
}

impl<S, A> ProcessMut for Emit<S, A> where S: ValuedSignal, A: 'static {}

pub trait CanEmit<R, A> where R: Runtime {
    /// Emits the value `emitted` to the signal.
    fn emit(&mut self, runtime: &mut R, emitted: A);
}

// Non-parallel

impl<S, A> ProcessSt for Emit<S, A>
    where S: ValuedSignal, S::RuntimeRef: CanEmit<SingleThreadRuntime, A>, A: 'static
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        self.signal.runtime().emit(runtime, self.emitted);
        next.call(runtime, ());
    }
}

impl<S, A> ProcessMutSt for Emit<S, A>
    where S: ValuedSignal, S::RuntimeRef: CanEmit<SingleThreadRuntime, A>, A: Clone + 'static
{
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        self.signal.runtime().emit(runtime, self.emitted.clone());
        next.call(runtime, (self, ()));
    }
}

// Parallel

impl<S, A> ConstraintOnValue for Emit<S, A> {
    type T = ();
}

impl<S, A> ProcessPl for Emit<S, A>
    where S: ValuedSignal + Send + Sync,
          S::RuntimeRef: CanEmit<ParallelRuntime, A>,
          A: Send + Sync + 'static,
{
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        self.signal.runtime().emit(runtime, self.emitted);
        next.call(runtime, ());
    }
}

impl<S, A> ProcessMutPl for Emit<S, A>
    where S: ValuedSignal + Send + Sync,
          S::RuntimeRef: CanEmit<ParallelRuntime, A>,
          A: Clone + Send + Sync + 'static,
{
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        self.signal.runtime().emit(runtime, self.emitted.clone());
        next.call(runtime, (self, ()));
    }
}

/* Await */

/// Process awaiting a signal to be emitted to fetch its value.
pub struct Await<S>(pub(crate) S);

impl<S> Process for Await<S> where S: ValuedSignal {
    type Value = S::Stored;
}

impl<S> ProcessMut for Await<S> where S: ValuedSignal {}

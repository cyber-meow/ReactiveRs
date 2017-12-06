use process::{Process, ProcessMut};
use signal::Signal;

/// Defines the behavior of a pure signal. This is the interface exposed to users.
pub trait MpmcSignal: Signal {
    /// The type of value stored in the signal.
    type Stored;
    
    /// The type for the closure used to gather new value which is emitted.
    type Gather;

    /// Returns a process that emits the signal with value `emitted` when it is called.
    fn emit<A>(&self, emitted: A) -> Emit<Self, A> where Self: Sized {
        Emit { signal: self.clone(), emitted }
    }

    /// Waits the  signal to be emitted, gets its content and
    /// terminates at the following instant.
    fn await(&self) -> Await<Self> where Self: Sized {
        Await(self.clone())
    }

    /// Returns the last value associated to the signal when it was emitted.
    /// Evaluates to the default value before the first emission.
    fn last_value(&self) -> Self::Stored;
}

pub struct Emit<S, A> {
    pub(crate) signal: S,
    pub(crate) emitted: A,
}

impl<S, A> Process for Emit<S, A>
    where S: MpmcSignal, S::Gather: FnMut(A, &mut S::Stored), A: 'static 
{
    type Value = ();
}

impl<S, A> ProcessMut for Emit<S, A>
    where S: MpmcSignal, S::Gather: FnMut(A, &mut S::Stored), A: 'static {}

pub struct Await<S>(pub(crate) S);

impl<S> Process for Await<S> where S: MpmcSignal {
    type Value = S::Stored;
}

impl<S> ProcessMut for Await<S> where S: MpmcSignal {}

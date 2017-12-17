use process::{Process, ProcessMut};
use signal::Signal;

/// Defines the behavior of a pure signal. This is the interface exposed to users.
pub trait PureSignal: Signal {
    /// Creates a new pure signal.
    fn new() -> Self;

    /// Returns a process that emits the signal when it is called.
    fn emit(&self) -> Emit<Self> where Self: Sized {
        Emit(self.clone())
    }

    fn try_emit(&self) -> TryEmit<Self> where Self: Sized {
        TryEmit(self.clone())
    }
}

pub struct Emit<S>(pub(crate) S);

impl<S> Process for Emit<S> where S: PureSignal {
    type Value = ();
}

impl<S> ProcessMut for Emit<S> where S: PureSignal {}

pub struct TryEmit<S>(pub(crate) S);

impl<S> Process for TryEmit<S> where S: PureSignal {
    type Value = bool;
}

impl<S> ProcessMut for TryEmit<S> where S: PureSignal {}

//! A reactive signal without value.

use process::{Process, ProcessMut};
use signal::Signal;

/// Defines the behavior of a pure signal.
pub trait PureSignal: Signal {
    /// Creates a new pure signal.
    fn new() -> Self;

    /// Returns a process that emits the signal when it is called.
    fn emit(&self) -> Emit<Self> where Self: Sized {
        Emit(self.clone())
    }

    /// Emits the signal if it is not yet emitted and returns a bool indicating
    /// if the emission is successful.
    fn try_emit(&self) -> TryEmit<Self> where Self: Sized {
        TryEmit(self.clone())
    }
}

/// Emits a pure signal.
pub struct Emit<S>(pub(crate) S);

impl<S> Process for Emit<S> where S: PureSignal {
    type Value = ();
}

impl<S> ProcessMut for Emit<S> where S: PureSignal {}

/// Emits a pure signal if it is not yet emitted. Created by the `try_emit` method.
pub struct TryEmit<S>(pub(crate) S);

impl<S> Process for TryEmit<S> where S: PureSignal {
    type Value = bool;
}

impl<S> ProcessMut for TryEmit<S> where S: PureSignal {}

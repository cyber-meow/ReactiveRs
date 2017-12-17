//! A reactive signal with value.

mod emit;
mod await;
mod try_emit;
pub use self::emit::{EmitValue, CanEmit};
pub use self::await::{AwaitValue, GetValue};
pub use self::try_emit::{TryEmitValue, CanTryEmit};

use std::marker::PhantomData;

use signal::Signal;

/// A reactive signal with value.
pub trait ValuedSignal: Signal {
    /// The value stored in the signal.
    type Stored;

    /// A more specific type of a signal. This is necessary because some generic
    /// implementation may depend on the specific type of the signal.
    type SigType: SignalType;

    /// Returns a process that emits the signal with value `emitted` when it is called.
    fn emit<A>(&self, emitted: A) -> EmitValue<Self, A> where Self: Sized {
        EmitValue { signal: self.clone(), emitted }
    }

    /// Waits the signal to be emitted and gets its content.
    ///
    /// For a single-producer signal the process terminates immediately and for a
    /// multi-producer signal it terminates at the following instant. 
    /// For example, when `s1` is a spmc signal and when `s2` is a mpmc signal,
    /// `s1.await().pause()` is semantically equivalent to `s2.await()`.
    fn await(&self) -> AwaitValue<Self, Self::SigType> where Self: Sized {
        AwaitValue{ signal: self.clone(), signal_type: PhantomData }
    }
}

/// Define some subtypes that a signal with value can have.
pub trait SignalType: 'static {}

/// Multi-producer signal type. For multi-producer signal, `await` terminates at
/// the following instant.
pub struct MpSignal;

impl SignalType for MpSignal {}

/// Single-producer signal type. For single-producer signal, `await` terminates immediately.
pub struct SpSignal;

impl SignalType for SpSignal {}

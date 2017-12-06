//! Inter-process communicatin depends on the use of signals.
//! Several types of signals are defined.
//!
//! In particular, the signals for the non-parallel and the parallel version
//! of the library are different, so the user must decide which kind of
//! signal (the parallel or no-parallel ones) to use from the beginning.
//!
//! I would love to have something like in the case of processes: the real
//! behavior of the signal is only determined when it's used with some
//! particular runtime, but I didn't find a way to do this.

pub(crate) mod signal_runtime;

mod await_immediate;
mod present_else;
pub use self::await_immediate::AwaitImmediate;
pub use self::present_else::PresentElse;

pub mod pure_signal;
pub mod valued_signal;
pub use self::pure_signal::PureSignal;
pub use self::valued_signal::ValuedSignal;

pub mod parallel;
pub mod single_thread;

use process::Process;

/// A reactive signal.  
/// The signal implement the trait `Clone` to assure that it can be used multiple times
/// in the program. However, note that for most of the constructions `clone` is used
/// implicitly so one can pass the signal directly.
/// The user needs to call `clone` explicitly only in scenarios where the signal's
/// ownership must be shared in different places, ex: closure.
pub trait Signal: Clone + 'static {
    /// The runtime reference type associated with the signal.
    type RuntimeRef;
    
    /// Returns a reference to the signal's runtime.
    fn runtime(&self) -> Self::RuntimeRef;
    
    /// Returns a process that waits for the next emission of the signal, current instant
    /// included.
    fn await_immediate(&self) -> AwaitImmediate<Self> where Self: Sized {
        AwaitImmediate(self.clone())
    }
    
    /// Test the status of a signal `s`. If the signal is present, the process `p1`
    /// is executed instantaneously, other wise `p2` is executed at the following instant.
    fn present_else<P1, P2, V>(&self, p1: P1, p2: P2) -> PresentElse<Self, P1, P2>
        where Self: Sized, P1: Process<Value=V>, P2: Process<Value=V>
    {
        PresentElse {
            signal: self.clone(),
            present_proc: p1,
            else_proc: p2,
        }
    }
}

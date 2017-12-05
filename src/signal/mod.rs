use runtime::Runtime;
use continuation::Continuation;

pub mod signal_runtime;
use self::signal_runtime::{SignalRuntimeRefBase, SignalRuntimeRef};

/// A reactive signal.  
/// The signal implement the trait `Clone` to assure that it can be used multiple times
/// in the program. However, note that for most of the constructions `clone` is used
/// implicitly so one can pass the signal directly.
/// The user needs to call `clone` explicitly only in scenarios where the signal's
/// ownership must be shared in different places, ex: closure.
pub trait Signal<R>: Clone + 'static where R: Runtime {
    /// The runtime reference type associated with the signal.
    type RuntimeRef: SignalRuntimeRef<R>;

    /// Returns a reference to the signal's runtime.
    fn runtime(&self) -> Self::RuntimeRef;
}

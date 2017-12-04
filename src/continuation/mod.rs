mod map;
pub use self::map::Map;
mod pause;
pub use self::pause::Pause;

use runtime::{Runtime, SingleThreadRuntime, ParallelRuntime};

/// A reactive continuation awaiting a value of type `V`. For the sake of simplicity,
/// continuation must be valid on the static lifetime.
pub trait Continuation<R, V>: 'static where R: Runtime {
    /// Calls the continuation.
    fn call(self, runtime: &mut R, value: V);

    /// Calls the continuation. Works even if the continuation is boxed.
    ///
    /// This is necessary because the size of a value must be known to unbox it. It is
    /// thus impossible to take the ownership of a `Box<Continuation>` whitout knowing the
    /// underlying type of the `Continuation`.
    fn call_box(self: Box<Self>, runtime: &mut R, value: V);
    
    /// Creates a new continuation that applies a function to the input value before
    /// calling `Self`.
    fn map<F, V2>(self, map: F) -> Map<Self, F> where Self: Sized, F: FnOnce(V2) -> V + 'static {
        Map { continuation: self, map }
    }

    /// Create a new continuation that calls `Self` in the next instant.
    fn pause(self) -> Pause<Self> where Self: Sized {
        Pause(self)
    }
}

impl<R, V, F> Continuation<R, V> for F where R: Runtime, F: FnOnce(&mut R, V) + 'static {
    fn call(self, runtime: &mut R, value: V) {
        self(runtime, value);
    }

    fn call_box(self: Box<Self>, runtime: &mut R, value: V) {
        (*self).call(runtime, value);
    }
}

/// Continuation to be run in a single thread.
pub trait ContinuationSt<V>: Continuation<SingleThreadRuntime, V> {}

impl<C, V> ContinuationSt<V> for C where C: Continuation<SingleThreadRuntime, V> {}

/// Continuation which can be safely passed and shared between different threads.
/// Used for the parallel implementation of the library.
/// The `Sync` trait is only necessary when signals come into the scene.
pub trait ContinuationPl<V>: Continuation<ParallelRuntime, V> + Send + Sync {}

impl<C, V> ContinuationPl<V> for C where C: Continuation<ParallelRuntime, V> + Send + Sync {}

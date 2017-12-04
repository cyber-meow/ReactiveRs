use runtime::{Runtime, SingleThreadRuntime};

/// A reactive continuation awaiting a value of type `V`. For the sake of simplicity,
/// continuation must be valid on the static lifetime.
pub trait Continuation<R, V>: 'static {
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

impl<R, V, F> Continuation<R, V> for F where F: FnOnce(&mut R, V) + 'static {
    fn call(self, runtime: &mut R, value: V) {
        self(runtime, value);
    }

    fn call_box(self: Box<Self>, runtime: &mut R, value: V) {
        (*self).call(runtime, value);
    }
}

/// A continuation that applies a function before calling another continuation.
pub struct Map<C, F> { continuation: C, map: F }

impl<R, C, F, V1, V2> Continuation<R, V1> for Map<C, F>
    where C: Continuation<R, V2>, F: FnOnce(V1) -> V2 + 'static
{
    fn call(self, runtime: &mut R, value: V1) {
        let v2 = (self.map)(value);
        self.continuation.call(runtime, v2);
    }

    fn call_box(self: Box<Self>, runtime: &mut R, value: V1) {
        (*self).call(runtime, value);
    }
}

/// A continuation that calls another continuation in the next instant.
pub struct Pause<C>(C);

impl<C, V> Continuation<SingleThreadRuntime, V> for Pause<C>
    where C: Continuation<SingleThreadRuntime, V>, V: 'static
{
    fn call(self, runtime: &mut SingleThreadRuntime, value: V) {
        runtime.on_next_instant(Box::new(self.0.map({|_| value})));
    }
    
    fn call_box(self: Box<Self>, runtime: &mut SingleThreadRuntime, value: V) {
        (*self).call(runtime, value);
    }
}

/*
pub trait ContinuationPl<R, V>: Continuation<R, V> + Send + Sync {}

impl<R, V, F> ContinuationPl<R, V> for F
    where F: FnOnce(&mut R, V) + Send + Sync + 'static {}

impl<R, C, F, V1, V2> ContinuationPl<R, V1> for Map<C, F>
    where C: ContinuationPl<R, V2>,
          F: FnOnce(V1) -> V2 + Send + Sync + 'static,
          V2: Send + Sync + 'static {}

impl<R, C, V> ContinuationPl<R, V> for Pause<C>
    where R: Runtime, C: ContinuationPl<R, V>, V: Send + Sync + 'static {}*/

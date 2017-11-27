//! One possibility to put the codes of the two implementations together.

use parallel::Runtime;

/// A reactive continuation awaiting a value of type `V`. For the sake of simplicity,
/// continuation must be valid on the static lifetime.
pub trait ContinuationBase<V>: 'static {
    /// Calls the continuation.
    fn call(self, runtime: &mut Runtime, value: V);

    /// Calls the continuation. Works even if the continuation is boxed.
    ///
    /// This is necessary because the size of a value must be known to unbox it. It is
    /// thus impossible to take the ownership of a `Box<Continuation>` whitout knowing the
    /// underlying type of the `Continuation`.
    fn call_box(self: Box<Self>, runtime: &mut Runtime, value: V);
    
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

impl<V, F> ContinuationBase<V> for F where F: FnOnce(&mut Runtime, V) + 'static {
    fn call(self, runtime: &mut Runtime, value: V)  {
        self(runtime, value);
    }

    fn call_box(self: Box<Self>, runtime: &mut Runtime, value: V) {
        (*self).call(runtime, value);
    }
}

/// A continuation that applies a function before calling another continuation.
pub struct Map<C, F> { continuation: C, map: F }

impl<C, F, V1, V2> ContinuationBase<V1> for Map<C, F>
    where C: Continuation<V2>, F: FnOnce(V1) -> V2 + 'static, V2: Send + 'static
{
    fn call(self, runtime: &mut Runtime, value: V1) {
        let v2 = (self.map)(value);
        let c = self.continuation;
        runtime.on_current_instant(
            Box::new(|r: &mut Runtime, ()| c.call(r, v2)));
    }

    fn call_box(self: Box<Self>, runtime: &mut Runtime, value: V1) {
        (*self).call(runtime, value);
    }
}

/// A continuation that calls another continuation in the next instant.
pub struct Pause<C>(C);

impl<C, V> ContinuationBase<V> for Pause<C> where C: Continuation<V>, V: Send + 'static {
    fn call(self, runtime: &mut Runtime, value: V) {
        runtime.on_next_instant(Box::new(self.0.map({|_| value})));
    }
    
    fn call_box(self: Box<Self>, runtime: &mut Runtime, value: V) {
        (*self).call(runtime, value);
    }
}

/// A reactive continuation awaiting a value of type `V`. For the sake of simplicity,
/// continuation must be valid on the static lifetime.
pub trait Continuation<V>: Send + ContinuationBase<V> {}

impl<V, F> Continuation<V> for F where F: FnOnce(&mut Runtime, V) + Send + 'static {}

impl<C, F, V1, V2> Continuation<V1> for Map<C, F>
    where C: Continuation<V2>, F: FnOnce(V1) -> V2 + Send + 'static, V2: Send + 'static {}

impl<C, V> Continuation<V> for Pause<C> where C: Continuation<V>, V: Send + 'static {}

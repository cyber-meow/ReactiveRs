use runtime::Runtime;
use continuation::Continuation;
use process::{Process, ProcessMut, ConstraintOnValue};

/// Create a new process that returns the value v immediately.
pub fn value<V>(v: V) -> Value<V> where V: 'static {
    Value(v)
}

/// A process that returns a value of type V.
pub struct Value<V>(V);

impl<R, V> Process<R> for Value<V> where R: Runtime, V: 'static {
    type Value = V;
    
    fn call<C>(self, runtime: &mut R, next: C) where C: Continuation<R, V> {
        next.call(runtime, self.0);
    }
}

impl<R, V> ProcessMut<R> for Value<V> where R: Runtime, V: Copy + 'static {
    fn call_mut<C>(self, runtime: &mut R, next: C)
        where Self: Sized, C: Continuation<R, (Self, Self::Value)>
    {
        let v = self.0;
        next.call(runtime, (self, v));
    }
}

impl<V> ConstraintOnValue for Value<V> where V: Send + Sync + 'static {
    type T = V;
}

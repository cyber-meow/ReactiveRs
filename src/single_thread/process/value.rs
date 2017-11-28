use {Runtime, Continuation};
use process::{Process, ProcessMut};

/// Create a new process that returns the value v immediately.
pub fn value<V>(v: V) -> Value<V> where V: 'static {
    Value(v)
}

/// A process that returns a value of type V.
pub struct Value<V>(V);

impl<V> Process for Value<V> where V: 'static {
    type Value = V;
    
    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<V> {
        next.call(runtime, self.0);
    }
}

impl<V> ProcessMut for Value<V> where V: Copy + 'static {
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        let v = self.0;
        next.call(runtime, (self, v));
    }
}

use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

/// Create a new process that returns the value v immediately.
pub fn value<V>(v: V) -> Value<V> where V: 'static {
    Value(v)
}

/// A process that returns a value of type V.
pub struct Value<V>(V);

impl<V> Process for Value<V> where V: 'static {
    type Value = V;
}

impl<V> ProcessSt for Value<V> where V: 'static {
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C) where C: ContinuationSt<V> {
        next.call(runtime, self.0);
    }
}

impl<V> ProcessMutSt for Value<V> where V: Copy + 'static {
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let v = self.0;
        next.call(runtime, (self, v));
    }
}

impl<V> ConstraintOnValue for Value<V> where V: Send + Sync + 'static {
    type T = V;
}

impl<V> ProcessPl for Value<V> where V: Send + Sync + 'static {
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C) where C: ContinuationPl<V> {
        next.call(runtime, self.0);
    }
}

impl<V> ProcessMutPl for Value<V> where V: Copy + Send + Sync + 'static {
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let v = self.0;
        next.call(runtime, (self, v));
    }
}

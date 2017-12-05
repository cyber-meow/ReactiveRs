use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{Continuation, ContinuationSt, ContinuationPl};

/// A continuation that calls another continuation in the next instant.
pub struct Pause<C>(pub(crate) C);

impl<C, V> Continuation<SingleThreadRuntime, V> for Pause<C>
    where C: ContinuationSt<V>, V: 'static
{
    fn call(self, runtime: &mut SingleThreadRuntime, value: V) {
        runtime.on_next_instant(Box::new(self.0.map({|()| value})));
    }
    
    fn call_box(self: Box<Self>, runtime: &mut SingleThreadRuntime, value: V) {
        (*self).call(runtime, value);
    }
}

impl<C, V> Continuation<ParallelRuntime, V> for Pause<C>
    where C: ContinuationPl<V>, V: Send + Sync + 'static
{
    fn call(self, runtime: &mut ParallelRuntime, value: V) {
        runtime.on_next_instant(Box::new(self.0.map({|()| value})));
    }
    
    fn call_box(self: Box<Self>, runtime: &mut ParallelRuntime, value: V) {
        (*self).call(runtime, value);
    }
}

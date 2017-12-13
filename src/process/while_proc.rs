use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{Continuation, ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

/// Indicates if a loop is terminated or not.
#[derive(Copy, Clone)]
pub enum LoopStatus<V> { Continue, Exit(V) }

/// Repeats a process having `LoopStatus` as return type until it returns `Exit(v)`
/// in which case the created process exits and returns `v`.
pub struct While<P>(pub(crate) P);

impl<P, V> Process for While<P> where P: ProcessMut<Value=LoopStatus<V>> {
    type Value = V;
}

impl<P, V> ProcessMut for While<P> where P: ProcessMut<Value=LoopStatus<V>> {}

// Implements the traits for the single thread version of the library.

impl<P, V> ProcessSt for While<P> where P: ProcessMutSt<Value=LoopStatus<V>> {
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        self.0.call_mut(runtime, WhileContinuation(next.map(|(_, v)| v)));
    }
}

impl<P, V> ProcessMutSt for While<P> where P: ProcessMutSt<Value=LoopStatus<V>> {
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        self.0.call_mut(runtime, WhileContinuation(next));
    }
}

/// The continuation to call when using the `while_proc` combinator.
/// Note that the implementation supposed by default being called by `call_mut`.
struct WhileContinuation<C>(C);

impl<P, C, V> Continuation<SingleThreadRuntime, (P, P::Value)> for WhileContinuation<C>
    where P: ProcessMutSt<Value=LoopStatus<V>>, C: ContinuationSt<(While<P>, V)>
{
    fn call(self, runtime: &mut SingleThreadRuntime, (p, loop_status): (P, LoopStatus<V>)) {
        match loop_status {
            LoopStatus::Continue => p.call_mut(runtime, self),
            LoopStatus::Exit(v) => self.0.call(runtime, (p.while_proc(), v)),
        }
    }

    fn call_box(self: Box<Self>, runtime: &mut SingleThreadRuntime, value: (P, LoopStatus<V>)) {
        (*self).call(runtime, value);
    }
}

// Implements the traits for the parallel version of the library.

impl<P, V> ConstraintOnValue for While<P>
    where P: ProcessMut<Value=LoopStatus<V>>, V: Send + Sync
{
    type T = V;
}

// Note that we must use `<T=..>` instead of `<Value=..>` because in the definition of 
// `ProcessPl` we have <Value = <Self::ConstraintOnValue>::T>.

impl<P, V> ProcessPl for While<P> where P: ProcessMutPl<T=LoopStatus<V>>, V: Send + Sync {
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        self.0.call_mut(runtime, WhileContinuation(next.map(|(_, v)| v)));
    }
}

impl<P, V> ProcessMutPl for While<P> where P: ProcessMutPl<T=LoopStatus<V>>, V: Send + Sync {
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        self.0.call_mut(runtime, WhileContinuation(next));
    }
}

impl<P, C, V> Continuation<ParallelRuntime, (P, P::Value)> for WhileContinuation<C>
    where P: ProcessMutPl<T=LoopStatus<V>>, C: ContinuationPl<(While<P>, V)>, V: Send + Sync
{
    fn call(self, runtime: &mut ParallelRuntime, (p, loop_status): (P, LoopStatus<V>)) {
        match loop_status {
            LoopStatus::Continue => p.call_mut(runtime, self),
            LoopStatus::Exit(v) => self.0.call(runtime, (p.while_proc(), v)),
        }
    }

    fn call_box(self: Box<Self>, runtime: &mut ParallelRuntime, value: (P, LoopStatus<V>)) {
        (*self).call(runtime, value);
    }
}

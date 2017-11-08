use {Runtime, Continuation};
use process::Process;

/// A process that can be executed multiple times, modifying its environement each time.
pub trait ProcessMut: Process {
    /// Executes the mutable process in the runtime, then calls `next` with the process and the
    /// process's return value.
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>;

    fn while_loop<V>(self) -> While<Self> where Self: ProcessMut<Value=LoopStatus<V>> + Sized {
        While(self)
    }
}

pub enum LoopStatus<V> { Continue, Exit(V) }

pub struct While<P>(P);

impl<P, V> Process for While<P> where P: ProcessMut<Value=LoopStatus<V>> {
    type Value = V;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.0.call_mut(runtime, WhileContinuation(next));
    }
}

pub struct WhileContinuation<C>(C);

impl<P, C, V> Continuation<(P, P::Value)> for WhileContinuation<C>
    where P: ProcessMut<Value=LoopStatus<V>>, C: Continuation<V>
{
    fn call(self, runtime: &mut Runtime, (process, loop_status): (P, LoopStatus<V>)) {
        match loop_status {
            LoopStatus::Continue => process.call_mut(runtime, self),
            LoopStatus::Exit(v) => self.0.call(runtime, v),
        }
    }

    fn call_box(self: Box<Self>, runtime: &mut Runtime, value: (P, LoopStatus<V>)) {
        (*self).call(runtime, value);
    }
}

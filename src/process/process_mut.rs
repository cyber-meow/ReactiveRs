use { Runtime, Continuation, Process };
use process::{ Value, AndThen };

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

impl<V> ProcessMut for Value<V> where V: Copy + 'static {
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        let v = self.0;
        next.call(runtime, (self, v));
    }
}

impl<P1, P2, F> ProcessMut for AndThen<P1, F>
    where P1: ProcessMut, P2: Process, F: FnMut(P1::Value) -> P2 + 'static
{
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        let mut chain = self.chain;
        let c = |r: &mut Runtime, (process, v): (P1, P1::Value)| {
            chain(v).map(|v2| (process.and_then(chain), v2)).call(r, next);
        };
        self.process.call_mut(runtime, c);
    }
}

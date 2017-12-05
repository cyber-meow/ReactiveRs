use runtime::{Runtime, SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{ProcessSt, ProcessPl};

/// A process that can be executed multiple times, modifying its environement each time.
pub trait ProcessMutSt: ProcessSt {
    /// Executes the mutable process in the runtime, then calls `next` with the process and the
    /// process's return value.
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>;
}

/// A process that can be executed multiple times, modifying its environement each time.
pub trait ProcessMutPl: ProcessPl {
    /// Executes the mutable process in the runtime, then calls `next` with the process and the
    /// process's return value.
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>;
}


/*
    /// A classic loop that continues or stops accroding to the returned value of the process.
    fn while_proc<V>(self) -> While<Self>
        where Self: ProcessMut<Value=LoopStatus<V>> + Sized, V: Send + Sync
    {
        While(self)
    }

    /// An infinite loop. The process must return type ().
    fn loop_proc(self)-> Loop<Self> where Self: ProcessMut<Value=()> + Sized {
        Loop(self)
    }
}

/// Indicates if a loop is terminated or not.
pub enum LoopStatus<V> { Continue, Exit(V) }

/// Repeats a process having `LoopStatus` as return type until it returns `Exit(v)`
/// in which case the created process exits and returns `v`.
pub struct While<P>(P);

impl<P, V> Process for While<P> where P: ProcessMut<Value=LoopStatus<V>>, V: Send + Sync {
    type Value = V;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.0.call_mut(runtime, WhileContinuation(next.map(|(_, v)| v)));
    }
}

impl<P, V> ProcessMut for While<P> where P: ProcessMut<Value=LoopStatus<V>>, V: Send + Sync {
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        self.0.call_mut(runtime, WhileContinuation(next));
    }
}

/// The continuation to call when using the `while_proc` combinator.
/// Note that the implementation supposed by default being called by `call_mut`.
pub struct WhileContinuation<C>(C);

impl<P, C, V> Continuation<(P, P::Value)> for WhileContinuation<C>
    where P: ProcessMut<Value=LoopStatus<V>>, C: Continuation<(While<P>, V)>, V: Send + Sync
{
    fn call(self, runtime: &mut Runtime, (process, loop_status): (P, LoopStatus<V>)) {
        match loop_status {
            LoopStatus::Continue => process.call_mut(runtime, self),
            LoopStatus::Exit(v) => self.0.call(runtime, (process.while_proc(), v)),
        }
    }

    fn call_box(self: Box<Self>, runtime: &mut Runtime, value: (P, LoopStatus<V>)) {
        (*self).call(runtime, value);
    }
}

/// Repeats a process forever.
pub struct Loop<P>(P);

impl<P> Process for Loop<P> where P: ProcessMut<Value=()> {
    type Value = ();

    fn call<C>(self, runtime: &mut Runtime, _: C) where C: Continuation<Self::Value> {
        self.0.call_mut(runtime, LoopContinuation);
    }
}

// TODO: This is useless without other control structures.
impl<P> ProcessMut for Loop<P> where P: ProcessMut<Value=()> {
    fn call_mut<C>(self, runtime: &mut Runtime, _: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        self.0.call_mut(runtime, LoopContinuation);
    }
}

/// The continuation to call when using the `loop_proc` combinator.
pub struct LoopContinuation;

impl<P> Continuation<(P, P::Value)> for LoopContinuation where P: ProcessMut<Value=()> {
    fn call(self, runtime: &mut Runtime, (process, ()): (P, ())) {
        process.call_mut(runtime, self);
    }

    fn call_box(self: Box<Self>, runtime: &mut Runtime, value: (P, ())) {
        (*self).call(runtime, value);
    }
}*/

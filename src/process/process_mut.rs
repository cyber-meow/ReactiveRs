use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessSt, ProcessPl};

use process::while_proc::{While, LoopStatus};
use process::loop_proc::Loop;

/// A process that can be executed multiple times, modifying its environement each time.
pub trait ProcessMut: Process {
    /// A classic loop that continues or stops accroding to the returned value of the process.
    fn while_proc<V>(self) -> While<Self>
        where Self: ProcessMut<Value=LoopStatus<V>> + Sized
    {
        While(self)
    }
    
    /// An infinite loop. The returned value of the process is ignored if any.
    fn loop_proc(self)-> Loop<Self> where Self: ProcessMut + Sized {
        Loop(self)
    }
}

/// A repeatable reactive process to be executed in a single thread.
pub trait ProcessMutSt: ProcessMut + ProcessSt {
    /// Executes the mutable process in the runtime, then calls `next` with the process and the
    /// process's return value.
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>;
}

/// A repeatable reactive process that can be safely passed and shared between threads.
pub trait ProcessMutPl: ProcessMut + ProcessPl {
    /// Executes the mutable process in the runtime, then calls `next` with the process and the
    /// process's return value.
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>;
}

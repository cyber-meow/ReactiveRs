//! A process encapsulates what should be runned by a reactive engine.
//!
//! Starting from the function `value_proc` or some signal, users are free
//! to define their own process to be executed in the reactive environment
//! using the methods that are offered by the traits `Process` and `ProcessMut`.
//! There is no need to manipulate directly the runtime engines because we only
//! need to call `execute_process`, `execute_process_parallel`, or
//! `execute_process_parallel_with_main` at the end to execute the process.
//!
//! If a process is only defined with things found in this module (in other words,
//! no signal is used), we can execute it in the two kinds of runtime as long as
//! it's `Send` and `Sync`.
//!
//! Concerning the code structure, the code for the parallel and non-parallel part of
//! the library are in fact very similar. However, I'm not able to figure out a way
//! to design traits so that all can be put together through some abstraction in order
//! to reduce repeated code.
//!
//! The problem is that the method `call` has a generic type parameter `C` and in the
//! two cases it must implement the trait `ContinuationSt` or `ContinuationPl`.
//! I didn't find a way to integrate this information into a same trait, so I must
//! have two separated trait implementations.
//! For example, a trait that can able to parametrized by another trait could be very
//! helpful, but that doesn't exist in Rust at this moment (and I admit that I don't
//! even know if this is possible from a theretical viewpoint).

mod execute_process;
mod process_mut;
pub use self::execute_process::{execute_process, execute_process_parallel};
pub use self::execute_process::execute_process_parallel_with_main;
pub use self::process_mut::{ProcessMut, ProcessMutSt, ProcessMutPl};

mod value;
mod pause;
mod map;
mod flatten;
mod and_then;
mod then;
mod if_else;
mod join;
mod join_p;
mod join_all;
mod join_all_p;
mod loop_proc;
mod repeat;
mod while_proc;
pub use self::value::{value_proc, Value};
pub use self::pause::Pause;
pub use self::map::Map;
pub use self::flatten::Flatten;
pub use self::and_then::AndThen;
pub use self::then::Then;
pub use self::if_else::IfElse;
pub use self::join::Join;
pub use self::join_all::{join_all, JoinAll};
pub use self::loop_proc::Loop;
pub use self::repeat::Repeat;
pub use self::while_proc::{While, LoopStatus};

use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};

/// A abstract reactive process. A method `call` is in fact also necessary.
/// Please see `ProcessSt` and `ProcessPl` for more information.
pub trait Process: 'static {
    /// The value created by the process.
    type Value;  

    /// Suspends the execution of a process until next instant.
    fn pause(self) -> Pause<Self> where Self: Sized {
        Pause(self)
    }
    
    /// Applies a function to the value returned by the process before passing it to
    /// its continuation.
    fn map<F, V>(self, map: F) -> Map<Self, F>
        where Self: Sized, F: FnOnce(Self::Value) -> V + 'static
    {
        Map { process: self, map }
    } 

    /// Flattens the execution of a process when its returned value is itself another process.
    fn flatten(self) -> Flatten<Self> where Self: Sized, Self::Value: Process {
        Flatten(self)
    }

    /// Chains another process after the exectution of one process (like the `bind` for a monad).
    fn and_then<F, P>(self, chain: F) -> AndThen<Self, F>
        where Self: Sized, F: FnOnce(Self::Value) -> P + 'static, P: Process
    {
        AndThen { process: self, chain }
    }

    /// Executes a second process after one process terminates.
    /// The returned value of the first process is ignored.
    fn then<P>(self, successor: P) -> Then<Self, P> where Self: Sized, P: Process {
        Then { process: self, successor }
    }

    /// Decides whether to execute `if_branch` or `else_branch` according to
    /// the returned value of `self`, which must be of type `bool`.  
    /// The combinator `and_then` defined earlier together with the built-in
    /// `if`-`else` branching in Rust cannot allow us to achieve the same purpose
    /// since `if` branch and `else` branch in Rust must result in the same type.
    fn if_else<P1, P2, V>(self, if_branch: P1, else_branch: P2) -> IfElse<Self, P1, P2>
        where Self: Process<Value=bool> + Sized, P1: Process<Value=V>, P2: Process<Value=V>
    {
        IfElse {
            process: self,
            if_branch,
            else_branch,
        }
    }

    /// Executes two processes in parallel. 
    fn join<P>(self, proc2: P) -> Join<Self, P> where Self: Sized, P: Process {
        Join(self, proc2)
    }
}

/// A reactive process to be executed in a single thread.
pub trait ProcessSt: Process {
    /// Executes the reactive process in the runtime, calls `next` with the resulting value.
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>;
}

/// A reactive process that can be safely passed and shared between threads.
pub trait ProcessPl: 
        Process<Value = <Self as ConstraintOnValue>::T>
        + ConstraintOnValue + Send + Sync {
    /// Executes the reactive process in the runtime, calls `next` with the resulting value.
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>;
}

/// This is a workaround to have constraints on associated types. Must be implemented
/// by every type that want to represent some parallel process.
//
// For the implementation, we would like have something like this but it can
// cause cyclic evaluation.
// (overflow evaluating the requirement `<Self as process::ConstraintOnValue>::T`)
//
// ```
// impl<P> ConstraintOnValue for P where P: Process, P::Value: Send + Sync {
//     type T = P::Value;
// }
// ```
pub trait ConstraintOnValue {
    type T: Send + Sync;
}

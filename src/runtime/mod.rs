//! One runtime have some tasks to do at hand and chooses one to do at each time.
//!
//! This is where the execution engines are defined. The tasks to be carry out
//! are some continuations which are charaterized by the trait
//! `continuation::Continuation`. Each instants terminates only when there are no
//! more tasks to do at this instant. The whole execution terminates when
//! no more continuation is left in the runtime(s) and no signal is awaiting for
//! emisssion.
//!
//! A `SingleThreadRuntime` is itself the whole execution engine and is runned on
//! the main thread. In contrast, a `ParallelRuntime` is spawned on a new thread
//! and is only one part of the whole parallel engine which is given by the struct
//! `ParallelRuntimeCollection`.

mod single_thread_runtime;
pub use self::single_thread_runtime::SingleThreadRuntime;
mod parallel_runtime;
pub use self::parallel_runtime::ParallelRuntime;
mod parallel_runtime_collection;
pub use self::parallel_runtime_collection::ParallelRuntimeCollection;

/// Must be implemented by all concrete runtime types.
pub trait Runtime {
    /// Executes instants until all work is completed.
    fn execute(&mut self) {
        while self.instant() {};
    }

    /// Executes a single instant to completion. Indicates if more work remains to be done.
    fn instant(&mut self) -> bool;
}

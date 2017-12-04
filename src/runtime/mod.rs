mod single_thread_runtime;
pub use self::single_thread_runtime::SingleThreadRuntime;
mod parallel_runtime;
pub use self::parallel_runtime::ParallelRuntime;
/*mod parallel_runtime_collection;
pub use self::parallel_runtime_collection::ParallelRuntimeCollection;*/

pub trait Runtime {
    /// Executes instants until all work is completed.
    fn execute(&mut self) {
        while self.instant() {};
    }

    /// Executes a single instant to completion. Indicates if more work remains to be done.
    fn instant(&mut self) -> bool;
}

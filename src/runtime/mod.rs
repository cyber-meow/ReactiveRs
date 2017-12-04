mod single_thread_runtime;
pub use self::single_thread_runtime::SingleThreadRuntime;
/*mod parallel_runtime;
pub use self::parallel_runtime::ParallelRuntime;
mod parallel_runtime_collection;
pub use self::parallel_runtime_collection::ParallelRuntimeCollection;*/

use Continuation;
use signal::signal_runtime::SignalRuntimeRefBase;

pub trait Runtime: 'static {
    /// Executes instants until all work is completed.
    fn execute(&mut self);

    /// Executes a single instant to completion. Indicates if more work remains to be done.
    fn instant(&mut self) -> bool;

    /// Registers a continuation to execute on the current instant.
    fn on_current_instant(&mut self, c: Box<Continuation<Self, ()>>);

    /// Registers a continuation to execute at the next instant.
    fn on_next_instant(&mut self, c: Box<Continuation<Self, ()>>);

    /// Registers a continuation to execute at the end of the instant. Runtime calls for `c`
    /// behave as if they where executed during the next instant.
    fn on_end_of_instant(&mut self, c: Box<Continuation<Self, ()>>);

    /// Increases the await counter by 1 when some process await a signal to continue.
    fn incr_await_counter(&mut self);

    /// Decrease the await counter by 1 when some signal is emitted and
    /// the corresponding process is thus executed.
    fn decr_await_counter(&mut self);
    
    /// Registers a emitted signal for the current instant.
    fn emit_signal(&mut self, s: Box<SignalRuntimeRefBase<Self>>);

    /// Registers a signal for which we need to test its presence on the current instant.
    fn add_test_signal(&mut self, s: Box<SignalRuntimeRefBase<Self>>);
}

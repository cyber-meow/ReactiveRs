use runtime::{Runtime, SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};

pub trait SignalRuntimeRefBase<R>: 'static where R: Runtime {
    /// Returns a bool to indicate if the signal was emitted or not on the current instant.
    fn is_emitted(&self) -> bool;

    /// Resets the signal at the beginning of each instant.
    fn reset(&mut self);

    fn reset_box(mut self: Box<Self>) {
        (*self).reset();
    }

    /// Exececutes all the continuations found in the vector `self.present_works`.
    fn execute_present_works(&mut self, runtime: &mut R);

    fn execute_present_works_box(mut self: Box<Self>, runtime: &mut R) {
        (*self).execute_present_works(runtime);
    }
}

pub trait SignalRuntimeRefSt: SignalRuntimeRefBase<SingleThreadRuntime> {
    /// Calls `c` at the first cycle where the signal is present.
    fn on_signal<C>(&mut self, runtime: &mut SingleThreadRuntime, c: C)
        where C: ContinuationSt<()>;
    
    /// Calls `c` only if the signal is present during this cycle.
    fn on_signal_present<C>(&mut self, runtime: &mut SingleThreadRuntime, c: C)
        where C: ContinuationSt<()>;
}

pub trait SignalRuntimeRefPl: SignalRuntimeRefBase<ParallelRuntime> {
    /// Calls `c` at the first cycle where the signal is present.
    fn on_signal<C>(&mut self, runtime: &mut ParallelRuntime, c: C)
        where C: ContinuationPl<()>;
    
    /// Calls `c` only if the signal is present during this cycle.
    fn on_signal_present<C>(&mut self, runtime: &mut ParallelRuntime, c: C)
        where C: ContinuationPl<()>;
}

pub trait SignalRuntimeRefBaseSt: SignalRuntimeRefBase<SingleThreadRuntime> {}
pub trait SignalRuntimeRefBasePl: SignalRuntimeRefBase<ParallelRuntime> + Send {}

impl<S> SignalRuntimeRefBaseSt for S where S: SignalRuntimeRefBase<SingleThreadRuntime> {}
impl<S> SignalRuntimeRefBasePl for S where S: SignalRuntimeRefBase<ParallelRuntime> + Send {}

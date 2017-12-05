use parallel::{Runtime, Continuation};

pub trait SignalRuntimeRefBase: Send + 'static {
    /// Returns a bool to indicate if the signal was emitted or not on the current instant.
    fn is_emitted(&self) -> bool;

    /// Resets the signal at the beginning of each instant.
    fn reset(&mut self);

    fn reset_box(mut self: Box<Self>) {
        (*self).reset();
    }

    /// Exececutes all the continuations found in the vector `self.present_works`.
    fn execute_present_works(&mut self, runtime: &mut Runtime);

    fn execute_present_works_box(mut self: Box<Self>, runtime: &mut Runtime) {
        (*self).execute_present_works(runtime);
    }
}

pub trait SignalRuntimeRef: SignalRuntimeRefBase {
    /// Calls `c` at the first cycle where the signal is present.
    fn on_signal<C>(&mut self, runtime: &mut Runtime, c: C) where C: Continuation<()>;
    
    /// Calls `c` only if the signal is present during this cycle.
    fn on_signal_present<C>(&mut self, runtime: &mut Runtime, c: C)
        where C: Continuation<()>;
}

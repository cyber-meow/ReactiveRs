use Continuation;

pub trait SignalRuntimeRefBase<R>: 'static {
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

pub trait SignalRuntimeRef<R>: SignalRuntimeRefBase<R> {
    /// Calls `c` at the first cycle where the signal is present.
    fn on_signal<C>(&mut self, runtime: &mut R, c: C) where C: Continuation<R, ()>;
    
    /// Calls `c` only if the signal is present during this cycle.
    fn on_signal_present<C>(&mut self, runtime: &mut R, c: C)
        where C: Continuation<R, ()>;
}

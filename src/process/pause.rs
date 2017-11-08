use {Runtime, Continuation};
use process::{Process, ProcessMut};

/// The process is suspended until next instant.
pub struct Pause<P>(pub(crate) P);

impl<P> Process for Pause<P> where P: Process {
    type Value = P::Value;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.0.call(runtime, next.pause());
    }
}

impl<P> ProcessMut for Pause<P> where P: ProcessMut {
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>,
    {
        self.0.call_mut(
            runtime,
            next.pause().map(|(process, v): (P, P::Value)| (process.pause(), v)));
    }
}

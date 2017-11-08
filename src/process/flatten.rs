use {Runtime, Continuation};
use process::{Process, ProcessMut};

/// Flatten the process when it returns another process to get only the final process.
pub struct Flatten<P>(pub(crate) P);

impl<P> Process for Flatten<P> where P: Process, P::Value: Process {
    type Value = <P::Value as Process>::Value;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let c = |r: &mut Runtime, p: P::Value| p.call(r, next);
        self.0.call(runtime, c);
    }
}

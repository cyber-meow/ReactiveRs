use parallel::{Runtime, Continuation};
use parallel::process::{Process, ProcessMut};

/// Flatten the process when it returns another process to get only the final process.
pub struct Flatten<P>(pub(crate) P);

impl<P> Process for Flatten<P> where P: Process, P::Value: Process {
    type Value = <P::Value as Process>::Value;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let c = |r: &mut Runtime, p: P::Value| {
            r.on_current_instant(Box::new(|r: &mut Runtime, ()| p.call(r, next)));
        };
        self.0.call(runtime, c);
    }
}

impl<P> ProcessMut for Flatten<P> where P: ProcessMut, P::Value: Process {
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self:Sized, C: Continuation<(Self, Self::Value)>
    {
        let c = |r: &mut Runtime, (p_old, p_new): (P, P::Value)| {
            r.on_current_instant(Box::new(
                |r: &mut Runtime, ()| p_new.call(r, next.map(|v| (p_old.flatten(), v)))));
        };
        self.0.call_mut(runtime, c);
    }
}

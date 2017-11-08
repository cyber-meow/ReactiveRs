use {Runtime, Continuation};
use process::{Process, ProcessMut};

/// Chain a computation onto the end of another process.
pub struct AndThen<P, F> { pub(crate) process: P, pub(crate) chain: F }

impl<P1, P2, F> Process for AndThen<P1, F>
    where P1: Process, P2: Process, F: FnOnce(P1::Value) -> P2 + 'static
{
    type Value = P2::Value;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let chain = self.chain;
        let c = |r: &mut Runtime, v: P1::Value| chain(v).call(r, next);
        self.process.call(runtime, c);
    }
}

impl<P1, P2, F> ProcessMut for AndThen<P1, F>
    where P1: ProcessMut, P2: Process, F: FnMut(P1::Value) -> P2 + 'static
{
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        let mut chain = self.chain;
        let c = |r: &mut Runtime, (process, v): (P1, P1::Value)| {
            chain(v).map(|v2| (process.and_then(chain), v2)).call(r, next);
        };
        self.process.call_mut(runtime, c);
    }
}

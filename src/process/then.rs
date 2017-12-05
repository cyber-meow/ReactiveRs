use parallel::{Runtime, Continuation};
use parallel::process::{Process, ProcessMut};

/// Executes the second process while ignoring the returned value of the first process.
pub struct Then<P1, P2> { pub(crate) process: P1, pub(crate) successor: P2 }

impl<P1, P2> Process for Then<P1, P2> where P1: Process, P2: Process {
    type Value = P2::Value;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let successor = self.successor;
        let c = |r: &mut Runtime, _| successor.call(r, next);
        self.process.call(runtime, c);
    }
}

impl<P1, P2> ProcessMut for Then<P1, P2> where P1: ProcessMut, P2: ProcessMut {
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        let successor = self.successor;
        let c = |r: &mut Runtime, (process, _): (P1, _)| {
            successor.call_mut(r, next.map(|(p2, v)| (process.then(p2), v)));
        };
        self.process.call_mut(runtime, c);
    }
}

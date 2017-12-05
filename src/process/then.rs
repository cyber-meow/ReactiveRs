use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

/// Executes the second process while ignoring the returned value of the first process.
pub struct Then<P1, P2> { pub(crate) process: P1, pub(crate) successor: P2 }

impl<P1, P2> Process for Then<P1, P2> where P1: Process, P2: Process {
    type Value = P2::Value;
}

impl<P1, P2> ProcessMut for Then<P1, P2> where P1: ProcessMut, P2: ProcessMut {}

// Implements the traits for the single thread version of the library.

impl<P1, P2> ProcessSt for Then<P1, P2> where P1: ProcessSt, P2: ProcessSt {
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let successor = self.successor;
        let c = |r: &mut SingleThreadRuntime, _| successor.call(r, next);
        self.process.call(runtime, c);
    }
}

impl<P1, P2> ProcessMutSt for Then<P1, P2> where P1: ProcessMutSt, P2: ProcessMutSt {
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let successor = self.successor;
        let c = |r: &mut SingleThreadRuntime, (process, _): (P1, _)| {
            successor.call_mut(r, next.map(|(p2, v)| (process.then(p2), v)));
        };
        self.process.call_mut(runtime, c);
    }
}

// Implements the traits for the parallel version of the library.

impl<P1, P2> ConstraintOnValue for Then<P1, P2> where P1: Process, P2: ProcessPl {
    type T = P2::Value;
}

impl<P1, P2> ProcessPl for Then<P1, P2> where P1: ProcessPl, P2: ProcessPl {
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        let successor = self.successor;
        let c = |r: &mut ParallelRuntime, _| successor.call(r, next);
        self.process.call(runtime, c);
    }
}

impl<P1, P2> ProcessMutPl for Then<P1, P2> where P1: ProcessMutPl, P2: ProcessMutPl {
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let successor = self.successor;
        let c = |r: &mut ParallelRuntime, (process, _): (P1, _)| {
            successor.call_mut(r, next.map(|(p2, v)| (process.then(p2), v)));
        };
        self.process.call_mut(runtime, c);
    }
}

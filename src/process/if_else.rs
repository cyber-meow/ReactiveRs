use runtime::{SingleThreadRuntime, ParallelRuntime};
use continuation::{ContinuationSt, ContinuationPl};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

/// Selects the process to run according to what the previous process returns.
pub struct IfElse<P, P1, P2> {
    pub(crate) process: P,
    pub(crate) if_branch: P1,
    pub(crate) else_branch: P2
}

impl<P, P1, P2, V> Process for IfElse<P, P1, P2>
    where P: Process<Value=bool>, P1: Process<Value=V>, P2: Process<Value=V>
{
    type Value = V;
}

impl<P, P1, P2, V> ProcessMut for IfElse<P, P1, P2>
    where P: ProcessMut<Value=bool>, P1: ProcessMut<Value=V>, P2: ProcessMut<Value=V> {}

// Implements the traits for the single thread version of the library.

impl<P, P1, P2, V> ProcessSt for IfElse<P, P1, P2>
    where P: ProcessSt<Value=bool>, P1: ProcessSt<Value=V>, P2: ProcessSt<Value=V>
{
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let if_branch = self.if_branch;
        let else_branch = self.else_branch;
        let c = |r: &mut SingleThreadRuntime, cond| {
            if cond {
                if_branch.call(r, next);
            } else {
                else_branch.call(r, next);
            }
        };
        self.process.call(runtime, c);
    }
}

impl<P, P1, P2, V> ProcessMutSt for IfElse<P, P1, P2>
    where P: ProcessMutSt<Value=bool>, P1: ProcessMutSt<Value=V>, P2: ProcessMutSt<Value=V>
{
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let if_branch = self.if_branch;
        let else_branch = self.else_branch;
        let c = |r: &mut SingleThreadRuntime, (process, cond): (P, bool)| {
            if cond {
                if_branch.call_mut(
                    r,
                    next.map(|(p, v)| (process.if_else(p, else_branch), v))
                )
            } else {
                else_branch.call_mut(
                    r,
                    next.map(|(p, v)| (process.if_else(if_branch, p), v))
                )
            }
        };
        self.process.call_mut(runtime, c);
    }
}

// Implements the traits for the parallel version of the library.

impl<P, P1, P2, V> ConstraintOnValue for IfElse<P, P1, P2>
    where P: Process<Value=bool>, P1: Process<Value=V>, P2: Process<Value=V>, V: Send + Sync
{
    type T = V;
}

// Note that we must use `<T=..>` instead of `<Value=..>` because in the definition of 
// `ProcessPl` we have <Value = <Self::ConstraintOnValue>::T>.

impl<P, P1, P2, V> ProcessPl for IfElse<P, P1, P2>
    where P: ProcessPl<T=bool>, P1: ProcessPl<T=V>, P2: ProcessPl<T=V>, V: Send + Sync
{
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        let if_branch = self.if_branch;
        let else_branch = self.else_branch;
        let c = |r: &mut ParallelRuntime, cond| {
            if cond {
                if_branch.call(r, next);
            } else {
                else_branch.call(r, next);
            }
        };
        self.process.call(runtime, c);
    }
}

impl<P, P1, P2, V> ProcessMutPl for IfElse<P, P1, P2>
    where P: ProcessMutPl<T=bool>, P1: ProcessMutPl<T=V>, P2: ProcessMutPl<T=V>, V: Send + Sync
{
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let if_branch = self.if_branch;
        let else_branch = self.else_branch;
        let c = |r: &mut ParallelRuntime, (process, cond): (P, bool)| {
            if cond {
                if_branch.call_mut(
                    r,
                    next.map(|(p, v)| (process.if_else(p, else_branch), v))
                )
            } else {
                else_branch.call_mut(
                    r,
                    next.map(|(p, v)| (process.if_else(if_branch, p), v))
                )
            }
        };
        self.process.call_mut(runtime, c);
    }
}

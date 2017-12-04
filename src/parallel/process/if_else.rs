use parallel::{Runtime, Continuation};
use parallel::process::{Process, ProcessMut};

/// Selects the process to run according to what the previous process returns.
pub struct IfElse<P, P1, P2> {
    pub(crate) process: P,
    pub(crate) if_branch: P1,
    pub(crate) else_branch: P2
}

impl<P, P1, P2, V> Process for IfElse<P, P1, P2>
    where P: Process<Value=bool>, P1: Process<Value=V>, P2: Process<Value=V>, V: Send + Sync
{
    type Value = V;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let if_branch = self.if_branch;
        let else_branch = self.else_branch;
        let c = |r: &mut Runtime, cond| {
            if cond {
                if_branch.call(r, next);
            } else {
                else_branch.call(r, next);
            }
        };
        self.process.call(runtime, c);
    }
}

impl<P, P1, P2, V> ProcessMut for IfElse<P, P1, P2>
    where P: ProcessMut<Value=bool>,
          P1: ProcessMut<Value=V>, 
          P2: ProcessMut<Value=V>,
          V: Send + Sync,
{
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        let if_branch = self.if_branch;
        let else_branch = self.else_branch;
        let c = |r: &mut Runtime, (process, cond): (P, bool)| {
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

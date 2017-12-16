use std::sync::{Arc, Mutex};

use runtime::ParallelRuntime;
use continuation::{Continuation, ContinuationPl};
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

use process::{join_all, JoinAll};

// Implements the traits for the parallel version of the library.

impl<P> ConstraintOnValue for JoinAll<P> where P: ProcessPl {
    type T = Vec<P::Value>;
}    

impl<P> ProcessPl for JoinAll<P> where P: ProcessPl {
    fn call<C>(mut self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        if self.0.is_empty() {
            next.call(runtime, Vec::new());
        } else {
            let joint_point = Arc::new(Mutex::new(JoinPoint::new(self.0.len(), next)));
            while let Some(p) = self.0.pop() {
                let p_id = self.0.len();
                let joint_point = joint_point.clone();
                let c = move |r: &mut ParallelRuntime, ()| {
                    p.call(r, move |r: &mut ParallelRuntime, v|
                        joint_point.lock().unwrap().call_ref(r, (v, p_id)));
                };
                runtime.on_current_instant(Box::new(c));
            };
        }
    }
}

impl<P> ProcessMutPl for JoinAll<P> where P: ProcessMutPl {
    fn call_mut<C>(mut self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let mut_next = next.map(|pvs: Vec<(P, P::Value)>| {
            let (ps, vs): (Vec<_>, Vec<_>) = pvs.into_iter().unzip();
            (join_all(ps), vs)
        });
        if self.0.is_empty() {
            mut_next.call(runtime, Vec::new());
        } else {
            let joint_point = Arc::new(Mutex::new(JoinPoint::new(self.0.len(), mut_next)));
            while let Some(p) = self.0.pop() {
                let p_id = self.0.len();
                let joint_point = joint_point.clone();
                let c = move |r: &mut ParallelRuntime, ()| {
                    p.call_mut(r, move |r: &mut ParallelRuntime, p_v|
                        joint_point.lock().unwrap().call_ref(r, (p_v, p_id)));
                };
                runtime.on_current_instant(Box::new(c));
            };
        }
    }
}

/// Used by `JoinAll` as a barrier for multiple processes.
struct JoinPoint<V, C> {
    counter: usize,
    values: Vec<Option<V>>,
    continuation: Option<C>,
}

impl<V, C> JoinPoint<V, C> where C: ContinuationPl<Vec<V>> {
    fn new(num_procs: usize, continuation: C) -> Self {
        JoinPoint {
            counter: 0,
            values: (0..num_procs).map(|_| None).collect(),
            continuation: Some(continuation),
        }
    }
    
    fn call_ref(&mut self, runtime: &mut ParallelRuntime, (value, p_id): (V, usize)) {
        assert!(self.values[p_id].is_none());
        self.values[p_id] = Some(value);
        self.counter += 1;
        if self.counter == self.values.len() {
            let values = self.values.iter_mut().map(|v| v.take().unwrap()).collect();
            self.continuation.take().unwrap().call(runtime, values);
        }
    }
}

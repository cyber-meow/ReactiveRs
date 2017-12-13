use std::sync::{Arc, Mutex};
use either::{Either, Left, Right};

use runtime::ParallelRuntime;
use continuation::ContinuationPl;
use process::{ProcessPl, ProcessMutPl, ConstraintOnValue};

use process::Join;

// Implements the traits for the parallel version of the library.

impl<P1, P2> ConstraintOnValue for Join<P1, P2> where P1: ProcessPl, P2: ProcessPl {
    type T = (P1::Value, P2::Value);
}    

impl<P1, P2> ProcessPl for Join<P1, P2> where P1: ProcessPl, P2: ProcessPl {
    fn call<C>(self, runtime: &mut ParallelRuntime, next: C)
        where C: ContinuationPl<Self::Value>
    {
        let joint_point = Arc::new(Mutex::new(JoinPoint::new(next)));
        let joint_point2 = joint_point.clone();
        let (proc1, proc2) = (self.0, self.1);
        let c1 = |r: &mut ParallelRuntime, ()| {
            proc1.call(
                r,
                move |r: &mut ParallelRuntime, v|
                    joint_point.lock().unwrap().call_ref(r, Left(v)));
        };
        let c2 = |r: &mut ParallelRuntime, ()| {
            proc2.call(
                r,
                move |r: &mut ParallelRuntime, v|
                    joint_point2.lock().unwrap().call_ref(r, Right(v)));
        };
        runtime.on_current_instant(Box::new(c1));
        runtime.on_current_instant(Box::new(c2));
    }
}

impl<P1, P2> ProcessMutPl for Join<P1, P2> where P1: ProcessMutPl, P2: ProcessMutPl {
    fn call_mut<C>(self, runtime: &mut ParallelRuntime, next: C)
        where Self: Sized, C: ContinuationPl<(Self, Self::Value)>
    {
        let mut_next = next.map(
            |((p1, v1), (p2, v2)): ((P1, P1::Value), (P2, P2::Value))|
            (p1.join(p2), (v1, v2))
        );
        let joint_point = Arc::new(Mutex::new(JoinPoint::new(mut_next)));
        let joint_point2 = joint_point.clone();
        let (proc1, proc2) = (self.0, self.1);
        let c1 = |r: &mut ParallelRuntime, ()| {
            proc1.call_mut(
                r,
                move |r: &mut ParallelRuntime, p_v|
                    joint_point.lock().unwrap().call_ref(r, Left(p_v)));
        };
        let c2 = |r: &mut ParallelRuntime, ()| {
            proc2.call_mut(
                r,
                move |r: &mut ParallelRuntime, p_v|
                    joint_point2.lock().unwrap().call_ref(r, Right(p_v)));
        };
        runtime.on_current_instant(Box::new(c1));
        runtime.on_current_instant(Box::new(c2));
    }
}

/// Used by `Join` as a barrier for two processes.
struct JoinPoint<V1, V2, C> {
    counter: i32,
    values: (Option<V1>, Option<V2>),
    continuation: Option<C>,
}

impl<V1, V2, C> JoinPoint<V1, V2, C> where C: ContinuationPl<(V1, V2)> {
    fn new(continuation: C) -> Self where {
        JoinPoint {
            counter: 0,
            values: (None, None),
            continuation: Some(continuation),
        }
    }

    fn call_ref(&mut self, runtime: &mut ParallelRuntime, value: Either<V1, V2>) {
        match value {
            Left(value1) => {
                assert!(self.values.0.is_none());
                self.values.0 = Some(value1);
                self.counter += 1;
            },
            Right(value2) => {
                assert!(self.values.1.is_none());
                self.values.1 = Some(value2);
                self.counter += 1;
            }
        }
        if self.counter == 2 {
            let values = (self.values.0.take().unwrap(), self.values.1.take().unwrap());
            self.continuation.take().unwrap().call(runtime, values);
        }
    }
}

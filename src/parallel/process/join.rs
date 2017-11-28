use std::sync::{Arc, Mutex};
use either::{Either, Left, Right};

use parallel::{Runtime, Continuation};
use parallel::process::{Process, ProcessMut};

/// Parallel composition of two processes.
pub struct Join<P1, P2>(pub(crate) P1, pub(crate) P2);

impl<P1, P2> Process for Join<P1, P2> where P1: Process, P2: Process {
    type Value = (P1::Value, P2::Value);
    
    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let joint_point = Arc::new(Mutex::new(JoinPoint::new(next)));
        let joint_point2 = joint_point.clone();
        let (proc1, proc2) = (self.0, self.1);
        let c1 = |r: &mut Runtime, ()| {
            proc1.call(
                r,
                move |r: &mut Runtime, v|
                    joint_point.lock().unwrap().call_ref(r, Left(v)));
        };
        let c2 = |r: &mut Runtime, ()| {
            proc2.call(
                r,
                move |r: &mut Runtime, v|
                    joint_point2.lock().unwrap().call_ref(r, Right(v)));
        };
        runtime.on_current_instant(Box::new(c1));
        runtime.on_current_instant(Box::new(c2));
    }
}

impl<P1, P2> ProcessMut for Join<P1, P2> where P1: ProcessMut, P2: ProcessMut {
    fn call_mut<C>(self, runtime: &mut Runtime, next: C)
        where Self: Sized, C: Continuation<(Self, Self::Value)>
    {
        let mut_next = next.map(
            |((p1, v1), (p2, v2)): ((P1, P1::Value), (P2, P2::Value))|
            (p1.join(p2), (v1, v2))
        );
        let joint_point = Arc::new(Mutex::new(JoinPoint::new(mut_next)));
        let joint_point2 = joint_point.clone();
        let (proc1, proc2) = (self.0, self.1);
        let c1 = |r: &mut Runtime, ()| {
            proc1.call_mut(
                r,
                move |r: &mut Runtime, p_v|
                    joint_point.lock().unwrap().call_ref(r, Left(p_v)));
        };
        let c2 = |r: &mut Runtime, ()| {
            proc2.call_mut(
                r,
                move |r: &mut Runtime, p_v|
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

impl<V1, V2, C> JoinPoint<V1, V2, C> {
    fn new(continuation: C) -> Self where C: Continuation<(V1, V2)> {
        JoinPoint {
            counter: 0,
            values: (None, None),
            continuation: Some(continuation),
        }
    }
    
    fn call_ref(&mut self, runtime: &mut Runtime, value: Either<V1, V2>) 
        where V1: 'static, V2: 'static, C: Continuation<(V1, V2)>   
    {
        match value {
            Left(value1) => {
                assert_eq!(self.values.0.is_none(), true);
                self.values.0 = Some(value1);
                self.counter += 1;
            },
            Right(value2) => {
                assert_eq!(self.values.1.is_none(), true);
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

use std::rc::Rc;
use std::cell::RefCell;
use either::{Either, Left, Right};

use runtime::SingleThreadRuntime;
use continuation::ContinuationSt;
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};

/// Parallel composition of two processes.
pub struct Join<P1, P2>(pub(crate) P1, pub(crate) P2);

impl<P1, P2> Process for Join<P1, P2> where P1: Process, P2: Process {
    type Value = (P1::Value, P2::Value);
}

impl<P1, P2> ProcessMut for Join<P1, P2> where P1: ProcessMut, P2: ProcessMut {}

// Implements the traits for the single thread version of the library.

impl<P1, P2> ProcessSt for Join<P1, P2> where P1: ProcessSt, P2: ProcessSt {
    fn call<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        let joint_point = Rc::new(RefCell::new(JoinPoint::new(next)));
        let joint_point2 = joint_point.clone();
        self.0.call(
            runtime,
            move |r: &mut SingleThreadRuntime, v|
                joint_point.borrow_mut().call_ref(r, Left(v)));
        self.1.call(
            runtime,
            move |r: &mut SingleThreadRuntime, v|
                joint_point2.borrow_mut().call_ref(r, Right(v)));
    }
}

impl<P1, P2> ProcessMutSt for Join<P1, P2> where P1: ProcessMutSt, P2: ProcessMutSt {
    fn call_mut<C>(self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let mut_next = next.map(
            |((p1, v1), (p2, v2)): ((P1, P1::Value), (P2, P2::Value))|
            (p1.join(p2), (v1, v2))
        );
        let joint_point = Rc::new(RefCell::new(JoinPoint::new(mut_next)));
        let joint_point2 = joint_point.clone();
        self.0.call_mut(
            runtime,
            move |r: &mut SingleThreadRuntime, p_v|
                joint_point.borrow_mut().call_ref(r, Left(p_v)));
        self.1.call_mut(
            runtime,
            move |r: &mut SingleThreadRuntime, p_v|
                joint_point2.borrow_mut().call_ref(r, Right(p_v)));
    }
}

/// Used by `Join` as a barrier for two processes.
struct JoinPoint<V1, V2, C> {
    counter: i32,
    values: (Option<V1>, Option<V2>),
    continuation: Option<C>,
}

impl<V1, V2, C> JoinPoint<V1, V2, C> {
    fn new(continuation: C) -> Self where C: ContinuationSt<(V1, V2)> {
        JoinPoint {
            counter: 0,
            values: (None, None),
            continuation: Some(continuation),
        }
    }
    
    fn call_ref(&mut self, runtime: &mut SingleThreadRuntime, value: Either<V1, V2>) 
        where V1: 'static, V2: 'static, C: ContinuationSt<(V1, V2)>   
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

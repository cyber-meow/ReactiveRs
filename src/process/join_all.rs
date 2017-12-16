use std::rc::Rc;
use std::cell::RefCell;

use runtime::SingleThreadRuntime;
use continuation::{Continuation, ContinuationSt};
use process::{Process, ProcessMut, ProcessSt, ProcessMutSt};

/// Creates a process that executes a collection of processes in parallel and collects
/// the results into a destination `Vec<T>` in the same oreder as they were provided.  
/// However, since the Rust's type system force all the values in a vector to take
/// the same type and most of the processes have different type from each other,
/// this may not be very useful.
/// (What's worse, a process cannot be made into a trait object.)
pub fn join_all<I>(i: I) -> JoinAll<I::Item> where I: IntoIterator, I::Item: Process {
    JoinAll(i.into_iter().collect())
}

/// A process which takes a list of processes and terminates with a  vector of the
/// completed values.
pub struct JoinAll<P>(pub(crate) Vec<P>);

impl<P> Process for JoinAll<P> where P: Process {
    type Value = Vec<P::Value>;
}

impl <P> ProcessMut for JoinAll<P> where P: ProcessMut {}

// Implements the traits for the single thread version of the library.

impl <P> ProcessSt for JoinAll<P> where P: ProcessSt {
    fn call<C>(mut self, runtime: &mut SingleThreadRuntime, next: C)
        where C: ContinuationSt<Self::Value>
    {
        if self.0.is_empty() {
            next.call(runtime, Vec::new());
        } else {
            let joint_point = Rc::new(RefCell::new(JoinPoint::new(self.0.len(), next)));
            while let Some(p) = self.0.pop() {
                let p_id = self.0.len();
                let joint_point = joint_point.clone();
                p.call(
                    runtime,
                    move |r: &mut SingleThreadRuntime, v|
                        joint_point.borrow_mut().call_ref(r, (v, p_id)));
            }
        }
    }
}

impl<P> ProcessMutSt for JoinAll<P> where P: ProcessMutSt {
    fn call_mut<C>(mut self, runtime: &mut SingleThreadRuntime, next: C)
        where Self: Sized, C: ContinuationSt<(Self, Self::Value)>
    {
        let mut_next = next.map(|pvs: Vec<(P, P::Value)>| {
            let (ps, vs): (Vec<_>, Vec<_>) = pvs.into_iter().unzip();
            (join_all(ps), vs)
        });
        if self.0.is_empty() {
            mut_next.call(runtime, Vec::new());
        } else {
            let joint_point = Rc::new(RefCell::new(JoinPoint::new(self.0.len(), mut_next)));
            while let Some(p) = self.0.pop() {
                let p_id = self.0.len();
                let joint_point = joint_point.clone();
                p.call_mut(
                    runtime,
                    move |r: &mut SingleThreadRuntime, p_v|
                        joint_point.borrow_mut().call_ref(r, (p_v, p_id)));
            }
        }
    }
}

/// Used by `JoinAll` as a barrier for multiple processes.
struct JoinPoint<V, C> {
    counter: usize,
    values: Vec<Option<V>>,
    continuation: Option<C>,
}

impl<V, C> JoinPoint<V, C> where C: ContinuationSt<Vec<V>> {
    fn new(num_procs: usize, continuation: C) -> Self {
        JoinPoint {
            counter: 0,
            values: (0..num_procs).map(|_| None).collect(),
            continuation: Some(continuation),
        }
    }
    
    fn call_ref(&mut self, runtime: &mut SingleThreadRuntime, (value, p_id): (V, usize)) {
        assert!(self.values[p_id].is_none());
        self.values[p_id] = Some(value);
        self.counter += 1;
        if self.counter == self.values.len() {
            let values = self.values.iter_mut().map(|v| v.take().unwrap()).collect();
            self.continuation.take().unwrap().call(runtime, values);
        }
    }
}

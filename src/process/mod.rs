use std::rc::Rc;
use std::cell::{ RefCell };
use either::{ Either, Left, Right };

pub mod process_mut;

use { Runtime, Continuation };

/// A reactive process.
pub trait Process: 'static {
    /// The value created by the process.
    type Value;

    /// Executes the reactive process in the runtime, calls `next` with the resulting value.
    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value>;

    /// Suspends the execution of a process until next instant.
    fn pause(self) -> Pause<Self> where Self: Sized {
        Pause(self)
    }

    /// Apply a function to the value returned by the process before passing it to
    /// its continuation.
    fn map<F, V>(self, map: F) -> 
        Map<Self, F> where Self: Sized, F: FnOnce(Self::Value) -> V + 'static
    {
        Map { process: self, map }
    }

    /// Flatten the execution of a process when its returned value is itself another process.
    fn flatten(self) -> Flatten<Self> where Self: Sized, Self::Value: Process {
        Flatten(self)
    }

    /// Chain another process after the exectution of one process (like the `bind` for a monad).
    fn and_then<F, P>(self, chain: F) ->
        AndThen<Self, F> where Self: Sized, F: FnOnce(Self::Value) -> P + 'static, P: Process
    {
        AndThen { process: self, chain }
    }

    /// Execute two processes in parallel. 
    fn join<P>(self, proc2: P) -> Join<Self, P> where Self: Sized, P: Process {
        Join(self, proc2)
    }

    // TODO: add combinators
}

/// Execute a process in a newly created runtime and return its value.
pub fn execute_process<P>(p: P) -> P::Value where P: Process {
    let mut runtime = Runtime::new();
    let res: Rc<RefCell<Option<P::Value>>> = Rc::new(RefCell::new(None));
    let res2 = res.clone();
    let c = move |_: &mut Runtime, v| { *res2.borrow_mut() = Some(v); drop(res2) };
    runtime.on_current_instant(Box::new(|r: &mut Runtime, _| p.call(r, c)));
    runtime.execute();
    Rc::try_unwrap(res).map_err(|_| ()).unwrap().into_inner().unwrap()
}

/// A process that returns a value of type V.
pub struct Value<V>(V);

impl<V> Process for Value<V> where V: 'static {
    type Value = V;
    
    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<V> {
        next.call(runtime, self.0);
    }
}

/// Create a new process that returns the value v immediately.
pub fn value<V>(v: V) -> Value<V> {
    Value(v)
}

/// A process that applies a function to the returned value of another process.
pub struct Map<P, F> { process: P, map: F }

impl<P, F, V> Process for Map<P, F> 
    where P: Process, F: FnOnce(P::Value) -> V + 'static
{
    type Value = V;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.process.call(runtime, next.map(self.map));
    }
}

/// The process is suspended until next instant.
pub struct Pause<P>(P);

impl<P> Process for Pause<P> where P: Process {
    type Value = P::Value;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        self.0.call(runtime, next.pause());
    }
}

/// Flatten the process when it returns another process to get only the final process.
pub struct Flatten<P>(P);

impl<P> Process for Flatten<P> where P: Process, P::Value: Process {
    type Value = <P::Value as Process>::Value;

    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let c = |r: &mut Runtime, p: P::Value| p.call(r, next);
        self.0.call(runtime, c);
    }
}

/// Chain a computation onto the end of another process.
pub struct AndThen<P, F> { process: P, chain: F }

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

/// Parallel composition of two processes.
pub struct Join<P1, P2>(P1, P2);

impl<P1, P2> Process for Join<P1, P2> where P1: Process, P2: Process {
    type Value = (P1::Value, P2::Value);
    
    fn call<C>(self, runtime: &mut Runtime, next: C) where C: Continuation<Self::Value> {
        let joint_point = Rc::new(RefCell::new(JoinPoint::new(next)));
        let joint_point2 = joint_point.clone();
        self.0.call(
            runtime,
            move |r: &mut Runtime, v| joint_point.borrow_mut().call_ref(r, Left(v)));
        self.1.call(
            runtime,
            move |r: &mut Runtime, v| joint_point2.borrow_mut().call_ref(r, Right(v)));
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

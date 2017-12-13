extern crate reactive;

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use reactive::runtime::{Runtime, SingleThreadRuntime};
use reactive::runtime::{ParallelRuntime, ParallelRuntimeCollection};
use reactive::continuation::Continuation;

#[test]
fn continuation42_s() {
    let val = Rc::new(RefCell::new(7));
    let val2 = val.clone();
    let c = move |_: &mut SingleThreadRuntime, _| *val2.borrow_mut() = 42;
    let c = c.pause();
    let mut runtime = SingleThreadRuntime::new();
    c.call(&mut runtime, ());
    assert!(runtime.instant());
    assert_eq!(*val.borrow(), 7);
    assert!(!runtime.instant());
    assert_eq!(*val.borrow(), 42);
}

#[test]
fn continuation42_p() {
    let val = Arc::new(Mutex::new(7));
    let val2 = val.clone();
    let c = move |_: &mut ParallelRuntime, _| *val2.lock().unwrap() = 42;
    let c = c.pause().map(|()| println!("hello")).pause().pause();
    let mut runtime_collection = ParallelRuntimeCollection::new(2);
    runtime_collection.register_work(Box::new(c));
    runtime_collection.execute(|| ());
    assert_eq!(*val.lock().unwrap(), 42);
}

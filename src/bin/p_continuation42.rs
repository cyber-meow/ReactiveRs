extern crate reactive;

use reactive::continuation::Continuation;
use reactive::runtime::{ParallelRuntime, ParallelRuntimeCollection};

fn main() {
    let c = |_: &mut ParallelRuntime, _| println!("{}", 42);
    let c = c.pause().map(|()| println!("hello")).pause().pause();
    let mut runtime_collection = ParallelRuntimeCollection::new(2);
    runtime_collection.register_work(Box::new(c));
    runtime_collection.execute();
}

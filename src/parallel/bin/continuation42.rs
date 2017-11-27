extern crate reactive;

use reactive::parallel::Continuation;
use reactive::parallel::{Runtime, RuntimeCollection};

fn main() {
    let c = |_: &mut Runtime, _| println!("{}", 42);
    let c = c.pause().map(|()| println!("hello")).pause().pause();
    let mut runtime_collection = RuntimeCollection::new(2);
    runtime_collection.register_work(Box::new(c));
    runtime_collection.execute();
}

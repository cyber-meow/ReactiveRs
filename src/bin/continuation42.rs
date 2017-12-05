extern crate reactive;

use reactive::runtime::{Runtime, SingleThreadRuntime};
use reactive::continuation::Continuation;

fn main() {
    let c = |_: &mut SingleThreadRuntime, _| println!("{}", 42);
    let c = c.pause();
    let mut runtime = SingleThreadRuntime::new();
    c.call(&mut runtime, ());
    runtime.instant();
    runtime.instant();
}

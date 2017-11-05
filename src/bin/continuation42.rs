extern crate reactive;

use reactive::runtime::Runtime;
use reactive::continuation::Continuation;

fn main() {
    let c = |_: &mut Runtime, _| println!("{}", 42);
    let c = c.pause();
    let mut runtime = Runtime::new();
    c.call(&mut runtime, ());
    runtime.instant();
    runtime.instant();
}

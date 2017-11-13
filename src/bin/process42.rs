extern crate reactive;

use reactive::Process;
use reactive::process::{value, execute_process};

fn main() {
    let p = value(42);
    let p = p.pause().pause();
    let p = p.map(|v| { println!("{:?}", v) });
    execute_process(p);
}

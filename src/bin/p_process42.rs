extern crate reactive;

use reactive::process::{Process, value, execute_process_parallel};

fn main() {
    let p = value(42);
    let p = p.pause().pause();
    let p = p.map(|v| { println!("{:?}", v) });
    execute_process_parallel(p, 3);
}

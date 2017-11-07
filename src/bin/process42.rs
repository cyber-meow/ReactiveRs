extern crate reactive;

use reactive::Process;
use reactive::process::{ value, execute_process };

fn main() {
    let p = value(42);
    let p = p.map(|v| { println!("{:?}", v) });
    let p = p.pause().pause();
    execute_process(p);
}

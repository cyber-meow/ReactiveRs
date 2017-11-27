extern crate reactive;

use reactive::parallel::Process;
use reactive::parallel::process::{value, execute_process};

fn main() {
    let p = value(42);
    let p = p.pause().pause();
    let p = p.map(|v| { println!("{:?}", v) });
    execute_process(p, 3);
}

extern crate reactive;

use reactive::{Process, ProcessMut};
use reactive::process::{value, execute_process};
use reactive::process::LoopStatus::{Continue, Exit};

fn main () {
    let mut counter = 0;
    let while_cond = move |n| {
        if counter == n {
            Exit(true)
        } else if counter%2 == 0 {
            counter += 1;
            println!("pair number");
            Continue
        } else {
            counter += 1;
            println!("odd number");
            Continue
        }
    };
    let p1 = value(100)
             .map(while_cond)
             .pause()
             .while_proc()
             .if_else(
                 value(()).pause().map(|()| println!("true")),
                 value(()).map(|()| println!("false")));
    execute_process(p1);
}

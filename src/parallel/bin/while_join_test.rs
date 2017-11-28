extern crate reactive;

use reactive::parallel::{Process, ProcessMut};
use reactive::parallel::process::{value, execute_process};
use reactive::parallel::process::LoopStatus::{Continue, Exit};

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
    let p1 = value(10)
             .map(while_cond)
             .pause()
             .while_proc();
    let p2 = value(())
             .pause()
             .map(|()| println!("hello"))
             .pause()
             .map(|()| println!("hello"))
             .pause()
             .map(|()| println!("hello"))
             .pause()
             .map(|()| println!("hello"))
             .pause()
             .map(|()| println!("hello"));
    execute_process(p1.join(p2), 2);
}

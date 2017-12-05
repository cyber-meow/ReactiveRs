extern crate reactive;

use reactive::process::{Process, ProcessMut};
// use reactive::process::execute_process;
use reactive::process::{value_proc, execute_process_parallel};
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
    let p1 = value_proc(10)
             .map(while_cond)
             .pause()
             .while_proc();
    let p2 = value_proc(())
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
    // Just to say that a same process can be executed in the two kinds of runtime
    // as long as it's `Send` and `Sync`, can't demonstrate here because the process is
    // consumed once executed.
    // execute_process(p1.join(p2));
    execute_process_parallel(p1.join(p2), 2);
}

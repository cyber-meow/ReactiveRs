extern crate reactive;

use std::rc::Rc;
use std::cell::RefCell;

use reactive::{Process, ProcessMut};
use reactive::process::{value, execute_process};
use reactive::process::LoopStatus::{Continue, Exit};

use reactive::signal::MpmcSignal;

fn main () {
    let gather = |x: isize, xs: &mut Vec<isize>| xs.push(x);
    let s = MpmcSignal::new(Vec::new(), gather);
    let counter = Rc::new(RefCell::new(0));
    let s_clone = s.clone();
    let counter_clone = counter.clone();
    let decide_emit = move |n| {
        let v = *counter_clone.borrow();
        value(v%2==0)
        .if_else(s_clone.emit(v), value(()))
        .map(move |()| n)
    };
    let while_p1 = move |n| {
        if *counter.borrow() == n {
            Exit(())
        } else {
            *counter.borrow_mut() += 1;
            Continue
        }
    };
    let while_p2 = |v| {
        println!("Get {:?} at last instant.", v);
        if v == vec![80] {
            println!("No more await.");
            Exit(())
        } else {
            Continue
        }
    };
    let p1 = value(100)
             .and_then(decide_emit)
             .map(while_p1)
             .pause()
             .while_proc();
    let p2 = s.await()
             .map(while_p2)
             .while_proc();
    execute_process(p1.join(p2));
}

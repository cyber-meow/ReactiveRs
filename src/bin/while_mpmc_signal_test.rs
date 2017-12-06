extern crate reactive;

use std::rc::Rc;
use std::cell::RefCell;

use reactive::process::{Process, ProcessMut, value_proc, execute_process};
use reactive::process::LoopStatus::{Continue, Exit};
use reactive::signal::ValuedSignal;
use reactive::signal::single_thread::MpmcSignal;

fn main () {
    let gather = |x: isize, xs: &mut Vec<isize>| xs.push(x);
    let s = MpmcSignal::new(Vec::new(), gather);
    let counter = Rc::new(RefCell::new(0));
    let s_clone = s.clone();
    let counter_clone = counter.clone();
    let decide_emit = move |n| {
        let v = *counter_clone.borrow();
        value_proc(v%2==0)
        .if_else(s_clone.emit(v), value_proc(()))
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
    let p1 = value_proc(100)
             .and_then(decide_emit)
             .map(while_p1)
             .pause()
             .while_proc();
    let p2 = s.await()
             .map(while_p2)
             .while_proc();
    execute_process(p1.join(p2));
}

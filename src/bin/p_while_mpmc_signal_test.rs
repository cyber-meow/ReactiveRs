extern crate reactive;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};


use reactive::process::{Process, ProcessMut, value_proc, execute_process_parallel};
use reactive::process::LoopStatus::{Continue, Exit};
use reactive::signal::MpmcSignal;
use reactive::signal::parallel::MpmcSignalImpl;

fn main () {
    let gather = |x: usize, xs: &mut Vec<usize>| xs.push(x);
    let s = MpmcSignalImpl::new(Vec::new(), gather);
    let counter = Arc::new(AtomicUsize::new(0));
    let s_clone = s.clone();
    let counter_clone = counter.clone();
    let decide_emit = move |n| {
        let v = counter_clone.load(Ordering::SeqCst);
        value_proc(v%2==0)
        .if_else(s_clone.emit(v), value_proc(()))
        .map(move |()| n)
    };
    let while_p1 = move |n| {
        if counter.load(Ordering::SeqCst) == n {
            Exit(())
        } else {
            counter.fetch_add(1, Ordering::SeqCst);
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
    let p1 = value_proc(1000)
             .and_then(decide_emit)
             .map(while_p1)
             .pause()
             .while_proc();
    let p2 = s.await()
             .map(while_p2)
             .while_proc();
    execute_process_parallel(p1.join(p2), 2);
}

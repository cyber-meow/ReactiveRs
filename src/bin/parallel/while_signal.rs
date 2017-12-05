extern crate reactive;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use reactive::parallel::{Process, ProcessMut};
use reactive::parallel::process::{value, execute_process};
use reactive::parallel::process::LoopStatus::{Continue, Exit};

use reactive::parallel::signal::MpmcSignal;

fn main () {
    let gather = |x: usize, xs: &mut Vec<usize>| xs.push(x);
    let s = MpmcSignal::new(Vec::new(), gather);
    let counter = Arc::new(AtomicUsize::new(0));
    let s_clone = s.clone();
    let counter_clone = counter.clone();
    let decide_emit = move |n| {
        let v = counter_clone.load(Ordering::SeqCst);
        value(v%2==0)
        .if_else(s_clone.emit(v), value(()))
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
        if v == vec![800] {
            println!("No more await.");
            Exit(())
        } else {
            Continue
        }
    };
    let p1 = value(1000)
             .and_then(decide_emit)
             .map(while_p1)
             .pause()
             .while_proc();
    let p2 = s.await()
             .map(while_p2)
             .while_proc();
    execute_process(p1.join(p2), 2);
}

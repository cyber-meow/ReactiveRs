extern crate reactive;

use reactive::process::{Process, value_proc, execute_process};
use reactive::signal::{Signal, ValuedSignal};
use reactive::signal::single_thread::MpmcSignal;

fn main () {
    let gather = |x: isize, xs: &mut Vec<isize>| xs.push(x);
    let s = MpmcSignal::new(Vec::new(), gather);
    let p1 = s.emit(3).pause();
    let p2 = s.await_immediate().map(|()| println!("receive s"));
    let print_s_value = |x| println!("The value of s is {:?}", x);
    let p3 = s.await().map(print_s_value);
    let p4 = s.emit(5).pause().then(s.emit(7));
    let p5 = value_proc(()).pause().pause().then(
                s.present_else(
                value_proc(()).map(|()| println!("present")),
                value_proc(()).map(|()| println!("abscent"))));
    execute_process(p1.join(p2).join(p3).join(p4).join(p5));
}

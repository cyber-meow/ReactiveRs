extern crate reactive;

use reactive::{Process, ProcessMut};
use reactive::process::{value, execute_process};
use reactive::signal::{Signal, PureSignal};

fn main () {
    let s = PureSignal::new();
    let p1 = s.emit().pause().loop_proc();
    let receive_cl = |()| println!("s received");
    let p2= s.await_immediate().map(receive_cl).pause().loop_proc();
    let present_cl = |()| println!("present");
    let abscent_cl = |()| println!("not present");
    let present_s = value(()).map(present_cl).pause();
    let abscent_s = value(()).map(abscent_cl);
    let p3 = s.present_else(present_s, abscent_s).loop_proc();
    execute_process(p1.join(p2).join(p3));
}

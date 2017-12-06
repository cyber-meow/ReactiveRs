extern crate reactive;

use reactive::process::{Process, ProcessMut, value_proc, execute_process};
use reactive::signal::{Signal, PureSignal};
use reactive::signal::single_thread::PureSignalImpl;

fn main () {
    let s = PureSignalImpl::new();
    let p1 = s.emit().pause().loop_proc();
    let receive_cl = |()| println!("s received");
    let p2= s.await_immediate().map(receive_cl).pause().loop_proc();
    let present_cl = |()| println!("present");
    let abscent_cl = |()| println!("not present");
    let present_s = value_proc(()).map(present_cl).pause();
    let abscent_s = value_proc(()).map(abscent_cl);
    let p3 = s.present_else(present_s, abscent_s).loop_proc();
    execute_process(p1.join(p2).join(p3));
}

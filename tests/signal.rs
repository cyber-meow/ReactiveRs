extern crate reactive;

use reactive::process::{Process, ProcessMut, value_proc};
use reactive::process::{execute_process, execute_process_parallel};
use reactive::process::LoopStatus::{Continue, Exit};
use reactive::signal::{Signal, PureSignal, ValuedSignal};
use reactive::signal::single_thread::{PureSignalSt, MpmcSignalSt, MpscSignalSt, SpmcSignalSt};
use reactive::signal::parallel::{PureSignalPl, MpmcSignalPl, MpscSignalPl, SpmcSignalPl};

#[test]
fn pure_signal_s () {
    let s = PureSignalSt::new();
    let p1 = s.emit();
    let p2 = s.present_else(value_proc(2), value_proc(3));
    assert_eq!(execute_process(p1.join(p2)), ((), 2));
}

#[test]
fn pure_signal_p () {
    let s = PureSignalPl::new();
    let p1 = s.emit();
    let p2 = s.present_else(value_proc(2), value_proc(3));
    assert_eq!(execute_process_parallel(p1.join(p2), 2), ((), 2));
}

#[test]
fn mpmc_signal_s () {
    let s = MpmcSignalSt::default();
    let p1 = s.emit(3).pause();
    let p2 = s.await_immediate()
             .map(|()| println!("receive s"));
    let p3 = s.await();
    let p4 = s.emit(5)
             .pause()
             .then(s.emit(7));
    let p5 = value_proc(())
             .pause()
             .then(s.await());
    let p6 = value_proc(())
             .pause()
             .pause()
             .then(s.present_else(value_proc(1), value_proc(0)));
    let vals = execute_process(p1.join(p2).join(p3).join(p4).join(p5).join(p6));
    assert_eq!(vals, ((((((), ()), vec![3, 5]), ()), vec![7]), 0));
}

#[test]
fn while_mpsc_spmc_s () {
    let gather = |x: isize, xs: &mut Vec<isize>| xs.push(x);
    let s = MpscSignalSt::new(Vec::new, gather);
    let s_clone = s.clone();
    let counter = SpmcSignalSt::new();
    let counter_clone = counter.clone();
    let decide_emit = move |n| {
        let v = counter_clone.last_value().unwrap();
        value_proc(v%2==0)
        .if_else(s_clone.emit(v), value_proc(()))
        .then(counter_clone.emit(v+1))
        .map(move |()| n)
    };
    let counter_clone = counter.clone();
    let while_p1 = move |n| {
        if counter_clone.last_value() == Some(n) { Exit(()) } else { Continue }
    };
    let mut received = Vec::new();
    let while_p2 = move |v: Vec<isize>| {
        received.push(v[0]);
        if v == vec![80] { Exit(received.clone()) } else { Continue }
    };
    let mut received = Vec::new();
    let while_p3 = move |v: isize| {
        received.push(v);
        if v == 80 { Exit(received.clone()) } else { Continue }
    };
    let p1 = value_proc(100)
             .and_then(decide_emit)
             .map(while_p1)
             .pause()
             .while_proc();
    let p1 = counter.emit(0).pause().then(p1);
    let p2 = s.await().map(while_p2).while_proc();
    let p3 = counter.await().pause().map(while_p3).while_proc();
    let vals = execute_process(p1.join(p2).join(p3));
    let p2_expected_val: Vec<_> = (0..81).filter(|x| x%2==0).collect();
    let p3_expected_val: Vec<_> = (0..81).collect();
    assert_eq!(vals, (((), p2_expected_val), p3_expected_val));
}

#[test]
fn while_mpmc_spmc_p () {
    let gather = |x: isize, xs: &mut Vec<isize>| xs.push(x);
    let s = MpmcSignalPl::new(Vec::new(), gather);
    let s_clone = s.clone();
    let counter = SpmcSignalPl::new();
    let counter_clone = counter.clone();
    let decide_emit = move |n| {
        let v = counter_clone.last_value().unwrap();
        value_proc(v%3==0)
        .if_else(s_clone.emit(v), value_proc(()))
        .then(counter_clone.emit(v+1))
        .map(move |()| n)
    };
    let counter_clone = counter.clone();
    let while_p1 = move |n| {
        if counter_clone.last_value() == Some(n) { Exit(()) } else { Continue }
    };
    let mut received = Vec::new();
    let while_p2 = move |v: Vec<isize>| {
        received.push(v[0]);
        if v == vec![81] { Exit(received.clone()) } else { Continue }
    };
    let mut received = Vec::new();
    let while_p3 = move |v: isize| {
        received.push(v);
        if v == 80 { Exit(received.clone()) } else { Continue }
    };
    let p1 = value_proc(100)
             .and_then(decide_emit)
             .map(while_p1)
             .pause()
             .while_proc();
    let p1 = counter.emit(0).pause().then(p1);
    let p2 = s.await().map(while_p2).while_proc();
    let p3 = counter.await().pause().pause().map(while_p3).while_proc();
    let vals = execute_process_parallel(p1.join(p2).join(p3), 3);
    let p2_expected_val: Vec<_> = (0..82).filter(|x| x%3==0).collect();
    let p3_expected_val: Vec<_> = (0..81).filter(|x| x%2==0).collect();
    assert_eq!(vals, (((), p2_expected_val.clone()), p3_expected_val));
}

#[test]
fn mpsc_signal_p () {
    let s = MpscSignalPl::default();
    let s_clone = s.clone();
    let mut counter = 0;
    let incr_counter = move |()| { counter += 1; counter };
    let emit_v = move |v| s_clone.emit(v);
    let p1 = value_proc(())
             .map(incr_counter)
             .and_then(emit_v)
             .repeat(10);
    let p2 = s.await().map(|x| { println!("{:?}", x); x });
    assert_eq!(execute_process_parallel(p1.join(p2), 2), ((), (1..11).collect::<Vec<_>>()));
}

#[test]
#[should_panic(expected = "Multiple emissions")]
fn spmc_multiple_emission_s () {
    let s = SpmcSignalSt::new();
    execute_process(s.emit("hello").repeat(2));
}

#[test]
#[should_panic(expected = "more than once")]
fn mpsc_multiple_reception_s () {
    let s = MpscSignalSt::default();
    execute_process(s.emit(true).join(s.await()).join(s.await()));
}

// Other speical behavoirs that can not be easily tested with Rust's built-in
// functionalities.
// 
/// Infinite loop that never ends.
#[test]
#[ignore]
fn loop_pure_signal_s () {
    let s = PureSignalSt::new();
    let p1 = s.emit().pause().loop_proc();
    let receive_cl = |()| println!("s received");
    let p2 = s.await_immediate().map(receive_cl).pause().loop_proc();
    let present_cl = |()| println!("present");
    let abscent_cl = |()| println!("not present");
    let present_s = value_proc(()).map(present_cl).pause();
    let abscent_s = value_proc(()).map(abscent_cl);
    let p3 = s.present_else(present_s, abscent_s).loop_proc();
    execute_process(p1.join(p2).join(p3));
}

/// Infinite loop that never ends.
#[test]
#[ignore]
fn loop_pure_signal_p () {
    let s = PureSignalPl::new();
    let p1 = s.emit().pause().loop_proc();
    let receive_cl = |()| println!("s received");
    let p2= s.await_immediate().map(receive_cl).pause().loop_proc();
    let present_cl = |()| println!("present");
    let abscent_cl = |()| println!("not present");
    let present_s = value_proc(()).map(present_cl).pause();
    let abscent_s = value_proc(()).map(abscent_cl);
    let p3 = s.present_else(present_s, abscent_s).loop_proc();
    execute_process_parallel(p1.join(p2).join(p3), 3);
}

/// The expected behavoir here is to hang because the signal is never emitted.
#[test]
#[ignore]
fn pure_signal_hang_s () {
    let s = PureSignalSt::new();
    let p = value_proc(())
            .map(|()| println!("waiting..."))
            .then(s.await_immediate());
    execute_process(p);
}

/// The expected behavoir here is to hang because the signal is never emitted.
#[test]
#[ignore]
fn mpmc_signal_hang_p () {
    let gather = |x: isize, xs: &mut Vec<isize>| xs.push(x);
    let s = MpmcSignalPl::new(Vec::new(), gather);
    let p = value_proc(())
            .map(|()| println!("waiting..."))
            .then(s.await());
    execute_process_parallel(p, 4);
}

/// Stack overflow should be detected in this case because for a single-producer
/// signal `await` terminates immediately.
#[test]
#[ignore]
fn spmc_overflow_s () {
    let s = SpmcSignalSt::new();
    let p1 = s.emit(0).pause().loop_proc();
    let p2 = s.await().loop_proc();
    execute_process(p1.join(p2));
}

/// Stack overflow should be detected in this case because for a single-producer
/// signal `await` terminates immediately.
#[test]
#[ignore]
fn spmc_overflow_p () {
    let s = SpmcSignalPl::new();
    let p1 = s.emit(0).pause().loop_proc();
    let p2 = s.await().loop_proc();
    execute_process_parallel(p1.join(p2), 5);
}

/// Some thread will panic due to the multiple emissions of the signal. However,
/// since the main thread doesn't panic, the attribute `should_panic` doesn't
/// work. Naturally, we will not pass the test.
#[test]
#[ignore]
fn spmc_multiple_emission_p () {
    let s = SpmcSignalPl::new();
    execute_process_parallel(s.emit("hello").repeat(2), 2);
}

/// Just as the above example, some thread will panic since the signal is awaited
/// more than once inside an instant, but the thread that panics isnot the 
/// main thread.
#[test]
#[ignore]
fn mpsc_multiple_reception_p () {
    let s = MpscSignalPl::default();
    execute_process_parallel(s.emit(true).join(s.await()).join(s.await()), 3);
}

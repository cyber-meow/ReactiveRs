extern crate reactive;

use reactive::process::{Process, ProcessMut, value_proc};
use reactive::process::{execute_process, execute_process_parallel};
use reactive::process::LoopStatus::{Continue, Exit};

#[test]
fn process42_s() {
    let p = value_proc(39);
    let p = p.pause().pause();
    let p = p.map(|v| v+3);
    assert_eq!(execute_process(p), 42);
}

#[test]
fn process42_p() {
    let p = value_proc(21);
    let p = p.pause().pause();
    let p = p.map(|v| v*2);
    assert_eq!(execute_process_parallel(p, 3), 42);
}

#[test]
fn while_proc() {
    let mut counter = 0;
    let mut num_pair = 0;
    let while_cond = move |n| {
        if counter == n {
            Exit((true, num_pair))
        } else if counter%2 == 0 {
            counter += 1;
            num_pair += 1;
            println!("pair number");
            Continue
        } else {
            counter += 1;
            println!("odd number");
            Continue
        }
    };
    let choose_value =
        |(b, v)| value_proc(b).if_else(
            value_proc(()).pause().then(value_proc(v)),
            value_proc(()).map(|()| 0));
    let p1 = value_proc(100)
             .map(while_cond)
             .pause()
             .while_proc()
             .and_then(choose_value);
    assert_eq!(execute_process(p1), 50);
    // The value is copied and moved so the original one is not changed.
    assert_eq!(num_pair, 0);
}

#[test]
fn while_repeat_join () {
    let mut counter = 0;
    let while_cond = move |n| {
        if counter == n {
            Exit(counter)
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
    let say_hello = move |()| {
        counter += 1;
        println!("hello");
        counter
    };
    let p1 = value_proc(10)
             .map(while_cond)
             .pause()
             .while_proc();
    let p2 = value_proc(()).map(say_hello).pause().repeat(5);
    assert_eq!(execute_process_parallel(p1.join(p2), 2), (10, 5));
}

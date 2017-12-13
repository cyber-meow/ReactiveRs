use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use runtime::{Runtime, SingleThreadRuntime};
use runtime::{ParallelRuntime, ParallelRuntimeCollection};
use process::{ProcessSt, ProcessPl};

/// Executes a process in a newly created runtime and return its value (without parallization).
pub fn execute_process<P>(p: P) -> P::Value where P: ProcessSt
{
    let mut runtime = SingleThreadRuntime::new();
    let res: Rc<RefCell<Option<P::Value>>> = Rc::new(RefCell::new(None));
    let res2 = res.clone();
    let c = move |_: &mut SingleThreadRuntime, v| *res2.borrow_mut() = Some(v);
    runtime.on_current_instant(Box::new(|r: &mut SingleThreadRuntime, _| p.call(r, c)));
    runtime.execute();
    let mut res = res.borrow_mut();
    res.take().unwrap()
}

/// Executes a process in newly created runtimes and return its value (with parallization).
pub fn execute_process_parallel<P>(p: P, num_runtimes: usize) -> P::Value where P: ProcessPl {
    if num_runtimes == 0 {
        panic!("There should be at least one runtime!");
    }
    let mut runtime_col = ParallelRuntimeCollection::new(num_runtimes);
    let res: Arc<Mutex<Option<P::Value>>> = Arc::new(Mutex::new(None));
    let res2 = res.clone();
    let c = move |_: &mut ParallelRuntime, v| *res2.lock().unwrap() = Some(v);
    runtime_col.register_work(Box::new(|r: &mut ParallelRuntime, _| p.call(r, c)));
    let f = || ();
    runtime_col.execute(f);
    let mut res = res.lock().unwrap();
    res.take().unwrap()
}

/// Executes a process in newly created runtimes and return its value. Each runtime is
/// runned in a separated child thread and these threads are runned in parallel with a
/// main function that is executed in the main thread. This construction is necessary
/// when some part of the program must be executed in the main thread.
pub fn execute_process_parallel_with_main<F, P>(f: F, p: P, num_runtimes: usize) -> P::Value
    where F: FnOnce(), P: ProcessPl
{
    if num_runtimes == 0 {
        panic!("There should be at least one runtime!");
    }
    let mut runtime_col = ParallelRuntimeCollection::new(num_runtimes);
    let res: Arc<Mutex<Option<P::Value>>> = Arc::new(Mutex::new(None));
    let res2 = res.clone();
    let c = move |_: &mut ParallelRuntime, v| *res2.lock().unwrap() = Some(v);
    runtime_col.register_work(Box::new(|r: &mut ParallelRuntime, _| p.call(r, c)));
    runtime_col.execute(f);
    let mut res = res.lock().unwrap();
    res.take().unwrap()
}

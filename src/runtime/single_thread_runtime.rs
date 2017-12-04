use std::rc::Rc;

use Continuation;
use runtime::Runtime;
use signal::signal_runtime::SignalRuntimeRefBase;

/// Runtime for executing reactive continuations.
pub struct SingleThreadRuntime {
    current_instant_works: Rc<Vec<Box<Continuation<SingleThreadRuntime, ()>>>>,
    next_instant_works: Rc<Vec<Box<Continuation<SingleThreadRuntime, ()>>>>,
    end_of_instant_works: Vec<Box<Continuation<SingleThreadRuntime, ()>>>,
    emitted_signals: Vec<Box<SignalRuntimeRefBase<SingleThreadRuntime>>>,
    await_counter: usize,
    test_presence_signals: Vec<Box<SignalRuntimeRefBase<SingleThreadRuntime>>>,
    #[cfg(feature = "debug")]
    instant: usize,
}

impl Runtime for SingleThreadRuntime {
    /// Executes a single instant to completion. Indicates if more work remains to be done.
    fn instant(&mut self) -> bool {
        #[cfg(feature = "debug")] {
            println!("instant {}", self.instant);
            self.instant += 1;
        }
        while let Some(work) = Rc::get_mut(&mut self.current_instant_works).unwrap().pop() {
            work.call_box(self, ());
        }
        while let Some(work) = self.end_of_instant_works.pop() {
            work.call_box(self, ());
        }
        self.end_of_instant();
        self.current_instant_works.len() != 0 || self.await_counter > 0
    }
}

impl SingleThreadRuntime {
    /// Creates a new `Runtime`.
    pub fn new() -> Self { 
        SingleThreadRuntime {
            current_instant_works: Rc::new(Vec::new()),
            next_instant_works: Rc::new(Vec::new()),
            end_of_instant_works: Vec::new(),
            emitted_signals: Vec::new(),
            await_counter: 0,
            test_presence_signals: Vec::new(),
            #[cfg(feature = "debug")]
            instant: 0,
        }
    }
    
    fn end_of_instant(&mut self) {
        while let Some(s) = self.test_presence_signals.pop() {
            s.execute_present_works_box(self);
        }
        while let Some(s) = self.emitted_signals.pop() {
            s.reset_box();
        }
        self.current_instant_works = self.next_instant_works.clone();
        self.next_instant_works = Rc::new(Vec::new());
    }
    
    /// Registers a continuation to execute on the current instant.
    pub(crate) fn on_current_instant(&mut self, c: Box<Continuation<Self, ()>>) {
        Rc::get_mut(&mut self.current_instant_works).unwrap().push(c);
    }

    /// Registers a continuation to execute at the next instant.
    pub(crate) fn on_next_instant(&mut self, c: Box<Continuation<Self, ()>>) {
        Rc::get_mut(&mut self.next_instant_works).unwrap().push(c);
    }

    /// Registers a continuation to execute at the end of the instant. Runtime calls for `c`
    /// behave as if they where executed during the next instant.
    pub(crate) fn on_end_of_instant(&mut self, c: Box<Continuation<Self, ()>>) {
        self.end_of_instant_works.push(c);
    }

    /// Increases the await counter by 1 when some process await a signal to continue.
    pub(crate) fn incr_await_counter(&mut self) {
        self.await_counter += 1;
    }

    /// Decrease the await counter by 1 when some signal is emitted and
    /// the corresponding process is thus executed.
    pub(crate) fn decr_await_counter(&mut self) {
        self.await_counter -= 1;
    }
    
    /// Registers a emitted signal for the current instant.
    pub(crate) fn emit_signal(&mut self, s: Box<SignalRuntimeRefBase<Self>>) {
        self.emitted_signals.push(s);
    }

    /// Registers a signal for which we need to test its presence on the current instant.
    pub(crate) fn add_test_signal(&mut self, s: Box<SignalRuntimeRefBase<Self>>) {
        self.test_presence_signals.push(s);
    }
}

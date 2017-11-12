use std::rc::Rc;

use Continuation;
use signal::SignalRuntimeRef;

/// Runtime for executing reactive continuations.
pub struct Runtime {
    current_instant_works: Rc<Vec<Box<Continuation<()>>>>,
    next_instant_works: Rc<Vec<Box<Continuation<()>>>>,
    end_of_instant_works: Vec<Box<Continuation<()>>>,
    emitted_signals: Vec<SignalRuntimeRef>,
    test_presence_signals: Vec<SignalRuntimeRef>,
}

impl Runtime {
    /// Creates a new `Runtime`.
    pub fn new() -> Self { 
        Runtime {
            current_instant_works: Rc::new(Vec::new()),
            next_instant_works: Rc::new(Vec::new()),
            end_of_instant_works: Vec::new(),
            emitted_signals: Vec::new(),
            test_presence_signals: Vec::new(),
        }
    }

    /// Executes instants until all work is completed.
    pub fn execute(&mut self) {
        while self.instant() {};
    }

    /// Executes a single instant to completion. Indicates if more work remains to be done.
    pub fn instant(&mut self) -> bool {
        while let Some(work) = Rc::get_mut(&mut self.current_instant_works).unwrap().pop() {
            work.call_box(self, ());
        }
        while let Some(work) = self.end_of_instant_works.pop() {
            work.call_box(self, ());
        }
        self.end_of_instant();
        self.current_instant_works.len() != 0
    }
    
    fn end_of_instant(&mut self) {
        while let Some(mut s) = self.test_presence_signals.pop() {
            s.execute_present_works(self);
        }
        while let Some(mut s) = self.emitted_signals.pop() {
            s.reset();
        }
        self.current_instant_works = self.next_instant_works.clone();
        self.next_instant_works = Rc::new(Vec::new());
        self.end_of_instant_works = Vec::new();
    }

    /// Registers a continuation to execute on the current instant.
    pub(crate) fn on_current_instant(&mut self, c: Box<Continuation<()>>) {
        Rc::get_mut(&mut self.current_instant_works).unwrap().push(c);
    }

    /// Registers a continuation to execute at the next instant.
    pub(crate) fn on_next_instant(&mut self, c: Box<Continuation<()>>) {
        Rc::get_mut(&mut self.next_instant_works).unwrap().push(c);
    }

    /// Registers a continuation to execute at the end of the instant. Runtime calls for `c`
    /// behave as if they where executed during the next instant.
    pub(crate) fn on_end_of_instant(&mut self, c: Box<Continuation<()>>) {
        self.end_of_instant_works.push(c);
    }

    /// Registers a emitted signal for the current instant.
    pub(crate) fn emit_signal(&mut self, s: SignalRuntimeRef) {
        self.emitted_signals.push(s);
    }

    /// Registers a signal for which we need to test its presence on the current instant.
    pub(crate) fn add_test_signal(&mut self, s: SignalRuntimeRef) {
        self.test_presence_signals.push(s);
    }
}

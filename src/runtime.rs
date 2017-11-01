use continuation::Continuation;

/// Runtime for executing reactive continuations.
pub struct Runtime {
    current_instant_works: Vec<Box<Continuation<()>>>,
    next_instant_works: Vec<Box<Continuation<()>>>,
}

impl Runtime {
    /// Creates a new `Runtime`.
    pub fn new() -> Self { 
        Runtime {
            current_instant_works: Vec::new(),
            next_instant_works: Vec::new(),
        }
    } // TODO

    /// Executes instants until all work is completed.
    pub fn execute(mut self) {
        while self.instant() {
            self.current_instant_works = self.next_instant_works;
            self.next_instant_works = Vec::new();
        };
    }

    /// Executes a single instant to completion. Indicates if more work remains to be done.
    pub fn instant(&mut self) -> bool {
        while let Some(new_work) = self.current_instant_works.pop() {
            new_work.call_box(self, ());
        }
        self.next_instant_works.len() != 0
    }

    /// Registers a continuation to execute on the current instant.
    pub(crate) fn on_current_instant(&mut self, c: Box<Continuation<()>>) {
        self.current_instant_works.push(c);
    }

    /// Registers a continuation to execute at the next instant.
    pub(crate) fn on_next_instant(&mut self, c: Box<Continuation<()>>) {
        self.next_instant_works.push(c);
    }

    /// Registers a continuation to execute at the end of the instant. Runtime calls for `c`
    /// behave as if they where executed during the next instant.
    pub(crate) fn on_end_of_instant(&mut self, c: Box<Continuation<()>>) {
        unimplemented!() // TODO
    }
}

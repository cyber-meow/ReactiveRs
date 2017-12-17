//! A Rust library for reactive programming that reuse the basic constructions of
//! [ReactiveML] (http://rml.lri.fr/).
//!
//! For the moment being, the library include implementation of underlying runtimes
//! and continuations that are essential for the execution engines, the module
//! `process` contating different methods for the creation of new
//! processes, and four kinds of signals in charge of inter-process communication.  
//!
//! In most of the time the users only care about modules `process` and `signal`.
//! However it is not always trivial to build a process due to the necessity of
//! having a `'static` lifetime for every process. There are two execution
//! engines for the library: one is parallel and another is not. There isn't a
//! difference when defining processes but for signals we must choose the right
//! version to use. Also notice that every process to be executed by the parallel
//! engine must implement the traits `Send` and `Sync`.
//!
//! I also plan to add some control structures (like `do..while` and `do..until`)
//! in the library but I don't have time to work on it at this moment. It will
//! be also great to modify the traits `process::Process` and `process::ProcessMut`
//! to enable more intuitive process definition and simpler manipulation.

extern crate either;
extern crate crossbeam;
extern crate rand;
extern crate ordermap;

pub mod runtime;
pub mod continuation;
pub mod process;
pub mod signal;

pub use process::*;
pub use signal::*;

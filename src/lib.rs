extern crate either;
extern crate crossbeam;
extern crate rand;
extern crate ordermap;

pub mod single_thread;
pub mod parallel;

pub use single_thread::*;

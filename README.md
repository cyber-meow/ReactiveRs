# ReactiveRs
A rust library for reactive programming inspired from [ReactiveML](http://rml.lri.fr/).
Class project of the ENS M1 course
"[parallel and reactive programming](http://www.di.ens.fr/~beaugnon/cours/parallele_et_reactif/)"
given by Albert Cohen and Marc Pouzet.

For the moment being, the library include implementation of underlying
runtimes and continuations that are essential for the execution engines,
the module `process` contating different methods for the creation of new
processes, and four kinds of signals in charge of inter-process communication.

In most of the time the users only care about modules `process` and `signal`.
However it is not always trivial to build a process due to the necessity of
having a `'static` lifetime for every process. There are two execution
engines for the library: one is parallel and another is not. There isn't a
difference when defining processes but for signals we must choose the right
version to use. Also notice that every process to be executed by the parallel
engine must implement the traits `Send` and `Sync`.

I also plan to add some control structures (like `do..while` and `do..until`)
in the library but I don't have time to work on it at this moment. It will be
also great to modify the traits `process::Process` and `process::ProcessMut`
to enable more intuitive process definition and simpler manipulation.

On the other hand, I particularly refer to the
[futures](https://github.com/alexcrichton/futures-rs) crate for the design of
my library. Nevertheless, the use of chaining structures may be the reason of
the slow compilation. For example, compiling the `sugarscape` binary on my core
i7 laptop can take up to several minutes.

## Documentation

Of course you can simply clone the repository and run `cargo doc --open`.

Otherwise it's also availabe [here](https://cyber-meow.github.io/ReactiveRs/).

## Some Examples

To create a simple process and execute it:

```Rust
extern crate reactive;

use reactive::{Process, ProcessMut, value_proc, execute_process};

fn main () {
    let mut counter = 0;
    let say_hello = move |()| {
        counter += 1;
        println!("hello");
        counter
    };
    let p = value_proc(()).map(say_hello).pause().repeat(5);
    assert_eq!(execute_process(p), (1..6).collect::<Vec<_>>());
}
```

The process will also print five lines of `hello` to the standard output.

To define processes that communicate with each other using signals and run
them in parallel (spawing two child threads):

```Rust
extern crate reactive;

use reactive::{Process, value_proc, execute_process_parallel};
use reactive::{PureSignal, Signal, PureSignalPl};

fn main () {
    let s = PureSignalPl::new();
    let p1 = s.emit();
    let p2 = s.present_else(value_proc(2), value_proc(3));
    assert_eq!(execute_process_parallel(p1.join(p2), 2), ((), 2));
}
```

For more examples please look into the `tests` directory.

## Sugarscape

As a more complicated example one can take a look at the file
`example/sugarscape.rs` where I use the library (parallel version) to implement
Sugarscape model. For the moment being I only include the basic moving rule and
the reproduction rule but I may add other features later. To run the model,
just type `cargo run --example sugarscape`.

The beginning of the simulation:

<img src="https://imgur.com/721tjkz.png" alt="Sugarscape1" width="600">

After some time:

<img src="https://imgur.com/HoWkwSE.png" alt="Sugarscape2" width="600">

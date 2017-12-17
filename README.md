# ReactiveRs
A rust library for reactive programming inspired from [ReactiveML](http://rml.lri.fr/).
Class project of the ENS M1 course "[parallel and reactive programming](http://www.di.ens.fr/~pouzet/cours/parallele_et_reactif/)" given by Albert Cohen and Marc Pouzet.

For the moment being, the library include implementation of underlying runtimes and continuations that are essential for the execution engines, the module `process` contating different methods for the creation of new processes, and four kinds of signals in charge of inter-process communication.

In most of the time the users only care about modules `process` and `signal`. However it is not always trivial to build a process due to the necessity of having a `'static` lifetime for every process. There are two execution engines for the library: one is parallel and another is not. There isn't a difference when defining processes but for signals we must choose the right version to use. Also notice that every process to be executed by the parallel engine must implement the traits `Send` and `Sync`.

I also plan to add some control structures (like `do..while` and `do..until`) in the library but I don't have time to work on it at this moment. It will be also great to modify the traits `process::Process` and `process::ProcessMut` to enable more intuitive process definition and simpler manipulation.

On the other hand, I particularly refer to the [futures](https://github.com/alexcrichton/futures-rs) crate for the design of my library. Nevertheless, the use of chaining structures may be the reason of the slow compilation. For example, compiling the `sugarscape` binary on my core i7 laptop can take up to several minutes.

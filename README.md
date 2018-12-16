# Rusty Tasking

My goal here is to learn and experiment with the Rust programming language by
writing a runtime library for scheduling task parallelism. I will try to
implement a few ideas from my [previous work][1] (in C), which I think map
nicely to Rust. Performance is not yet a concern. Make it work, before
attempting to fix the mess, as they say.

There are a couple of related projects, including [threadpool][2],
[crossbeam][3], and [rayon][4], which will be important for comparison later
on, after some progress on my own.

<!-- Links -->

[1]: https://epub.uni-bayreuth.de/2990
[2]: https://crates.io/crates/threadpool
[3]: https://crates.io/crates/crossbeam
[4]: https://crates.io/crates/rayon

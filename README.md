# Rusty Tasking

My goal here is to learn and experiment with the Rust programming language by
writing a runtime library for scheduling task parallelism. I will try to
implement a few ideas from my [previous work][1] (in C), which I think map
nicely to Rust. Performance is secondary at this point. Make it work, before
attempting to fix the mess, as they say.

There are a couple of related projects, including [threadpool][2],
[crossbeam][3], and [rayon][4], which will be important for comparison later
on, after some progress on my own.

## Interesting Directions

- Implement [scoped tasks](src/scope.rs)
- Implement [proactive work stealing for futures][5]
- Implement preemptible [tasks][6] or [workers][7]

## Learning Rust

- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [A Half Hour to Learn Rust](https://fasterthanli.me/articles/a-half-hour-to-learn-rust)
- [Safe Systems Programming in Rust](https://iris-project.org/pdfs/2021-rustbelt-cacm-final.pdf)
- [The Usability of Ownership](https://arxiv.org/abs/2011.06171)
- How to Read Rust Functions: [Part 1](https://www.possiblerust.com/guide/how-to-read-rust-functions-part-1)
- [Rust Atomics and Locks](https://marabos.nl/atomics/)

## Other Bookmarks

- [A hand-rolled replacement of Rayon](https://gendignoux.com/blog/2024/11/18/rust-rayon-optimized.html#a-hand-rolled-replacement-of-rayon)
  (discusses work splitting)

<!-- Links -->

[1]: https://github.com/aprell/tasking-2.0
[2]: https://crates.io/crates/threadpool
[3]: https://crates.io/crates/crossbeam
[4]: https://crates.io/crates/rayon
[5]: https://dl.acm.org/doi/10.1145/3293883.3295735
[6]: https://www.usenix.org/conference/atc20/presentation/boucher
[7]: https://dl.acm.org/doi/10.1145/3437801.3441610

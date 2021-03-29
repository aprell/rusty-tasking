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

- Implement scoped tasks
- Implement [proactive work stealing for futures][5]

## Learning Rust

- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [A Half Hour to Learn Rust](https://fasterthanli.me/articles/a-half-hour-to-learn-rust)
- [Safe Systems Programming in Rust](https://iris-project.org/pdfs/2021-rustbelt-cacm-final.pdf)
- [The Usability of Ownership](https://arxiv.org/abs/2011.06171)

<!-- Links -->

[1]: https://github.com/aprell/tasking-2.0
[2]: https://crates.io/crates/threadpool
[3]: https://crates.io/crates/crossbeam
[4]: https://crates.io/crates/rayon
[5]: https://dl.acm.org/citation.cfm?id=3295735

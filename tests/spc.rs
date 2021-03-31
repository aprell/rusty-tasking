#[macro_use]
extern crate rusty_tasking;

use rusty_tasking::runtime::Runtime;
use rusty_tasking::scope::Scope;
use rusty_tasking::task::ScopedAsync;
use rusty_tasking::worker::Worker;

use std::time::{Duration, Instant};

fn fib(n: u64) -> u64 {
    if n < 2 { return n; }
    let mut f = (0, 1);
    for _ in 2..=n { f = (f.1, f.0 + f.1); }
    f.1
}

fn time_fib(target: Duration) -> u64 {
    let mut n = 0;
    loop {
        let start = Instant::now();
        let _ = fib(n);
        if start.elapsed() >= target { break; }
        n += 1;
    }
    n
}

fn compute(n: u64, duration: Duration) {
    let start = Instant::now();
    while start.elapsed() < duration {
        let _ = fib(n);
    }
    // println!("{} us", start.elapsed().as_micros());
}

static NUM_TASKS: u32 = 100;
static TASK_LENGTH: Duration = Duration::from_micros(10);

#[test]
fn spc() {
    // Create three additional workers
    let runtime = Runtime::init(4);

    finish! {
        let n = time_fib(TASK_LENGTH / 5);
        for _ in 0..NUM_TASKS {
            scoped_spawn!(compute(n, TASK_LENGTH));
        }
    }

    let stats = runtime.join();
    assert_eq!(stats.num_tasks_executed.get(), NUM_TASKS);
}

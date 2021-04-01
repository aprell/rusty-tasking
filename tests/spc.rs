#[macro_use]
extern crate rusty_tasking;
extern crate utils;

use rusty_tasking::runtime::Runtime;
use rusty_tasking::scope::Scope;
use rusty_tasking::task::ScopedAsync;
use rusty_tasking::worker::Worker;
use std::time::Duration;

static NUM_TASKS: u32 = 100;
static TASK_LENGTH: Duration = Duration::from_micros(10);

fn produce() {
    for _ in 0..NUM_TASKS {
        scoped_spawn!(utils::compute(TASK_LENGTH));
    }
}

#[test]
fn spc() {
    // Create three additional workers
    let runtime = Runtime::init(4);

    finish! { produce(); }

    let stats = runtime.join();
    assert_eq!(stats.num_tasks_executed.get(), NUM_TASKS);
}

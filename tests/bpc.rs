#[macro_use]
extern crate rusty_tasking;
extern crate utils;

use rusty_tasking::runtime::Runtime;
use rusty_tasking::scope::Scope;
use rusty_tasking::task::ScopedAsync;
use rusty_tasking::worker::Worker;
use std::time::Duration;

static NUM_TASKS_TOTAL: u32 = 100;
static NUM_TASKS_LEVEL: u32 = NUM_TASKS_TOTAL / LEVELS - 1;
static LEVELS: u32 = 10;
static TASK_LENGTH: Duration = Duration::from_micros(10);

fn produce(level: u32) {
    if level > 0 {
        scoped_spawn!(produce(level - 1));
        for _ in 0..NUM_TASKS_LEVEL {
            scoped_spawn!(utils::compute(TASK_LENGTH));
        }
    }
}

#[test]
fn bpc() {
    // Create three additional workers
    let runtime = Runtime::init(4);

    finish! { produce(LEVELS); }

    let stats = runtime.join();
    assert_eq!(stats.num_tasks_executed.get(), NUM_TASKS_TOTAL);
}

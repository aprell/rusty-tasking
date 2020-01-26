#[macro_use]
extern crate rusty_tasking;

use rusty_tasking::runtime::Runtime;
use rusty_tasking::task::Async;
use rusty_tasking::worker::Worker;

#[test]
fn random_stealing() {
    // Create three additional workers
    let runtime = Runtime::init(4);
    let master = runtime.master;

    for _ in 0..999 {
        spawn!();
    }

    let mut num_tasks_executed = 0;
    while master.has_tasks() {
        master.try_handle_steal_request();
        match master.pop() {
            Some(task) => {
                task.run();
                num_tasks_executed += 1;
            }
            None => break,
        }
    }

    master.stats.num_tasks_executed.increment(num_tasks_executed);
    let stats = runtime.join();
    assert_eq!(stats.num_tasks_executed.get(), 999);
}

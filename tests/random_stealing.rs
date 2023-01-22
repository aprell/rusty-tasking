#[macro_use]
extern crate rusty_tasking;

use rusty_tasking::runtime::Runtime;
use rusty_tasking::task::Async;
use rusty_tasking::worker::Worker;

#[test]
fn random_stealing() {
    // Create three additional workers
    let runtime = Runtime::init(4);
    let leader = runtime.leader;

    for _ in 0..999 {
        spawn!();
    }

    let mut num_tasks_executed = 0;
    while leader.has_tasks() {
        leader.try_handle_steal_request();
        match leader.pop() {
            Some(task) => {
                task.run();
                num_tasks_executed += 1;
            }
            None => break,
        }
    }

    leader.stats.num_tasks_executed.add(num_tasks_executed);
    let stats = runtime.join();
    assert_eq!(stats.num_tasks_executed.get(), 999);
}

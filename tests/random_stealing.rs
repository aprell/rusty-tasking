use rusty_tasking::runtime::{Runtime};
use rusty_tasking::task::{Async};

#[test]
fn random_stealing() {
    // Create three additional workers
    let runtime = Runtime::init(4);
    let master = &runtime.master;

    for _ in 0..99 {
        let task = Async::task(Box::new(|| ()));
        master.push(Box::new(task));
    }

    while master.has_tasks() {
        master.try_handle_steal_request();
        match master.pop() {
            Some(task) => task.run(),
            None => break,
        }
    }

    runtime.join();
}
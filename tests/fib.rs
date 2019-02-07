use rusty_tasking::runtime::Runtime;
use rusty_tasking::task::Async;
use rusty_tasking::worker::Worker;

macro_rules! async_closure {
    // `tt` is a token tree
    ($($body: tt)*) => (Box::new(move || { $($body)* }))
}

macro_rules! async_task {
    // `tt` is a token tree
    ($($body: tt)*) => {
        let task = Async::task(async_closure! { $($body)* });
        Worker::current().push(Box::new(task));
        // No return value
    }
}

macro_rules! async_future {
    // `tt` is a token tree
    ($($body: tt)*) => {
        {
            let (task, future) = Async::future(async_closure! { $($body)* });
            Worker::current().push(Box::new(task));
            future
        }
    }
}

fn parfib(n: u64) -> u64 {
    if n < 2 { return n; }
    let x = async_future!(parfib(n - 1));
    let y = parfib(n - 2);
    x.wait() + y
}

#[test]
fn main() {
    // Create three additional workers
    let runtime = Runtime::init(4);

    let n = parfib(20);
    assert_eq!(n, 6765);

    let _stats = runtime.join();
}

use rusty_tasking::runtime::Runtime;
use rusty_tasking::task::Async;
use rusty_tasking::worker::Worker;

macro_rules! async_closure {
    // `tt` is a token tree
    ($($body: tt)*) => (Box::new(move || { $($body)* }))
}

fn parfib(n: u64) -> u64 {
    if n < 2 { return n; }
    let (task, x) = Async::future(async_closure! { parfib(n - 1) });
    Worker::current().push(Box::new(task));
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

#[macro_use]
extern crate rusty_tasking;

use rusty_tasking::runtime::Runtime;
use rusty_tasking::task::Async;
use rusty_tasking::worker::Worker;

fn parfib(n: u64) -> u64 {
    if n < 2 { return n; }
    let mut x = async_future!(parfib(n - 1));
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

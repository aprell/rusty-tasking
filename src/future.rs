use std::sync::mpsc::{Sender, Receiver};

use crate::worker::{Tasks, Worker};

// Futures and promises

pub struct Future<T>(pub Receiver<T>);

pub struct Promise<T>(pub Sender<T>);

impl<T> Future<T> {
    // Block until result is available
    pub fn get(self) -> T {
        self.0.recv().unwrap()
    }

    // Try to overlap waiting with useful work
    pub fn wait(self) -> T {
        if let Some(val) = self.0.try_recv().ok() {
            return val;
        }

        let worker = Worker::current();
        let mut num_tasks_executed = 0;

        while let Some(task) = worker.pop() {
            worker.try_handle_steal_request();
            task.run();
            num_tasks_executed += 1;
            if let Some(val) = self.0.try_recv().ok() {
                worker.stats.num_tasks_executed.increment(num_tasks_executed);
                return val;
            }
        }

        loop {
            match worker.steal_one().wait() {
                Tasks::None => (),
                Tasks::One(task) => {
                    task.run();
                    num_tasks_executed += 1;
                }
                _ => panic!(),
            }
            if let Some(res) = self.0.try_recv().ok() {
                worker.stats.num_tasks_executed.increment(num_tasks_executed);
                return res;
            }
        }
    }
}

impl<T> Promise<T> {
    pub fn set(self, value: T) {
        self.0.send(value).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;
    use std::thread;
    use super::*;

    #[test]
    fn future_promise() {
        let (sender, receiver) = channel();
        Promise(sender).set(1);
        assert_eq!(Future(receiver).get(), 1);
    }

    #[test]
    fn future_promise_thread() {
        let (sender1, receiver1) = channel();

        let t = thread::spawn(|| {
            let (sender2, receiver2) = channel();
            Promise(sender1).set(("ping", Promise(sender2)));
            assert_eq!(Future(receiver2).get(), "pong");
        });

        let (msg, promise) = Future(receiver1).get();
        assert_eq!(msg, "ping");
        promise.set("pong");
        t.join().unwrap();
    }
}

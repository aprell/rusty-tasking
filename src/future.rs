use crate::channel::{one_shot_channel, Sender, Receiver};
use crate::worker::{Tasks, Worker};

// Futures and promises

pub enum Future<T> {
    Lazy(Option<T>),
    Chan(Receiver<T>),
}

pub enum Promise<T> {
    Lazy(*mut Future<T>),
    Chan(Sender<T>),
}

// Rustonomicon: "Raw pointers are neither `Send` nor `Sync` (because they
// have no safety guards). (...) It's important that they aren't thread-safe
// to prevent types that contain them from being automatically marked as
// thread-safe. (...) Types that aren't automatically derived can simply
// implement them if desired."
//
// A `Promise` is sendable after it is promoted to a `Sender`
unsafe impl<T> Send for Promise<T> {}

impl<T> Future<T> {
    // Block until result is available
    pub fn get(self) -> T {
        match self {
            // Panic if opt.is_none() (better than waiting forever)
            Self::Lazy(opt) => opt.unwrap(),
            Self::Chan(chan) => {
                while !chan.is_ready() {
                    std::hint::spin_loop();
                }
                chan.receive()
            }
        }
    }

    pub fn is_ready(&self) -> bool {
        match self {
            Self::Lazy(opt) => opt.is_some(),
            Self::Chan(chan) => chan.is_ready(),
        }
    }

    fn try_get(&mut self) -> Option<T> {
        match self {
            Self::Lazy(opt) => opt.take(),
            Self::Chan(chan) => {
                match chan.is_ready() {
                    true => Some(chan.receive()),
                    false => None,
                }
            },
        }
    }

    // Try to overlap waiting with useful work
    // NOTE: We cannot consume the future because the associated promise
    // relies on the future's stack address!
    pub fn wait(&mut self) -> T {
        if let Some(val) = self.try_get() {
            return val;
        }

        let worker = Worker::current();
        let mut num_tasks_executed = 0;

        while let Some(task) = worker.pop() {
            worker.try_handle_steal_request();
            task.run();
            num_tasks_executed += 1;
            if let Some(val) = self.try_get() {
                worker.stats.num_tasks_executed.add(num_tasks_executed);
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
            if let Some(res) = self.try_get() {
                worker.stats.num_tasks_executed.add(num_tasks_executed);
                return res;
            }
        }
    }
}

impl<T> Promise<T> {
    pub fn promote(&mut self) {
        match *self {
            Self::Lazy(fut) => {
                let (sender, receiver) = one_shot_channel();
                unsafe { *fut = Future::Chan(receiver); }
                *self = Self::Chan(sender);
            },
            Self::Chan(_) => (),
        }
    }

    pub fn set(self, value: T) {
        match self {
            Self::Lazy(fut) => unsafe {
                match *fut {
                    Future::Lazy(ref mut opt) => {
                        assert!(opt.is_none());
                        *opt = Some(value);
                    }
                    Future::Chan(_) => {
                        // Something went wrong
                        panic!();
                    }
                }
            }
            Self::Chan(chan) => {
                chan.send(value);
            }
        }
    }
}

impl<T> From<Sender<T>> for Promise<T> {
    fn from(value: Sender<T>) -> Self {
        Promise::Chan(value)
    }
}

impl<T> From<&mut Future<T>> for Promise<T> {
    fn from(value: &mut Future<T>) -> Self {
        Promise::Lazy(value)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use super::*;

    #[test]
    fn future_promise() {
        let (sender, receiver) = one_shot_channel();
        Promise::Chan(sender).set(1);
        assert_eq!(Future::Chan(receiver).get(), 1);
    }

    #[test]
    fn future_promise_lazy() {
        let mut f = Future::Lazy(None);
        let p = Promise::Lazy(&mut f);
        p.set(1);
        assert_eq!(f.get(), 1);
    }

    #[test]
    fn future_promise_thread() {
        let (sender1, receiver1) = one_shot_channel();

        let t = thread::spawn(|| {
            let (sender2, receiver2) = one_shot_channel();
            Promise::Chan(sender1).set(("ping", Promise::Chan(sender2)));
            assert_eq!(Future::Chan(receiver2).get(), "pong");
        });

        let (msg, promise) = Future::Chan(receiver1).get();
        assert_eq!(msg, "ping");
        promise.set("pong");
        t.join().unwrap();
    }

    #[test]
    fn future_promise_lazy_thread() {
        let mut f1 = Future::Lazy(None);
        let mut p1 = Promise::Lazy(&mut f1);
        p1.promote();

        let t = thread::spawn(|| {
            let mut f2 = Future::Lazy(None);
            let mut p2 = Promise::Lazy(&mut f2);
            p2.promote();
            p1.set(("ping", p2));
            assert_eq!(f2.get(), "pong");
        });

        let (msg, p2) = f1.get();
        assert_eq!(msg, "ping");
        p2.set("pong");
        t.join().unwrap();
    }
}

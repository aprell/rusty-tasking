use std::sync::mpsc::{channel, Sender, Receiver};

use crate::worker::{Tasks, Worker};

// Futures and promises

pub struct Future<T>(pub Receiver<T>);

pub struct Promise<T>(pub Sender<T>);

impl<T> Future<T> {
    // Block until result is available
    pub fn get(self) -> T {
        self.0.recv().unwrap()
    }

    fn try_get(&mut self) -> Option<T> {
        self.0.try_recv().ok()
    }

    // Try to overlap waiting with useful work
    pub fn wait(mut self) -> T {
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
            if let Some(res) = self.try_get() {
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

// Lazy allocation

pub enum LazyFuture<T> {
    Lazy(Option<T>),
    Chan(Future<T>),
}

pub enum LazyPromise<T> {
    Lazy(*mut LazyFuture<T>),
    Chan(Promise<T>),
}

// Rustonomicon: "Raw pointers are neither `Send` nor `Sync` (because they
// have no safety guards). (...) It's important that they aren't thread-safe
// to prevent types that contain them from being automatically marked as
// thread-safe. (...) Types that aren't automatically derived can simply
// implement them if desired."
//
// A `LazyPromise` is sendable after promotion to a `Promise`
unsafe impl<T> Send for LazyPromise<T> {}

impl<T> LazyFuture<T> {
    pub fn new() -> LazyFuture<T> {
        LazyFuture::Lazy(None)
    }

    // Block until result is available
    pub fn get(self) -> T {
        match self {
            LazyFuture::Lazy(opt) => opt.unwrap(),
            LazyFuture::Chan(fut) => fut.get(),
        }
    }

    fn try_get(&mut self) -> Option<T> {
        match self {
            LazyFuture::Lazy(opt) => opt.take(),
            LazyFuture::Chan(fut) => fut.try_get(),
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
            if let Some(res) = self.try_get() {
                worker.stats.num_tasks_executed.increment(num_tasks_executed);
                return res;
            }
        }
    }
}

impl<T> LazyPromise<T> {
    pub fn new(future: &mut LazyFuture<T>) -> LazyPromise<T> {
        LazyPromise::Lazy(future)
    }

    pub fn promote(self) -> LazyPromise<T> {
        let (sender, receiver) = channel();
        match self {
            LazyPromise::Lazy(fut) => unsafe {
                *fut = LazyFuture::Chan(Future(receiver));
            }
            LazyPromise::Chan(_) => {
                // Something went wrong
                panic!();
            }
        }
        LazyPromise::Chan(Promise(sender))
    }

    pub fn set(self, value: T) {
        match self {
            LazyPromise::Lazy(fut) => unsafe {
                match *fut {
                    LazyFuture::Lazy(ref mut opt) => {
                        assert!(opt.is_none());
                        *opt = Some(value);
                    }
                    LazyFuture::Chan(_) => {
                        // Something went wrong
                        panic!();
                    }
                }
            }
            LazyPromise::Chan(fut) => {
                fut.set(value);
            }
        }
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
    fn future_promise_lazy() {
        let mut f = LazyFuture::new();
        let p = LazyPromise::new(&mut f);
        p.set(1);
        assert_eq!(f.get(), 1);
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

    #[test]
    fn future_promise_thread_lazy() {
        let mut f1 = LazyFuture::new();
        let p1 = LazyPromise::new(&mut f1).promote();

        let t = thread::spawn(|| {
            let mut f2 = LazyFuture::new();
            let p2 = LazyPromise::new(&mut f2).promote();
            p1.set(("ping", p2));
            assert_eq!(f2.get(), "pong");
        });

        let (msg, p2) = f1.get();
        assert_eq!(msg, "ping");
        p2.set("pong");
        t.join().unwrap();
    }
}

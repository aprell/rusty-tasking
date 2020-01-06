use std::sync::mpsc::{channel, Sender, Receiver};

use crate::worker::{Tasks, Worker};

// Storing closures requires generics and trait bounds. All closures implement
// at least one of the traits `Fn`, `FnMut`, or `FnOnce`. For instance, a
// closure that implements `FnMut` may capture variables by reference or
// mutable reference.

// From TRPL: "[...] we need `Send` to transfer the closure from one thread to
// another and `'static` (a lifetime bound) because we don’t know how long the
// thread will take to execute."
pub type Thunk<T> = dyn FnMut() -> T + Send + 'static;

// `Send` is a supertrait of `Task`, which means that only those task types
// that can be sent between threads safely are allowed to implement `Task`.
pub trait Task: Send {
    fn run(self: Box<Self>);
}

// A task with return type `T`
pub struct Async<T> {
    task: Box<Thunk<T>>,
    promise: Option<Promise<T>>,
}

impl Async<()> {
    // Constructor for tasks without return values (cannot overload `new`)
    pub fn task(task: Box<Thunk<()>>) -> Async<()> {
        Async { task, promise: None }
    }
}

impl<T> Async<T> {
    // Constructor for tasks with return values
    pub fn future(task: Box<Thunk<T>>) -> (Async<T>, Future<T>)  {
        let (sender, receiver) = channel();
        let promise = Some(Promise(sender));
        (Async { task, promise }, Future(receiver))
    }

    pub fn run(mut self) {
        let result = (self.task)();
        if let Some(promise) = self.promise {
            promise.set(result)
        }
    }
}

use std::fmt;

impl<T> fmt::Debug for Async<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (*self).promise {
            Some(_) => write!(f, "<Future>"),
            None => write!(f, "<Task>"),
        }
    }
}

impl<T> Task for Async<T> where T: Send {
    fn run(self: Box<Async<T>>) {
        (*self).run();
    }
}

// Futures and promises

pub struct Future<T>(Receiver<T>);

impl<T> Future<T> {
    // Block until result is available
    pub fn get(self) -> T {
        self.0.recv().unwrap()
    }

    // pub?
    fn try_get(&self) -> Option<T> {
        self.0.try_recv().ok()
    }

    // Try to overlap waiting with useful work
    pub fn wait(self) -> T {
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

pub struct Promise<T>(Sender<T>);

impl<T> Promise<T> {
    pub fn set(self, result: T) {
        self.0.send(result).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use super::*;

    // From TRPL: "The golden rule of dynamically sized types is that we must
    // always put values of dynamically sized types behind a pointer of some
    // kind."
    struct SimpleTask<T>(Box<Thunk<T>>);

    impl<T> SimpleTask<T> {
        fn new(task: Box<Thunk<T>>) -> SimpleTask<T> {
            SimpleTask(task)
        }

        fn run(mut self) {
            // Ignore result
            let _ = self.0();
        }
    }

    impl<T> Task for SimpleTask<T> {
        fn run(mut self: Box<SimpleTask<T>>) {
            // Ignore result
            let _ = (*self).0();
        }
    }

    #[test]
    fn simple_task() {
        // Unboxed task + boxed closure
        let a = SimpleTask::new(Box::new(|| 1));
        a.run();
        // `a` has been consumed

        // Boxed task + boxed closure
        let a = Box::new(SimpleTask::new(Box::new(|| 1)));
        a.run();
        // `a` has been consumed
    }

    #[test]
    fn simple_task_to_thread() {
        // Unboxed task + boxed closure
        let a = SimpleTask::new(Box::new(|| 1));
        thread::spawn(move || a.run()).join().unwrap();

        // Boxed task + boxed closure
        let a = Box::new(SimpleTask::new(Box::new(|| 1)));
        thread::spawn(move || a.run()).join().unwrap();
    }

    #[test]
    fn simple_unboxed_task_collection() {
        // Three tasks with different types
        let a = SimpleTask::new(Box::new(|| ()));
        let b = SimpleTask::new(Box::new(|| 1));
        let c = SimpleTask::new(Box::new(|| 1.2));

        // A vector of unboxed trait objects
        let mut v: Vec<&dyn Task> = vec![&a, &b, &c];

        while let Some(_t) = v.pop() {
            // `_t` has type `&Task`
            // Can we do something useful with `_t`?
        }
    }

    #[test]
    fn simple_boxed_task_collection() {
        // Three tasks with different types
        let a = SimpleTask::new(Box::new(|| ()));
        let b = SimpleTask::new(Box::new(|| 1));
        let c = SimpleTask::new(Box::new(|| 1.2));

        // A vector of boxed trait objects
        let mut v: Vec<Box<dyn Task>> = vec![
            Box::new(a),
            Box::new(b),
            Box::new(c),
        ];

        while let Some(t) = v.pop() {
            // `t` has type `Box<Task>`, which can be moved into `run`
            t.run();
        }
    }

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

    #[test]
    fn async_task() {
        let a = Async::task(Box::new(|| ()));
        a.run();
        // `a` has been consumed

        let (a, f) = Async::future(Box::new(|| 3.14));
        a.run();
        // `a` has been consumed
        assert_eq!(f.get(), 3.14);
    }

    #[test]
    fn async_task_to_thread() {
        let (a, f) = Async::future(Box::new(|| "hi"));
        let t = thread::spawn(|| a.run());
        assert_eq!(f.get(), "hi");
        t.join().unwrap();
    }
}

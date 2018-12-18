use std::sync::mpsc::{Sender, Receiver};

// Storing closures requires generics and trait bounds. All closures implement
// at least one of the traits `Fn`, `FnMut`, or `FnOnce`. For instance, a
// closure that implements `FnMut` may capture variables by reference or
// mutable reference.

// From TRPL: "[...] we need `Send` to transfer the closure from one thread to
// another and `'static` (a lifetime bound) because we don’t know how long the
// thread will take to execute."
pub type Thunk<T> = FnMut() -> T + Send + 'static;

// `Send` is a supertrait of `Task`, which means that only those task types
// that can be sent between threads safely are allowed to implement `Task`.
pub trait Task: Send {
    fn run(self: Box<Self>);
}

// Futures and promises

pub struct Future<T>(Receiver<T>);

impl<T> Future<T> {
    // Block until result is available
    pub fn get(self) -> T {
        self.0.recv().unwrap()
    }

    // Try to overlap waiting with useful work
    pub fn wait(self) -> T {
        unimplemented!();
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
    use std::sync::mpsc;
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
        let mut v: Vec<&Task> = vec![&a, &b, &c];

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
        let mut v: Vec<Box<Task>> = vec![
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
        let (sender, receiver) = mpsc::channel();
        Promise(sender).set(1);
        assert_eq!(Future(receiver).get(), 1);
    }

    #[test]
    fn future_promise_thread() {
        let (sender1, receiver1) = mpsc::channel();

        let t = thread::spawn(|| {
            let (sender2, receiver2) = mpsc::channel();
            Promise(sender1).set(("ping", Promise(sender2)));
            assert_eq!(Future(receiver2).get(), "pong");
        });

        let (msg, promise) = Future(receiver1).get();
        assert_eq!(msg, "ping");
        promise.set("pong");
        t.join().unwrap();
    }
}

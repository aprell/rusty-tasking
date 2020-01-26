use crate::future::Promise;

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
    fn promote(&mut self);
}

// A task with return type `T`
pub struct Async<T> {
    task: Box<Thunk<T>>,
    promise: Option<Promise<T>>,
}

impl<T> Async<T> {
    // Constructor for tasks with and without return values
    pub fn new(task: Box<Thunk<T>>, promise: Option<Promise<T>>) -> Async<T> {
        Async { task, promise }
    }

    pub fn run(mut self) {
        let result = (self.task)();
        if let Some(promise) = self.promise {
            promise.set(result)
        }
    }

    pub fn promote(&mut self) {
        if let Some(ref mut promise) = self.promise {
            promise.promote();
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

    fn promote(&mut self) {
        (*self).promote();
    }
}

#[cfg(test)]
mod tests {
    use crate::future::{Future, ToPromise};
    use std::sync::mpsc::channel;
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

        fn promote(&mut self) {}
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
    fn async_task() {
        let a = Async::new(Box::new(|| ()), None);
        a.run();
        // `a` has been consumed
    }

    #[test]
    fn async_future() {
        let (sender, receiver) = channel();
        let a = Async::new(Box::new(|| 3.14), sender.to_promise());
        a.run();
        // `a` has been consumed
        assert_eq!(Future::Chan(receiver).get(), 3.14);
    }

    #[test]
    fn async_future_lazy() {
        let mut f = Future::Lazy(None);
        let a = Async::new(Box::new(|| 3.14), (&mut f).to_promise());
        a.run();
        // `a` has been consumed
        assert_eq!(f.get(), 3.14);
    }

    #[test]
    fn async_future_to_thread() {
        let (sender, receiver) = channel();
        let a = Async::new(Box::new(|| "hi"), sender.to_promise());
        let t = thread::spawn(|| a.run());
        assert_eq!(Future::Chan(receiver).get(), "hi");
        t.join().unwrap();
    }

    #[test]
    fn async_future_to_thread_lazy() {
        let mut f = Future::Lazy(None);
        let mut a = Async::new(Box::new(|| "hi"), (&mut f).to_promise());
        a.promote();
        let t = thread::spawn(|| a.run());
        t.join().unwrap();
        assert_eq!(f.get(), "hi");
    }
}

use std::sync::mpsc::channel;

use crate::future::{Future, Promise};

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
        let promise = Some(Promise::Chan(sender));
        (Async { task, promise }, Future::Chan(receiver))
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

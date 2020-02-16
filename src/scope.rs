use crate::atomic;
use crate::stats;
use crate::worker::{Tasks, Worker};
use std::cell::{Ref, RefMut, RefCell};
use std::collections::LinkedList;
use std::sync::Arc;
use std::sync::atomic::Ordering;

// We use a linked list to avoid invalidating references returned by
// Scope::current()
thread_local! {
    static SCOPE: RefCell<LinkedList<Scope>> = RefCell::new(LinkedList::new());
}

pub enum TaskCount {
    Private(stats::Count),
    Shared(Arc<atomic::Count>),
}

impl TaskCount {
    pub fn new() -> TaskCount {
        TaskCount::Private(stats::Count::new(0))
    }

    pub fn get(&self) -> u32 {
        match self {
            TaskCount::Private(count) => count.get(),
            TaskCount::Shared(count) => count.get(Ordering::Relaxed),
        }
    }

    // Returns the previous value
    pub fn inc(&self) -> u32 {
        match self {
            TaskCount::Private(count) => {
                let n = count.get();
                count.inc();
                n
            }
            TaskCount::Shared(count) => {
                count.inc(Ordering::Relaxed)
            }
        }
    }

    // Returns the previous value
    pub fn dec(&self) -> u32 {
        match self {
            TaskCount::Private(count) => {
                let n = count.get();
                count.dec();
                n
            }
            TaskCount::Shared(count) => {
                count.dec(Ordering::Relaxed)
            }
        }
    }
}

pub struct NumTasks(RefCell<TaskCount>);

impl NumTasks {
    pub fn new() -> NumTasks {
        NumTasks::with_count(TaskCount::new())
    }

    pub fn with_count(count: TaskCount) -> NumTasks {
        NumTasks(RefCell::new(count))
    }

    pub fn get(&self) -> u32 {
        self.0.borrow().get()
    }

    // Returns the previous value
    pub fn inc(&self) -> u32 {
        self.0.borrow().inc()
    }

    // Returns the previous value
    pub fn dec(&self) -> u32 {
        self.0.borrow().dec()
    }

    pub fn borrow(&self) -> Ref<TaskCount> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<TaskCount> {
        self.0.borrow_mut()
    }
}

pub struct Scope {
    level: u32,
    pub num_tasks: NumTasks,
}

impl Scope {
    pub fn init() {
        Scope::with_level(0).push();
    }

    fn with_level(level: u32) -> Scope {
        Scope { level, num_tasks: NumTasks::new() }
    }

    pub fn with_num_tasks(num_tasks: NumTasks) -> Scope {
        let scope = Scope::current();
        assert_ne!(scope as *const Scope, std::ptr::null());
        Scope { level: scope.level + 1, num_tasks }

    }

    pub fn new() -> Scope {
        let scope = Scope::current();
        assert_ne!(scope as *const Scope, std::ptr::null());
        Scope::with_level(scope.level + 1)
    }

    pub fn push(self) {
        SCOPE.with(|scope| {
            let mut scope = scope.borrow_mut();
            scope.push_front(self);
        });
    }

    pub fn pop() -> Option<Scope> {
        SCOPE.with(|scope| {
            let mut scope = scope.borrow_mut();
            scope.pop_front()
        })
    }

    pub fn enter() {
        Scope::new().push();
    }

    pub fn leave() {
        Scope::current().wait();
        assert_eq!(Scope::current().num_tasks.get(), 0);
        Scope::pop().unwrap();
    }

    // Get a reference to the current scope
    pub fn current<'a>() -> &'a Scope {
        SCOPE.with(|scope| {
            // See `Worker::current`
            let ptr = match scope.borrow().front() {
                Some(ref scope) => *scope as *const Scope,
                None => std::ptr::null(),
            };
            // (2) Convert this pointer to a borrowed reference
            unsafe { &*ptr }
        })
    }

    pub fn share(&self) -> Arc<atomic::Count> {
        let count = match &*self.num_tasks.borrow() {
            TaskCount::Private(count) => count.get(),
            TaskCount::Shared(count) => return Arc::clone(&count),
        };
        let count = Arc::new(atomic::Count::new(count));
        let clone = Arc::clone(&count);
        *self.num_tasks.borrow_mut() = TaskCount::Shared(count);
        clone
    }

    pub fn wait(&self) {
        if self.num_tasks.get() == 0 { return; }

        let worker = Worker::current();
        let mut num_tasks_executed = 0;

        while let Some(task) = worker.pop() {
            worker.try_handle_steal_request();
            task.run();
            num_tasks_executed += 1;
            if self.num_tasks.get() == 0 {
                worker.stats.num_tasks_executed.add(num_tasks_executed);
                return;
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
            if self.num_tasks.get() == 0 {
                worker.stats.num_tasks_executed.add(num_tasks_executed);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use super::*;

    #[test]
    fn push_pop() {
        Scope::init();
        for i in 1..10 {
            Scope::new().push();
            assert_eq!(Scope::current().level, i);
        }

        for i in 1..10 {
            if let Some(scope) = Scope::pop() {
                assert_eq!(scope.level, 9-i+1);
            }
        }

        assert_eq!(Scope::current().level, 0);
    }

    #[test]
    fn inc_dec() {
        Scope::init();
        let scope = Scope::current();
        assert_eq!(scope.num_tasks.inc(), 0);
        assert_eq!(scope.num_tasks.inc(), 1);
        assert_eq!(scope.num_tasks.get(), 2);

        assert_eq!(scope.num_tasks.dec(), 2);
        assert_eq!(scope.num_tasks.dec(), 1);
        assert_eq!(scope.num_tasks.get(), 0);
    }

    #[test]
    fn inc_dec_threads() {
        Scope::init();
        let scope = Scope::current();
        let mut threads = Vec::with_capacity(3);

        for _ in 0..3 {
            let count = scope.share();
            threads.push(thread::spawn(move || {
                for i in 0..100 {
                    count.inc(Ordering::Relaxed);
                    if i % 2 == 0 {
                        count.dec(Ordering::Relaxed);
                    }
                }
            }));
        }

        for _ in 0..50 {
            scope.num_tasks.dec();
        }

        for t in threads {
            t.join().unwrap();
        }

        assert_eq!(scope.num_tasks.get(), 100);
    }
}

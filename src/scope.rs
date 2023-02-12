use crate::atomic;
use crate::stats;
use crate::worker::{Tasks, Worker};
use std::cell::{Ref, RefMut, RefCell};
use std::collections::LinkedList;
use std::sync::Arc;

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
    pub fn new() -> Self {
        Self::Private(stats::Count::new(0))
    }

    pub fn get(&self) -> u32 {
        match self {
            Self::Private(count) => count.get(),
            Self::Shared(count) => count.get(),
        }
    }

    // Returns the previous value
    pub fn inc(&self) -> u32 {
        match self {
            Self::Private(count) => {
                let n = count.get();
                count.inc();
                n
            }
            Self::Shared(count) => {
                count.inc()
            }
        }
    }

    // Returns the previous value
    pub fn dec(&self) -> u32 {
        match self {
            Self::Private(count) => {
                let n = count.get();
                count.dec();
                n
            }
            Self::Shared(count) => {
                count.dec()
            }
        }
    }
}

pub struct NumTasks(RefCell<TaskCount>);

impl NumTasks {
    pub fn new() -> Self {
        Self::with_count(TaskCount::new())
    }

    pub fn with_count(count: TaskCount) -> Self {
        Self(RefCell::new(count))
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
        Self::with_level(0).push();
    }

    fn with_level(level: u32) -> Self {
        Self { level, num_tasks: NumTasks::new() }
    }

    pub fn with_num_tasks(num_tasks: NumTasks) -> Self {
        let scope = Self::current();
        assert_ne!(scope as *const Scope, std::ptr::null());
        Self { level: scope.level + 1, num_tasks }

    }

    pub fn new() -> Self {
        let scope = Self::current();
        assert_ne!(scope as *const Self, std::ptr::null());
        Self::with_level(scope.level + 1)
    }

    pub fn push(self) {
        SCOPE.with(|scope| {
            let mut scope = scope.borrow_mut();
            scope.push_front(self);
        });
    }

    pub fn pop() -> Option<Self> {
        SCOPE.with(|scope| {
            let mut scope = scope.borrow_mut();
            scope.pop_front()
        })
    }

    pub fn enter() {
        Self::new().push();
    }

    pub fn leave() {
        Self::current().wait();
        assert_eq!(Self::current().num_tasks.get(), 0);
        Self::pop().unwrap();
    }

    // Get a reference to the current scope
    pub fn current<'a>() -> &'a Self {
        SCOPE.with(|scope| {
            // See `Worker::current`
            let ptr = match scope.borrow().front() {
                Some(scope) => scope as *const Self,
                None => std::ptr::null(),
            };
            // Convert this pointer to a borrowed reference
            unsafe { &*ptr }
        })
    }

    pub fn share(&self) -> Arc<atomic::Count> {
        let count = match &*self.num_tasks.borrow() {
            TaskCount::Private(count) => count.get(),
            TaskCount::Shared(count) => return Arc::clone(count),
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
                    count.inc();
                    if i % 2 == 0 {
                        count.dec();
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

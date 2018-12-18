use rusty_tasking::deque::{Deque, Steal, StealMany};
use rusty_tasking::task::{Task, Thunk};
use std::thread;

struct SimpleTask<T>(Box<Thunk<T>>);

impl<T> SimpleTask<T> {
    fn new(task: Box<Thunk<T>>) -> SimpleTask<T> {
        SimpleTask(task)
    }
}

impl<T> Task for SimpleTask<T> {
    fn run(mut self: Box<SimpleTask<T>>) {
        // Ignore result
        let _ = (*self).0();
    }
}

type TaskDeque = Deque<Box<Task>>;

fn setup() -> TaskDeque {
    let mut deque: TaskDeque = Deque::new();

    deque.push(Box::new(SimpleTask::new(Box::new(|| ()))));
    deque.push(Box::new(SimpleTask::new(Box::new(|| 1))));
    deque.push(Box::new(SimpleTask::new(Box::new(|| 1.2))));

    deque
}

#[test]
fn task_deque_pop() {
    let mut deque = setup();

    while let Some(t) = deque.pop() {
        t.run();
    }

    assert!(deque.is_empty());
}

#[test]
fn task_deque_steal() {
    let mut deque = setup();

    while let Some(t) = deque.steal() {
        thread::spawn(move || t.run()).join().unwrap();
    }

    assert!(deque.is_empty());
}

#[test]
fn task_deque_steal_many() {
    let mut deque = setup();

    while let Some(mut loot) = deque.steal_many() {
        thread::spawn(move || {
            while let Some(t) = loot.pop() {
                t.run();
            }
        }).join().unwrap();
    }

    assert!(deque.is_empty());
}

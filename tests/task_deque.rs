use rusty_tasking::deque::{Deque, Steal, StealMany};
use rusty_tasking::task::{Async, Future, Task, Thunk};
use std::thread;

type TaskDeque = Deque<Box<Task>>;

fn future<T>(thunk: Box<Thunk<T>>, deque: &mut TaskDeque) -> Future<T>
where T: Send + 'static
{
    let (task, future) = Async::future(thunk);
    deque.push(Box::new(task));
    future
}

#[test]
fn task_deque_pop() {
    let mut deque: TaskDeque = Deque::new();

    let f1 = future(Box::new(|| ()), &mut deque);
    let f2 = future(Box::new(|| 1), &mut deque);
    let f3 = future(Box::new(|| 1.2), &mut deque);

    while let Some(t) = deque.pop() {
        t.run();
    }

    assert_eq!(f1.get(), ());
    assert_eq!(f2.get(), 1);
    assert_eq!(f3.get(), 1.2);

    assert!(deque.is_empty());
}

#[test]
fn task_deque_steal() {
    let mut deque: TaskDeque = Deque::new();

    let f1 = future(Box::new(|| ()), &mut deque);
    let f2 = future(Box::new(|| 1), &mut deque);
    let f3 = future(Box::new(|| 1.2), &mut deque);

    while let Some(t) = deque.steal() {
        thread::spawn(move || t.run()).join().unwrap();
    }

    assert_eq!(f1.get(), ());
    assert_eq!(f2.get(), 1);
    assert_eq!(f3.get(), 1.2);

    assert!(deque.is_empty());
}

#[test]
fn task_deque_steal_many() {
    let mut deque: TaskDeque = Deque::new();

    let f1 = future(Box::new(|| ()), &mut deque);
    let f2 = future(Box::new(|| 1), &mut deque);
    let f3 = future(Box::new(|| 1.2), &mut deque);

    while let Some(mut loot) = deque.steal_many() {
        thread::spawn(move || {
            while let Some(t) = loot.pop() {
                t.run();
            }
        }).join().unwrap();
    }

    assert_eq!(f1.get(), ());
    assert_eq!(f2.get(), 1);
    assert_eq!(f3.get(), 1.2);

    assert!(deque.is_empty());
}

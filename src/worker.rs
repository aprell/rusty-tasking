use std::cell::RefCell;
use std::sync::mpsc::{channel, Sender, Receiver};

use crate::deque::*;
use crate::task::*;

type TaskDeque = Deque<Box<Task>>;

pub struct Worker {
    id: usize,
    deque: RefCell<TaskDeque>,
    tasks: (Sender<Tasks>, Receiver<Tasks>),
}

#[derive(Debug)]
pub struct StealRequest {
    thief: usize,
    steal_many: bool,
    response: Sender<Tasks>,
}

// Possible responses to a steal request
enum Tasks {
    None,
    One(Box<Task>),
    Many(TaskDeque),
    Exit,
}

impl Worker {
    pub fn new(id: usize) -> Worker {
        Worker {
            id,
            deque: RefCell::new(Deque::new()),
            tasks: channel(),
        }
    }

    pub fn handle_steal_request(&self, req: StealRequest) {
        let response = req.response;
        if req.steal_many {
            match self.deque.borrow_mut().steal_many() {
                Some(tasks) => response.send(Tasks::Many(tasks)).unwrap(),
                None => response.send(Tasks::None).unwrap(),
            }
        } else {
            match self.deque.borrow_mut().steal() {
                Some(task) => response.send(Tasks::One(task)).unwrap(),
                None => response.send(Tasks::None).unwrap(),
            }
        }
    }

    pub fn has_tasks(&self) -> bool {
        !self.deque.borrow_mut().is_empty()
    }

    pub fn push(&self, task: Box<Task>) {
        self.deque.borrow_mut().push(task);
    }

    pub fn pop(&self) -> Option<Box<Task>> {
        self.deque.borrow_mut().pop()
    }

    // Regular worker loop
    pub fn go(&self) {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use super::*;

    #[test]
    fn create_and_shutdown() {
        let mut workers = Vec::with_capacity(2);
        let (sender, receiver) = channel();

        // Create two workers that send steal requests to us
        for i in 0..2 {
            let sender = sender.clone();
            workers.push(thread::spawn(move || {
                let worker = Worker::new(i+1);
                // ===== Worker loop =====
                loop {
                    sender.send(StealRequest {
                        thief: worker.id,
                        steal_many: false,
                        response: worker.tasks.0.clone(),
                    }).unwrap();
                    match worker.tasks.1.recv().unwrap() {
                        Tasks::None => (),
                        Tasks::Exit => break,
                        _ => unreachable!(),
                    }
                }
            }));
        }

        // Respond to the first ten steal requests with `Tasks::None`
        for _ in 0..10 {
            let req = receiver.recv().unwrap();
            req.response.send(Tasks::None).unwrap();
        }

        // Respond with `Tasks::Exit` and join the workers
        for _ in 0..2 {
            let req = receiver.recv().unwrap();
            req.response.send(Tasks::Exit).unwrap();
        }

        for worker in workers {
            worker.join().unwrap();
        }
    }

    #[test]
    fn distribute_tasks() {
        let mut workers = Vec::with_capacity(2);
        let (sender, receiver) = channel();

        // Create two workers that send steal requests to us
        for i in 0..2 {
            let sender = sender.clone();
            workers.push(thread::spawn(move || {
                let worker = Worker::new(i+1);
                // ===== Worker loop =====
                loop {
                    // Worker 1 asks for single tasks, worker 2 asks for more
                    sender.send(StealRequest {
                        thief: worker.id,
                        steal_many: if worker.id == 1 { false } else { true },
                        response: worker.tasks.0.clone(),
                    }).unwrap();
                    match worker.tasks.1.recv().unwrap() {
                        Tasks::None => (),
                        Tasks::One(task) => {
                            assert_eq!(worker.id, 1);
                            task.run();
                        }
                        Tasks::Many(mut loot) => {
                            assert_eq!(worker.id, 2);
                            while let Some(task) = loot.pop() {
                                task.run();
                            }
                        }
                        Tasks::Exit => break,
                    }
                }
            }));
        }

        // Create a few dummy tasks
        let master = Worker::new(0);
        for _ in 0..10 {
            let task = Async::task(Box::new(|| ()));
            master.push(Box::new(task));
        }

        // Distribute tasks until deque is empty
        while master.has_tasks() {
            let req = receiver.recv().unwrap();
            master.handle_steal_request(req);
            // `req` consumed
        }

        // Ask workers to terminate
        for _ in 0..2 {
            let req = receiver.recv().unwrap();
            req.response.send(Tasks::Exit).unwrap();
        }

        for worker in workers {
            worker.join().unwrap();
        }
    }

    thread_local! {
        // See interior mutability pattern
        static ID: RefCell<usize> = RefCell::new(0);
    }

    fn get_id() -> usize {
        ID.with(|id| { *id.borrow() })
    }

    fn set_id(new_id: usize) {
        ID.with(|id| { *id.borrow_mut() = new_id; })
    }

    #[test]
    fn thread_local_data() {
        assert_eq!(get_id(), 0);
        set_id(42);
        assert_eq!(get_id(), 42);
    }
}

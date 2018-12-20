use std::sync::mpsc::{channel, Sender, Receiver};

pub struct Worker {
    id: usize,
    tasks: (Sender<Tasks>, Receiver<Tasks>),
}

#[derive(Debug)]
struct StealRequest {
    thief: usize,
    steal_many: bool,
    response: Sender<Tasks>,
}

// Possible responses to a steal request
enum Tasks {
    None,
    Exit,
}

impl Worker {
    pub fn new(id: usize) -> Worker {
        Worker { id, tasks: channel() }
    }

    // Regular worker loop
    pub fn go(&self) {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
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

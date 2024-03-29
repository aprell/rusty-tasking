use crate::deque::*;
use crate::stats::*;
use crate::task::*;
use rand::Rng;
use std::cell::RefCell;
use std::sync::mpsc::{channel, Sender, Receiver};

#[derive(Debug)]
pub struct StealRequest {
    thief: usize,
    steal_many: bool,
    response: Sender<Tasks>,
}

// Possible responses to a steal request
pub enum Tasks {
    None,
    One(Box<dyn Task>),
    Many(TaskDeque),
    Exit,
}

type TaskDeque = Deque<Box<dyn Task>>;

struct WorkerChannels {
    steal_requests: Receiver<StealRequest>,
    tasks: (Sender<Tasks>, Receiver<Tasks>),
}

pub struct Worker {
    pub id: usize,
    deque: RefCell<TaskDeque>,
    channels: WorkerChannels,
    coworkers: Vec<Coworker>,
    children: Vec<Sender<Tasks>>,
    pub stats: Stats,
}

thread_local! {
    static WORKER: RefCell<Option<Worker>> = RefCell::new(None);
}

impl Worker {
    pub fn new(
        id: usize,
        steal_requests: Receiver<StealRequest>,
        coworkers: Vec<Coworker>
    ) -> Self {
        let mut worker = Self {
            id,
            deque: RefCell::new(Deque::new()),
            channels: WorkerChannels { steal_requests, tasks: channel() },
            coworkers: coworkers.into_iter().filter(|c| c.id != id).collect(),
            children: vec![],
            stats: Stats::new(),
        };

        if id > 0 {
            // Determine parent and send a dummy steal request
            let parent_id = (id - 1) / 2;
            let parent = &worker.coworkers[parent_id];
            parent.send_steal_request(StealRequest {
                thief: id,
                steal_many: false,
                response: worker.channels.tasks.0.clone(),
            });
        }

        // Receive dummy steal requests from children
        let num_workers = worker.coworkers.len() + 1;
        let children = (2 * id + 1, 2 * id + 2);
        if children.0 < num_workers {
            let req = worker.channels.steal_requests.recv().unwrap();
            worker.children.push(req.response);
            if children.1 < num_workers {
                let req = worker.channels.steal_requests.recv().unwrap();
                worker.children.push(req.response);
            }
        }

        worker
    }

    // Move stack-allocated worker to thread-local storage
    pub fn make_current(self) {
        WORKER.with(|worker| {
            let mut worker = worker.borrow_mut();
            *worker = Some(self);
        });
    }

    // Get a handle to the current worker
    pub fn current<'a>() -> &'a Self {
        WORKER.with(|worker| {
            // Any attempt of trying to borrow from `worker` and have it
            // outlive the closure will fail -> `unsafe` to the rescue
            // (1) Get a raw pointer to thread-local `WORKER`
            let ptr = match worker.borrow().as_ref() {
                Some(worker) => worker as *const Self,
                None => std::ptr::null(),
            };
            // (2) Convert this pointer to a borrowed reference
            unsafe { &*ptr }
        })
    }

    pub fn select_victim(&self, id: usize) -> Option<&Coworker> {
        self.coworkers.iter().find(|&c| c.id == id)
    }

    // Send steal request to random worker != self
    pub fn send_steal_request(&self, req: StealRequest) {
        let rand_idx: usize = rand::thread_rng().gen_range(0, self.coworkers.len());
        let victim = &self.coworkers[rand_idx];
        victim.send_steal_request(req);
    }

    pub fn steal_one(&self) -> StealResponse {
        self.send_steal_request(StealRequest {
            thief: self.id,
            steal_many: false,
            response: self.channels.tasks.0.clone(),
        });
        StealResponse(&self.channels.tasks.1)
    }

    pub fn steal_many(&self) -> StealResponse {
        self.send_steal_request(StealRequest {
            thief: self.id,
            steal_many: true,
            response: self.channels.tasks.0.clone(),
        });
        StealResponse(&self.channels.tasks.1)
    }

    pub fn handle_steal_request(&self, req: StealRequest) {
        let response = req.response;
        if req.steal_many {
            match self.deque.borrow_mut().steal_many() {
                Some(mut tasks) => {
                    for task in tasks.iter_mut() {
                        task.promote();
                    }
                    response.send(Tasks::Many(tasks)).unwrap();
                },
                None => response.send(Tasks::None).unwrap(),
            }
        } else {
            match self.deque.borrow_mut().steal() {
                Some(mut task) => {
                    task.promote();
                    response.send(Tasks::One(task)).unwrap();
                },
                None => response.send(Tasks::None).unwrap(),
            }
        }
    }

    pub fn try_handle_steal_request(&self) {
        let req = self.channels.steal_requests.try_recv();
        if let Ok(req) = req {
            self.handle_steal_request(req);
        }
    }

    pub fn has_tasks(&self) -> bool {
        !self.deque.borrow_mut().is_empty()
    }

    pub fn push(&self, task: Box<dyn Task>) {
        self.deque.borrow_mut().push(task);
    }

    pub fn pop(&self) -> Option<Box<dyn Task>> {
        self.deque.borrow_mut().pop()
    }

    pub fn finalize(&self) {
        for child in self.children.iter() {
            child.send(Tasks::Exit).unwrap();
        }
    }

    // General worker loop
    pub fn go(&self) {
        let mut num_tasks_executed = 0;
        loop {
            // (1) Do local work
            while let Some(task) = self.pop() {
                self.try_handle_steal_request();
                task.run();
                num_tasks_executed += 1;
            }
            // (2) Request/steal work
            match self.steal_one().wait() {
                Tasks::None => (),
                Tasks::One(task) => {
                    task.run();
                    num_tasks_executed += 1;
                }
                Tasks::Many(tasks) => {
                    let _ = self.deque.replace(tasks);
                }
                Tasks::Exit => {
                    assert!(self.deque.borrow().is_empty());
                    break;
                }
            }
        }
        self.stats.num_tasks_executed.add(num_tasks_executed);
    }
}

#[derive(Debug)]
pub struct Coworker {
    id: usize,
    steal_requests: Sender<StealRequest>,
}

impl Coworker {
    pub fn new(id: usize, steal_requests: Sender<StealRequest>) -> Self {
        Self { id, steal_requests }
    }

    pub fn send_steal_request(&self, req: StealRequest) {
        assert_ne!(self.id, req.thief);
        self.steal_requests.send(req).unwrap();
    }
}

impl Clone for Coworker {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            steal_requests: Sender::clone(&self.steal_requests),
        }
    }
}

// The result of asynchronous work stealing
pub struct StealResponse<'a>(&'a Receiver<Tasks>);

impl<'a> StealResponse<'a> {
    pub fn wait(self) -> Tasks {
        let worker = Worker::current();
        loop {
            match self.0.try_recv().ok() {
                Some(tasks) => break tasks,
                None => worker.try_handle_steal_request(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};
    use std::thread;
    use super::*;

    fn setup(num_workers: usize) -> (Vec<Receiver<StealRequest>>, Vec<Coworker>) {
        // `N` workers communicate using `N` channels
        let channels = (0..num_workers)
            .map(|_| channel())
            .collect::<Vec<(Sender<StealRequest>, _)>>();

        let coworkers = channels
            .iter()
            .enumerate()
            .map(|(i, (chan, _))| Coworker::new(i, Sender::clone(&chan)))
            .collect::<Vec<Coworker>>();

        let channels = channels
            .into_iter()
            .map(|(_, r)| r)
            .collect::<Vec<Receiver<StealRequest>>>();

        (channels, coworkers)
    }

    #[test]
    fn create_and_shutdown() {
        let mut workers = Vec::with_capacity(2);
        let (mut channels, coworkers) = setup(3);
        let barrier = Arc::new(Barrier::new(3));

        // Create two workers that send steal requests to us
        for i in 0..2 {
            let channel = channels.remove(1);
            let coworkers = coworkers.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                let worker = Worker::new(i+1, channel, coworkers);
                barrier.wait();
                // ===== Worker loop =====
                loop {
                    let victim = worker.select_victim(0).unwrap();
                    victim.send_steal_request(StealRequest {
                        thief: worker.id,
                        steal_many: false,
                        response: worker.channels.tasks.0.clone(),
                    });
                    match worker.channels.tasks.1.recv().unwrap() {
                        Tasks::None => (),
                        Tasks::Exit => break,
                        _ => unreachable!(),
                    }
                }
            }));
        }

        let leader = Worker::new(0, channels.remove(0), coworkers);
        barrier.wait();

        // Respond to the first ten steal requests with `Tasks::None`
        for _ in 0..10 {
            let req = leader.channels.steal_requests.recv().unwrap();
            req.response.send(Tasks::None).unwrap();
        }

        // Respond with `Tasks::Exit` and join the workers
        for _ in 0..2 {
            let req = leader.channels.steal_requests.recv().unwrap();
            req.response.send(Tasks::Exit).unwrap();
        }

        for worker in workers {
            worker.join().unwrap();
        }
    }

    #[test]
    fn distribute_tasks() {
        let mut workers = Vec::with_capacity(2);
        let (mut channels, coworkers) = setup(3);
        let barrier = Arc::new(Barrier::new(3));

        // Create two workers that send steal requests to us
        for i in 0..2 {
            let channel = channels.remove(1);
            let coworkers = coworkers.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                let worker = Worker::new(i+1, channel, coworkers);
                barrier.wait();
                // ===== Worker loop =====
                loop {
                    // Worker 1 asks for single tasks, worker 2 asks for more
                    let victim = worker.select_victim(0).unwrap();
                    victim.send_steal_request(StealRequest {
                        thief: worker.id,
                        steal_many: if worker.id == 1 { false } else { true },
                        response: worker.channels.tasks.0.clone(),
                    });
                    match worker.channels.tasks.1.recv().unwrap() {
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

        let leader = Worker::new(0, channels.remove(0), coworkers);
        barrier.wait();

        // Create a few dummy tasks
        for _ in 0..10 {
            let task = Async::new(Box::new(|| ()), None);
            leader.push(Box::new(task));
        }

        // Distribute tasks until deque is empty
        while leader.has_tasks() {
            let req = leader.channels.steal_requests.recv().unwrap();
            leader.handle_steal_request(req);
            // `req` consumed
        }

        // Ask workers to terminate
        for _ in 0..2 {
            let req = leader.channels.steal_requests.recv().unwrap();
            req.response.send(Tasks::Exit).unwrap();
        }

        for worker in workers {
            worker.join().unwrap();
        }
    }

    #[test]
    fn worker_communication() {
        let mut workers = Vec::with_capacity(2);
        let (mut channels, coworkers) = setup(3);

        // Create two workers that communicate with each other
        for i in 0..2 {
            let channel = channels.remove(1);
            let coworkers = coworkers.clone();
            workers.push(thread::spawn(move || {
                let worker = Worker::new(i+1, channel, coworkers);
                match worker.id {
                    // Worker 1 sends steal requests to worker 2
                    1 => {
                        loop {
                            let victim = worker.select_victim(2).unwrap();
                            victim.send_steal_request(StealRequest {
                                thief: worker.id,
                                steal_many: true,
                                response: worker.channels.tasks.0.clone(),
                            });
                            match worker.channels.tasks.1.recv().unwrap() {
                                Tasks::None => (),
                                Tasks::One(task) => task.run(),
                                Tasks::Many(mut loot) => {
                                    while let Some(task) = loot.pop() {
                                        task.run();
                                    }
                                }
                                Tasks::Exit => break,
                            }
                        }
                    }
                    // Worker 2 creates a few tasks and handles worker 1's
                    // steal requests
                    2 => {
                        for _ in 0..10 {
                            let task = Async::new(Box::new(|| ()), None);
                            worker.push(Box::new(task));
                        }
                        while worker.has_tasks() {
                            let req = worker.channels.steal_requests.recv().unwrap();
                            worker.handle_steal_request(req);
                        }
                        // Send `Tasks::Exit` to worker 1 and exit
                        let req = worker.channels.steal_requests.recv().unwrap();
                        req.response.send(Tasks::Exit).unwrap();
                    }
                    _ => unreachable!()
                }
            }));
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

    #[test]
    fn current_worker() {
        let mut workers = Vec::with_capacity(3);
        let (mut channels, coworkers) = setup(4);

        for i in 0..3 {
            let channel = channels.remove(1);
            let coworkers = coworkers.clone();
            workers.push(thread::spawn(move || {
                Worker::new(i+1, channel, coworkers).make_current();
                assert_eq!(Worker::current().id, i+1);
            }));
        }

        Worker::new(0, channels.remove(0), coworkers).make_current();
        assert_eq!(Worker::current().id, 0);

        for worker in workers {
            worker.join().unwrap();
        }
    }
}

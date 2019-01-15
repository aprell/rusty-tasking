use std::sync::{Arc, Barrier};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

use crate::worker::*;

pub struct Runtime {
    pub master: Worker,
    workers: Vec<thread::JoinHandle<()>>,
    barrier: Arc<Barrier>,
}

impl Runtime {
    pub fn init(num_workers: usize) -> Runtime {
        assert!(num_workers > 0);

        let mut workers = Vec::with_capacity(num_workers - 1);

        // `N` workers communicate using `N` channels
        let channels = (0..num_workers)
            .map(|_| channel())
            .collect::<Vec<(Sender<StealRequest>, _)>>();

        let coworkers = channels
            .iter()
            .enumerate()
            .map(|(i, (chan, _))| Coworker::new(i, Sender::clone(&chan)))
            .collect::<Vec<Coworker>>();

        let mut channels = channels
            .into_iter()
            .map(|(_, r)| r)
            .collect::<Vec<Receiver<StealRequest>>>();

        let barrier = Arc::new(Barrier::new(num_workers));

        for i in 1..num_workers {
            let channel = channels.remove(1);
            let coworkers = coworkers.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                let worker = Worker::new(i, channel, coworkers);
                barrier.wait();
                worker.go();
                barrier.wait();
            }));
        }

        let master = Worker::new(0, channels.remove(0), coworkers);
        barrier.wait();

        Runtime { master, workers, barrier }
    }

    pub fn join(self) {
        // Ask workers to terminate
        self.master.finalize();
        self.barrier.wait();

        // Join workers
        for worker in self.workers {
            worker.join().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_and_join() {
        for n in 1..4 {
            Runtime::init(n).join();
        }
    }
}

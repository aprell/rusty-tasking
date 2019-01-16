use std::sync::{Arc, Barrier};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

use crate::stats::*;
use crate::worker::*;

pub struct Runtime {
    pub master: Worker,
    workers: Vec<thread::JoinHandle<Stats>>,
    barrier: Arc<Barrier>,
    stats: Stats,
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
                worker.stats
            }));
        }

        let master = Worker::new(0, channels.remove(0), coworkers);
        barrier.wait();

        Runtime { master, workers, barrier, stats: Stats::new() }
    }

    pub fn join(mut self) -> Stats {
        // Ask workers to terminate
        self.master.finalize();
        self.stats += self.master.stats;
        self.barrier.wait();

        // Join workers
        for worker in self.workers {
            let worker_stats = worker.join().unwrap();
            self.stats += worker_stats;
        }

        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_and_join() {
        for n in 1..4 {
            let stats = Runtime::init(n).join();
            assert_eq!(stats.num_tasks_executed.get(), 0);
        }
    }
}

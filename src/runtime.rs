use std::sync::{Arc, Barrier, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

use crate::stats::*;
use crate::worker::*;

pub struct Runtime {
    pub master: &'static Worker,
    workers: Vec<thread::JoinHandle<()>>,
    barrier: Arc<Barrier>,
    stats: Arc<Mutex<Stats>>,
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
        let stats = Arc::new(Mutex::new(Stats::new()));

        for i in 1..num_workers {
            let channel = channels.remove(1);
            let coworkers = coworkers.clone();
            let barrier = Arc::clone(&barrier);
            let stats = Arc::clone(&stats);
            workers.push(thread::spawn(move || {
                Worker::new(i, channel, coworkers).make_current();
                let worker = Worker::current();
                barrier.wait();
                worker.go();
                worker.finalize();
                {
                    let stats = stats.lock().unwrap();
                    stats.update(&worker.stats);
                }
                barrier.wait();
                // worker.stats
                // ^^^^^^^^^^^^ cannot move out of borrowed content
            }));
        }

        Worker::new(0, channels.remove(0), coworkers).make_current();
        let master = Worker::current();
        barrier.wait();

        Runtime { master, workers, barrier, stats }
    }

    pub fn join(self) -> Stats {
        let master = self.master;
        assert_eq!(master.id, 0);

        // Ask workers to terminate
        master.finalize();
        {
            let stats = self.stats.lock().unwrap();
            stats.update(&master.stats);
        }
        self.barrier.wait();

        // Join workers
        for worker in self.workers {
            worker.join().unwrap();
        }

        // Unpack self.stats
        Arc::try_unwrap(self.stats)
            .expect("There should be only one reference left")
            .into_inner() // Consumes mutex, returning inner data
            .unwrap()
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

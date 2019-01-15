use rusty_tasking::task::{Async};
use rusty_tasking::worker::{StealRequest, Worker, Coworker};
use std::sync::{Arc, Barrier};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

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
fn random_stealing() {
    let mut workers = Vec::with_capacity(3);
    let (mut channels, coworkers) = setup(4);
    let barrier = Arc::new(Barrier::new(4));

    // Create three additional workers
    for i in 0..3 {
        let channel = channels.remove(1);
        let coworkers = coworkers.clone();
        let barrier = Arc::clone(&barrier);
        workers.push(thread::spawn(move || {
            let worker = Worker::new(i+1, channel, coworkers);
            barrier.wait();
            worker.go();
            barrier.wait();
        }));
    }

    let master = Worker::new(0, channels.remove(0), coworkers);
    barrier.wait();

    for _ in 0..99 {
        let task = Async::task(Box::new(|| ()));
        master.push(Box::new(task));
    }

    while master.has_tasks() {
        master.try_handle_steal_request();
        match master.pop() {
            Some(task) => task.run(),
            None => break,
        }
    }

    // Ask workers to terminate
    master.finalize();
    barrier.wait();

    for worker in workers {
        worker.join().unwrap();
    }
}

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Release, Relaxed};

// One-shot channel from Chapter 5 of Rust Atomics and Locks
// https://marabos.nl/atomics/building-channels.html#safety-through-types

pub fn one_shot_channel<T>() -> (Sender<T>, Receiver<T>) {
    let a = Arc::new(Channel {
        message: UnsafeCell::new(MaybeUninit::uninit()),
        ready: AtomicBool::new(false),
    });
    (Sender { channel: a.clone() }, Receiver { channel: a })
}

pub struct Sender<T> {
    channel: Arc<Channel<T>>,
}
pub struct Receiver<T> {
    channel: Arc<Channel<T>>,
}

impl<T> Sender<T> {
    pub fn send(self, message: T) {
        unsafe { (*self.channel.message.get()).write(message) };
        self.channel.ready.store(true, Release);
    }
}

impl<T> Receiver<T> {
    pub fn is_ready(&self) -> bool {
        self.channel.ready.load(Relaxed)
    }

    // Here we give up safety through types:
    // `receive` can't consume `self` because of its use in `Future::try_get`
    pub fn receive(&self) -> T {
        if !self.channel.ready.swap(false, Acquire) {
            panic!("No message available!");
        }
        unsafe { (*self.channel.message.get()).assume_init_read() }
    }
}

struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn send_receive() {
        let (sender, receiver) = one_shot_channel();
        thread::scope(|s| {
            let t = s.spawn(|| {
                while !receiver.is_ready() {
                    thread::park();
                }
                assert_eq!(receiver.receive(), 42);
            });
            sender.send(42);
            t.thread().unpark();
        });
    }
}

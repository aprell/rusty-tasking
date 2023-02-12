use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

pub struct Count(AtomicU32);

impl Count {
    pub fn new(value: u32) -> Self {
        Self(AtomicU32::new(value))
    }

    pub fn get(&self) -> u32 {
        self.0.load(Relaxed)
    }

    pub fn set(&self, value: u32) {
        self.0.store(value, Relaxed);
    }

    // Returns the previous value
    pub fn add(&self, value: u32) -> u32 {
        self.0.fetch_add(value, Relaxed)
    }

    // Returns the previous value
    pub fn sub(&self, value: u32) -> u32 {
        self.0.fetch_sub(value, Relaxed)
    }

    // Returns the previous value
    pub fn inc(&self) -> u32 {
        self.add(1)
    }

    // Returns the previous value
    pub fn dec(&self) -> u32 {
        self.sub(1)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use super::*;

    #[test]
    fn count_up_and_down() {
        let a = Count::new(0);
        for i in 1..=10 {
            a.add(i);
        }
        assert_eq!(a.get(), 55);
        a.sub(45);
        assert_eq!(a.get(), 10);
        for _ in 1..=10 {
            a.dec();
        }
        assert_eq!(a.get(), 0);
        a.dec();
        assert_eq!(a.get(), std::u32::MAX);
        a.inc();
        assert_eq!(a.get(), 0);
    }

    #[test]
    fn count_up_and_down_threads() {
        let a = Arc::new(Count::new(0));
        let b = Arc::clone(&a);
        let c = Arc::clone(&a);
        let d = Arc::clone(&a);

        let ts = vec![
            thread::spawn(move || {
                for i in 1..=10 {
                    b.add(i);
                } // +55
            }),
            thread::spawn(move || {
                for i in 1..=13 {
                    c.sub(i);
                } // -91
            }),
            thread::spawn(move || {
                for i in 1..=20 {
                    d.add(i);
                } // +210
            }),
        ];

        for i in 1..=15 {
            a.sub(i);
        } // -120

        for t in ts {
            t.join().unwrap();
        }

        assert_eq!(a.get(), 54);
    }
}

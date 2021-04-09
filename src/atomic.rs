use std::sync::atomic::{AtomicU32, Ordering};

pub struct Count(AtomicU32);

impl Count {
    pub fn new(value: u32) -> Self {
        Self(AtomicU32::new(value))
    }

    pub fn get(&self, ordering: Ordering) -> u32 {
        self.0.load(ordering)
    }

    pub fn set(&self, value: u32, ordering: Ordering) {
        self.0.store(value, ordering);
    }

    // Returns the previous value
    pub fn add(&self, value: u32, ordering: Ordering) -> u32 {
        self.0.fetch_add(value, ordering)
    }

    // Returns the previous value
    pub fn sub(&self, value: u32, ordering: Ordering) -> u32 {
        self.0.fetch_sub(value, ordering)
    }

    // Returns the previous value
    pub fn inc(&self, ordering: Ordering) -> u32 {
        self.add(1, ordering)
    }

    // Returns the previous value
    pub fn dec(&self, ordering: Ordering) -> u32 {
        self.sub(1, ordering)
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
        let o = Ordering::Relaxed;
        for i in 1..=10 {
            a.add(i, o);
        }
        assert_eq!(a.get(o), 55);
        a.sub(45, o);
        assert_eq!(a.get(o), 10);
        for _ in 1..=10 {
            a.dec(o);
        }
        assert_eq!(a.get(o), 0);
        a.dec(o);
        assert_eq!(a.get(o), std::u32::MAX);
        a.inc(o);
        assert_eq!(a.get(o), 0);
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
                    b.add(i, Ordering::Relaxed);
                } // +55
            }),
            thread::spawn(move || {
                for i in 1..=13 {
                    c.sub(i, Ordering::Relaxed);
                } // -91
            }),
            thread::spawn(move || {
                for i in 1..=20 {
                    d.add(i, Ordering::Relaxed);
                } // +210
            }),
        ];

        for i in 1..=15 {
            a.sub(i, Ordering::Relaxed);
        } // -120

        for t in ts {
            t.join().unwrap();
        }

        assert_eq!(a.get(Ordering::Relaxed), 54);
    }
}

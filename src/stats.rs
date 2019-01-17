use std::cell::Cell;
use std::ops::AddAssign;

#[derive(Debug)]
pub struct Count(Cell<u32>);

impl Count {
    pub fn new() -> Count {
        Count(Cell::new(0))
    }

    pub fn get(&self) -> u32 {
        self.0.get()
    }

    pub fn set(&self, val: u32) {
        self.0.set(val);
    }

    pub fn increment(&self, incr: u32) {
        self.set(self.get() + incr);
    }

    pub fn decrement(&self, decr: u32) {
        self.set(self.get() - decr);
    }
}

#[derive(Debug)]
pub struct Stats {
    pub num_tasks_executed: Count,
}

impl Stats {
    pub fn new() -> Stats {
        Stats { num_tasks_executed: Count::new() }
    }

    pub fn update(&self, other: &Stats) {
        let num_tasks_executed = other.num_tasks_executed.get();
        self.num_tasks_executed.increment(num_tasks_executed);
    }
}

impl AddAssign for Stats {
    fn add_assign(&mut self, other: Stats) {
        self.update(&other);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_up_and_down() {
        let c = Count::new();
        for i in 1..=10 {
            c.increment(i);
        }
        assert_eq!(c.get(), 55);
        c.decrement(10);
        assert_eq!(c.get(), 45);
    }

    #[test]
    fn update_stats() {
        let s = Stats::new();
        s.num_tasks_executed.increment(100);

        let mut t = Stats::new();
        t += s;
        // `s` has been moved
        assert_eq!(t.num_tasks_executed.get(), 100);
    }
}
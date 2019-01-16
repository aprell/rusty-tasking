use std::cell::Cell;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_count() {
        let s = Stats::new();
        for i in 1..=10 {
            s.num_tasks_executed.increment(i);
        }
        assert_eq!(s.num_tasks_executed.get(), 55);
        s.num_tasks_executed.decrement(10);
        assert_eq!(s.num_tasks_executed.get(), 45);
    }

    #[test]
    fn update_stats() {
        let s = Stats::new();
        s.num_tasks_executed.increment(100);

        let t = Stats::new();
        t.update(&s);
        assert_eq!(t.num_tasks_executed.get(), 100);
    }
}

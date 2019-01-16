use std::cell::Cell;

#[derive(Debug)]
pub struct Stats {
    pub num_tasks_executed: Cell<u32>,
}

impl Stats {
    pub fn new() -> Stats {
        Stats { num_tasks_executed: Cell::new(0) }
    }

    pub fn increment(&self, count: &Cell<u32>, incr: u32) {
        count.set(count.get() + incr);
    }

    pub fn decrement(&self, count: &Cell<u32>, decr: u32) {
        count.set(count.get() - decr);
    }

    pub fn update(&self, other: &Stats) {
        let num_tasks_executed = other.num_tasks_executed.get();
        self.increment(&self.num_tasks_executed, num_tasks_executed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_count() {
        let s = Stats::new();
        for i in 1..=10 {
            s.increment(&s.num_tasks_executed, i);
        }
        assert_eq!(s.num_tasks_executed.get(), 55);
        s.decrement(&s.num_tasks_executed, 10);
        assert_eq!(s.num_tasks_executed.get(), 45);
    }

    #[test]
    fn update_stats() {
        let s = Stats::new();
        s.increment(&s.num_tasks_executed, 100);

        let t = Stats::new();
        t.update(&s);
        assert_eq!(t.num_tasks_executed.get(), 100);
    }
}

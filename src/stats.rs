use std::cell::Cell;
use std::ops::AddAssign;

#[derive(Debug)]
pub struct Count(Cell<u32>);

impl Count {
    pub fn new(value: u32) -> Count {
        Count(Cell::new(value))
    }

    pub fn get(&self) -> u32 {
        self.0.get()
    }

    pub fn set(&self, value: u32) {
        self.0.set(value);
    }

    pub fn add(&self, value: u32) {
        self.set(self.get() + value);
    }

    pub fn sub(&self, value: u32) {
        self.set(self.get() - value);
    }

    pub fn inc(&self) {
        self.add(1);
    }

    pub fn dec(&self) {
        self.sub(1);
    }
}

#[derive(Debug)]
pub struct Stats {
    pub num_tasks_executed: Count,
}

impl Stats {
    pub fn new() -> Stats {
        Stats { num_tasks_executed: Count::new(0) }
    }

    pub fn update(&self, other: &Stats) {
        self.num_tasks_executed.add(other.num_tasks_executed.get());
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
        let c = Count::new(0);
        for i in 1..=10 {
            c.add(i);
        }
        assert_eq!(c.get(), 55);
        c.sub(55);
        assert_eq!(c.get(), 0);
    }

    #[test]
    fn update_stats() {
        let s = Stats::new();
        s.num_tasks_executed.add(100);

        let mut t = Stats::new();
        t += s;
        // `s` has been moved
        assert_eq!(t.num_tasks_executed.get(), 100);
    }
}

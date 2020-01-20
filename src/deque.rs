use std::collections::VecDeque;
use std::collections::vec_deque::{Iter, IterMut};

pub trait Steal<T> {
    fn steal(&mut self) -> Option<T>;
}

// Could provide a default, though inefficient, implementation of `steal_many`
// in terms of `steal`
pub trait StealMany<T>: Steal<T> {
    type Loot;

    fn steal_many(&mut self) -> Option<Self::Loot>;
}

// See newtype pattern
pub struct Deque<T>(VecDeque<T>);

impl<T> Deque<T> {
    pub fn new() -> Deque<T> {
        Deque(VecDeque::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn push(&mut self, item: T) {
        self.0.push_front(item);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.0.pop_front()
    }

    pub fn append(&mut self, other: &mut VecDeque<T>) {
        self.0.append(other);
    }

    pub fn iter(&self) -> Iter<T> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        self.0.iter_mut()
    }
}

impl<T> Steal<T> for Deque<T> {
    fn steal(&mut self) -> Option<T> {
        self.0.pop_back()
    }
}

impl<T> StealMany<T> for Deque<T> {
    type Loot = Deque<T>;

    fn steal_many(&mut self) -> Option<Self::Loot> {
        let len = self.0.len();
        if len == 0 { return None; }
        let split_deque = self.0.split_off(len / 2);
        assert!(self.0.len() <= split_deque.len());
        assert!(self.0.len() + split_deque.len() == len);
        Some(Deque(split_deque))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deque() {
        let mut deque = Deque::new();
        assert!(deque.is_empty());

        deque.push(1);
        assert!(!deque.is_empty());

        assert_eq!(deque.pop().unwrap(), 1);
        assert!(deque.is_empty());
    }

    #[test]
    fn deque_pop() {
        let mut deque = Deque::new();

        for i in 0..10 {
            deque.push(i);
        }

        // deque: [9, 8, 7, 6, 5, 4, 3, 2, 1, 0]

        for i in 0..10 {
            // `pop` is LIFO
            assert_eq!(deque.pop().unwrap(), 9-i);
        }

        assert!(deque.is_empty());
    }

    #[test]
    fn deque_steal() {
        let mut deque = Deque::new();

        for i in 0..10 {
            deque.push(i);
        }

        // deque: [9, 8, 7, 6, 5, 4, 3, 2, 1, 0]

        for i in 0..10 {
            // `steal` is FIFO
            assert_eq!(deque.steal().unwrap(), i);
        }

        assert!(deque.is_empty());
    }

    #[test]
    fn deque_steal_many() {
        let mut deque = Deque::new();

        for i in 0..10 {
            deque.push(i);
        }

        // deque: [9, 8, 7, 6, 5, 4, 3, 2, 1, 0]

        let mut loot = deque.steal_many().unwrap();

        // deque: [9, 8, 7, 6, 5]
        // loot:  [4, 3, 2, 1, 0]

        for i in 0..5 {
            assert_eq!(loot.pop().unwrap(), 4-i);
        }

        assert!(loot.is_empty());

        loot = deque.steal_many().unwrap();

        // deque: [9, 8]
        // loot:  [7, 6, 5]

        for i in 0..3 {
            assert_eq!(loot.pop().unwrap(), 7-i);
        }

        assert!(loot.is_empty());

        loot = deque.steal_many().unwrap();

        // deque: [9]
        // loot:  [8]

        assert_eq!(loot.pop().unwrap(), 8);

        assert!(loot.is_empty());

        loot = deque.steal_many().unwrap();

        // deque: [ ]
        // loot:  [9]

        assert_eq!(loot.pop().unwrap(), 9);

        assert!(deque.is_empty());
        assert!(loot.is_empty());
    }
}

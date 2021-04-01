use std::time::{Duration, Instant};

fn fib(n: u64) -> u64 {
    if n < 2 { return n; }
    let mut f = (0, 1);
    for _ in 2..=n { f = (f.1, f.0 + f.1); }
    f.1
}

pub fn compute(duration: Duration) {
    let start = Instant::now();
    while start.elapsed() < duration {
        let _ = fib(10);
    }
    // println!("{:?}", start.elapsed());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_fib() {
        for n in 0..=50 {
            let start = Instant::now();
            let _ = fib(n);
            println!("{:?}", start.elapsed());
        }
    }
}
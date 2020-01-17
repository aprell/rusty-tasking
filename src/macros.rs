#[macro_export]
macro_rules! async_closure {
    // `tt` is a token tree
    ($($body: tt)*) => (Box::new(move || { $($body)* }))
}

#[macro_export]
macro_rules! async_task {
    // `tt` is a token tree
    ($($body: tt)*) => {
        let task = Async::task(async_closure! { $($body)* });
        Worker::current().push(Box::new(task));
        // No return value
    }
}

#[macro_export]
macro_rules! async_future {
    // `tt` is a token tree
    ($($body: tt)*) => {
        {
            let (task, future) = Async::future(async_closure! { $($body)* });
            Worker::current().push(Box::new(task));
            future
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Runtime;
    use crate::task::Async;
    use crate::worker::Worker;

    #[test]
    fn async_tasks() {
        let runtime = Runtime::init(3);
        let master = runtime.master;

        for _ in 0..5 {
            async_task! {
                for _ in 0..5 {
                    async_task! {
                        for _ in 0..3 {
                            async_task!();
                        }
                    }
                }
            }
        }

        let mut num_tasks_executed = 0;
        while master.has_tasks() {
            master.try_handle_steal_request();
            match master.pop() {
                Some(task) => {
                    task.run();
                    num_tasks_executed += 1;
                }
                None => break,
            }
        }

        master.stats.num_tasks_executed.increment(num_tasks_executed);

        // TODO Task barrier needed

        let stats = runtime.join();
        assert_eq!(stats.num_tasks_executed.get(), 105);
    }

    fn sum(n: u32) -> u32 {
        if n <= 1 { n } else { n + async_future!(sum(n - 1)).wait() }
    }

    #[test]
    fn async_futures() {
        let runtime = Runtime::init(3);

        let mut n = async_future!(sum(10));
        assert_eq!(n.wait(), 55);

        let stats = runtime.join();
        assert_eq!(stats.num_tasks_executed.get(), 10);
    }
}

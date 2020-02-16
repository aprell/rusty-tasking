#[macro_export]
macro_rules! async_closure {
    // `tt` is a token tree
    ($($body: tt)*) => (Box::new(move || { $($body)* }))
}

#[macro_export]
macro_rules! spawn {
    // `tt` is a token tree
    ($i: ident, $($body: tt)*) => {
        {
            // $i is supposed to be `channel`
            let (sender, receiver) = $i();
            let task = Async::new(async_closure! { $($body)* }, sender.to_promise());
            Worker::current().push(Box::new(task));
            Future::Chan(receiver)
        }
    };

    ($e: expr, $($body: tt)*) => {
        {
            let task = Async::new(async_closure! { $($body)* }, $e.to_promise());
            Worker::current().push(Box::new(task));
            $e
        }
    };

    ($($body: tt)*) => {
        {
            let task = Async::new(async_closure! { $($body)* }, None);
            Worker::current().push(Box::new(task));
            // No return value
        }
    }
}

#[macro_export]
macro_rules! scoped_spawn {
    // `tt` is a token tree
    ($i: ident, $($body: tt)*) => {
        {
            // $i is supposed to be `channel`
            let (sender, receiver) = $i();
            let task = ScopedAsync::new(async_closure! { $($body)* }, sender.to_promise());
            Worker::current().push(Box::new(task));
            Future::Chan(receiver)
        }
    };

    ($e: expr, $($body: tt)*) => {
        {
            let task = ScopedAsync::new(async_closure! { $($body)* }, $e.to_promise());
            Worker::current().push(Box::new(task));
            $e
        }
    };

    ($($body: tt)*) => {
        {
            let task = ScopedAsync::new(async_closure! { $($body)* }, None);
            Worker::current().push(Box::new(task));
            // No return value
        }
    }
}

#[macro_export]
macro_rules! finish {
    // `tt` is a token tree
    ($($body: tt)*) => {
        {
            Scope::enter();
            $($body)*
            Scope::leave();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::future::{Future, ToPromise};
    use crate::runtime::Runtime;
    use crate::scope::Scope;
    use crate::task::{Async, ScopedAsync};
    use crate::worker::Worker;

    #[test]
    fn async_tasks() {
        let runtime = Runtime::init(3);
        let master = runtime.master;

        for _ in 0..5 {
            spawn! {
                for _ in 0..5 {
                    spawn! {
                        for _ in 0..3 {
                            spawn!();
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

        master.stats.num_tasks_executed.add(num_tasks_executed);

        // TODO Task barrier needed

        let stats = runtime.join();
        assert_eq!(stats.num_tasks_executed.get(), 105);
    }

    #[test]
    fn scoped_async_tasks() {
        let runtime = Runtime::init(3);

        finish! {
            for _ in 0..5 {
                scoped_spawn! {
                    for _ in 0..5 {
                        scoped_spawn! {
                            for _ in 0..3 {
                                scoped_spawn!();
                            }
                        }
                    }
                }
            }
        } // Implicit barrier

        let stats = runtime.join();
        assert_eq!(stats.num_tasks_executed.get(), 105);
    }

    fn sum(n: u32) -> u32 {
        if n <= 1 { n }
        else {
            let mut f = Future::Lazy(None);
            n + spawn!(&mut f, sum(n - 1)).wait()
        }
    }

    #[test]
    fn async_futures() {
        let runtime = Runtime::init(3);

        let mut n = Future::Lazy(None);
        let _ = spawn!(&mut n, sum(10));
        assert_eq!(n.wait(), 55);

        let stats = runtime.join();
        assert_eq!(stats.num_tasks_executed.get(), 10);
    }
}

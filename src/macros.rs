#[macro_export]
macro_rules! async_closure {
    // `tt` is a token tree
    ($($body: tt)*) => (Box::new(move || { $($body)* }))
}

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

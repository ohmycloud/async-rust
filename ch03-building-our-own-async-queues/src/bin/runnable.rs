use std::{future::Future, panic::catch_unwind, thread};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use std::sync::LazyLock;
use async_task::{Runnable, Task};
use futures_lite::future;
use flume::{Sender, Receiver};

#[derive(Debug, Clone, Copy)]
enum FutureType {
    High,
    Low,
}

trait FutureOrderLabel: Future {
    fn get_order(&self) -> FutureType;
}

struct CounterFuture {
    count: u32,
    order: FutureType,
}

impl FutureOrderLabel for CounterFuture {
    fn get_order(&self) -> FutureType {
        self.order
    }
}

impl Future for CounterFuture {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>)
            -> Poll<Self::Output> {
        self.count += 1;
        println!("polling with result: {}", self.count);
        std::thread::sleep(Duration::from_secs(1));
        if self.count < 3 {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(self.count)
        }
    }
}

// Using built-in async functionality automatically handles the polling and scheduling of the task for us.
// Note that our sleep in the `async_fn` is blocking because we want to see how the tasks are processed in our queue.
async fn async_fn() {
    std::thread::sleep(Duration::from_secs(1));
    println!("async fn");
}

struct AsyncSleep {
    start_time: Instant,
    duration: Duration,
}

impl AsyncSleep {
    fn new(duration: Duration) -> Self {
        Self {
            start_time: Instant::now(),
            duration,
        }
    }
}

impl Future for AsyncSleep {
    type Output = bool;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let elapsed_time = self.start_time.elapsed();
        if elapsed_time >= self.duration {
            Poll::Ready(true)
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

// `Send` trait enforces constraints that ensure that our future can be safely shared between threads.
// The static means that out future does not contain any reference that have a shorter lifetime than
// the static lifetime. This means that the future can be used for as long as the program is running.
// As we cannot guarantee when a task is finished, we must ensure that the lifetime of our task is static.
fn spawn_task<F, T>(future: F) -> Task<T>
    where
        F: Future<Output = T> + Send + 'static + FutureOrderLabel,
        T: Send + 'static,
{
    static HIGH_CHANNEL: LazyLock<(Sender<Runnable>, Receiver<Runnable>)> = LazyLock::new(|| {
       flume::unbounded::<Runnable>()
    });

    static LOW_CHANNEL: LazyLock<(Sender<Runnable>, Receiver<Runnable>)> = LazyLock::new(|| {
        flume::unbounded::<Runnable>()
    });

    // With the static we are ensuring our queue is living throughout the lifetime of the program.
    // This make sense as we will want to send tasks to our queue throughout the lifetime of the program.
    // The LazyLock struct gets initialised on the first access of the struct. Once the struct is initialised,
    // it is not initialised again. This is because we will be calling our task spawning function every time we send
    // a future to the async runtime. If we initialise the queue every time we call the spawn_task function, we would
    // be wiping the queue of previous tasks.
    // A Runnable is a handle for a runnable task. Every spawned task has a single Runnable handle,
    // and this handle only exists when the task is scheduled for running. The runnable handle essentially has
    // the run function that polls the task's future once. Then the runnable is dropped. The runnable only appears
    // again when the waker wakes the task in turn scheduling the task again. If we do not pass the waker into our future,
    // it would not be polled again. This is because the future cannot be woken to be polled again.
    static HIGH_QUEUE: LazyLock<flume::Sender<Runnable>> = LazyLock::new(|| {
        for _ in 0..2 {
            let high_receiver = HIGH_CHANNEL.1.clone();
            let low_receiver = LOW_CHANNEL.1.clone();
            thread::spawn(move || {
                loop {
                    match high_receiver.try_recv() {
                        Ok(runnable) => {
                            // We use the `catch_unwind` function because we do not know the quality of the code
                            // being passed to our async runtime. The `catch_unwind` function catches the error if it is thrown whilst
                            //the code is returnning a `Ok` or `Err`.
                            let _ = catch_unwind(|| runnable.run());
                        },
                        Err(_) => {
                            match low_receiver.try_recv() {
                                Ok(runnable) => {
                                    let _ = catch_unwind(|| runnable.run());
                                },
                                Err(_) => {
                                    thread::sleep(Duration::from_millis(100));
                                }
                            }
                        }
                    }
                }
            });
        }

        // We return the transmitter channel so that we can send runnables to our thread.
        HIGH_CHANNEL.0.clone()
    });

    static LOW_QUEUE: LazyLock<flume::Sender<Runnable>> = LazyLock::new(|| {
        for _ in 0..1 {
            let high_receiver = HIGH_CHANNEL.1.clone();
            let low_receiver = LOW_CHANNEL.1.clone();
            thread::spawn(move || {
                loop {
                    match high_receiver.try_recv() {
                        Ok(runnable) => {
                            // We use the `catch_unwind` function because we do not know the quality of the code
                            // being passed to our async runtime. The `catch_unwind` function catches the error if it is thrown whilst
                            //the code is returnning a `Ok` or `Err`.
                            let _ = catch_unwind(|| runnable.run());
                        },
                        Err(_) => {
                            match low_receiver.try_recv() {
                                Ok(runnable) => {
                                    let _ = catch_unwind(|| runnable.run());
                                },
                                Err(_) => {
                                    thread::sleep(Duration::from_millis(100));
                                }
                            }
                        }
                    }
                }
            });
        }
        LOW_CHANNEL.0.clone()
    });

    // We have created a closure that accepts a runnable and sends it to our queue.
    let schedule_high = |runnable| HIGH_QUEUE.send(runnable).unwrap();
    let schedule_low = |runnable| LOW_QUEUE.send(runnable).unwrap();

    let schedule = match future.get_order() {
        FutureType::High => schedule_high,
        FutureType::Low => schedule_low,
    };

    // We then create the runnable and task by using the `async_task` spawn function.
    // `async_task` spawn function leads to an unsafe function that allocates the future onto the heap.
    // The task and runnable returned from the spawn function essentially have a pointer to the same future.
    let (runnable, task) = async_task::spawn(future, schedule);
    // Now that the runnable and task have pointers to the same future, we have to schedule the runnable to be run
    // and return the task. When we schedule the runnable, we essentially put the task on the queue to be processed.
    // If we didn't schedule the runnable, the task would not be run, and our program would be crash when we try to
    // block the main thread to wait on the task being executed because there is no runnable on the queue, but we still
    // return the task. Remember the task and the runnable have pointers to the same future.
    runnable.schedule();
    println!("High queue count: {:?},low queue count: {:?}", HIGH_QUEUE.len(), LOW_QUEUE.len());
    return task;
}

fn main() {
    let high = CounterFuture { count: 0, order: FutureType::High };
    let low = CounterFuture { count: 0, order: FutureType::Low };
    let t_high = spawn_task(high);
    let t_low = spawn_task(low);
    future::block_on(t_high);
    future::block_on(t_low);
}

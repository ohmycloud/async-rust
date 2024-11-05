use std::{
    future::Future,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Duration,
};

use tokio::task::JoinHandle;

#[derive(Debug)]
enum CounterType {
    Increment,
    Decrement,
}

struct SharedData {
    counter: i32,
}

impl SharedData {
    fn increment(&mut self) {
        self.counter += 1;
    }

    fn decrement(&mut self) {
        self.counter -= 1;
    }
}

struct CounterFuture {
    counter_type: CounterType,
    data_refenrece: Arc<Mutex<SharedData>>,
    count: u32,
}

impl Future for CounterFuture {
    type Output = u32;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::thread::sleep(Duration::from_secs(1));
        let mut guard = match self.data_refenrece.try_lock() {
            Ok(guard) => guard,
            Err(error) => {
                println!("error for {:?}: {}", self.counter_type, error);
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };
        let value = &mut *guard;

        match self.counter_type {
            CounterType::Increment => {
                value.increment();
                println!("after increment: {}", value.counter);
            }
            CounterType::Decrement => {
                value.decrement();
                println!("after decrement: {}", value.counter);
            }
        }

        std::mem::drop(guard);
        self.count += 1;
        if self.count < 3 {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        } else {
            return Poll::Ready(self.count);
        }
    }
}

#[tokio::main]
async fn main() {
    let shared_data = Arc::new(Mutex::new(SharedData { counter: 0 }));
    let increment_counter = CounterFuture {
        counter_type: CounterType::Increment,
        data_refenrece: shared_data.clone(),
        count: 0,
    };

    let decrement_counter = CounterFuture {
        counter_type: CounterType::Decrement,
        data_refenrece: shared_data.clone(),
        count: 0,
    };

    let increment_handle: JoinHandle<u32> =
        tokio::task::spawn(async move { increment_counter.await });
    let decrement_counter: JoinHandle<u32> =
        tokio::task::spawn(async move { decrement_counter.await });
    tokio::join!(increment_handle, decrement_counter);
}

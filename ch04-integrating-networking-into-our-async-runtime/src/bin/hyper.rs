use std::future::Future;
use std::panic::catch_unwind;
use std::sync::LazyLock;
use std::thread;
use std::time::Duration;
use async_task::Runnable;
use bytes::Bytes;
use flume::{Receiver, Sender};
use http::Uri;
use hyper::{Request};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use http_body_util::Full;
use smol::{future, Task};

#[derive(Debug, Clone, Copy)]
enum FutureType {
    High,
    Low,
}

trait FutureOrderLabel: Future {
    fn get_order(&self) -> FutureType;
}

fn spawn_task<F, T>(future: F, order: FutureType) -> Task<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
{
    static HIGH_CHANNEL: LazyLock<(Sender<Runnable>, Receiver<Runnable>)> = LazyLock::new(|| {
        flume::unbounded::<Runnable>()
    });

    static LOW_CHANNEL: LazyLock<(Sender<Runnable>, Receiver<Runnable>)> = LazyLock::new(|| {
        flume::unbounded::<Runnable>()
    });

    static HIGH_QUEUE: LazyLock<flume::Sender<Runnable>> = LazyLock::new(|| {
        for _ in 0..2 {
            let high_receiver = HIGH_CHANNEL.1.clone();
            let low_receiver = LOW_CHANNEL.1.clone();
            thread::spawn(move || {
                loop {
                    match high_receiver.try_recv() {
                        Ok(runnable) => {
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

    let schedule_high = |runnable| HIGH_QUEUE.send(runnable).unwrap();
    let schedule_low = |runnable| LOW_QUEUE.send(runnable).unwrap();

    let schedule = match order {
        FutureType::High => schedule_high,
        FutureType::Low => schedule_low,
    };

    let (runnable, task) = async_task::spawn(future, schedule);
    runnable.schedule();
    println!("High queue count: {:?},low queue count: {:?}", HIGH_QUEUE.len(), LOW_QUEUE.len());
    return task;
}

macro_rules! spawn_task {
    ($future:expr) => {
        spawn_task!($future, FutureType::Low)
    };
    ($future:expr, $order:expr) => {
        spawn_task($future, $order)
    };
}

fn main() -> anyhow::Result<()> {
    let url = "http://www.raku.org";
    let uri: Uri = url.parse().unwrap();

    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .header("User-Agent", "hyper/0.14.2")
        .header("Accept", "text/html")
        .body(Full::new(Bytes::from("Rakudo Star")))?;


    println!("{:?}", request);

    let future = async {
        let client = Client::builder(TokioExecutor::new())
            .build_http();
        client.request(request).await.unwrap()
    };

    let test = spawn_task!(future);
    let response = future::block_on(test);
    println!("Response status: {}", response.status());

    Ok(())
}
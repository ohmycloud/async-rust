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

macro_rules! spawn_task {
    ($future:expr) => {
        spawn_task!($future, FutureType::Low)
    };
    ($future:expr, $order:expr) => {
        spawn_task($future, $order)
    };
}

macro_rules! join {
    ($($future:expr),*) => {
        {
            let mut results = Vec::new();
            $(
                results.push(future::block_on($future));
            )*
            results
        }
    };
}

#[derive(Debug, Clone, Copy)]
enum FutureType {
    High,
    Low,
}

trait FutureOrderLabel: Future {
    fn get_order(&self) -> FutureType;
}

#[derive(Clone)]
struct Runtime {
    high_num: usize,
    low_num: usize,
}

impl Runtime {
    pub fn new() -> Self {
        let num_cores = std::thread::available_parallelism().unwrap().get();
        Self {
            high_num: num_cores -2,
            low_num: 1,
        }
    }
    pub fn with_high_num(mut self, num: usize) -> Self {
        self.high_num = num;
        self
    }
    pub fn with_low_num(mut self, num: usize) -> Self {
        self.low_num = num;
        self
    }
    pub fn run(&self) {
        std::env::set_var("HIGH_NUM", self.high_num.to_string());
        std::env::set_var("LOW_NUM", self.low_num.to_string());

        let high = spawn_task!(async {}, FutureType::High);
        let low = spawn_task!(async {}, FutureType::Low);

        join!(high, low);
    }
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

struct CustomExecutor;

impl<F> hyper::rt::Executor<F> for CustomExecutor
where F: Future + Send + 'static,
      F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        spawn_task!(async {
            println!("sending request");
            fut.await;
        }).detach();
    }
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

    let future = async {
        let client = Client::builder(Runtime::new())
            .build_http();
        client.request(request).await.unwrap()
    };

    let test = spawn_task!(future);
    let response = future::block_on(test);
    println!("Response status: {}", response.status());

    Ok(())
}
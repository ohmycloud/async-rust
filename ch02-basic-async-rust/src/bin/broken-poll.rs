use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::{channel, Receiver};
use std::task::{Context, Poll};

struct Demo {
    receiver: Receiver<()>,
}

impl Future for Demo {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        println!("poll");

        // Option 1: Using `while let`
        while let Ok(x) = this.receiver.try_recv() {
            println!("{}", "processing");
            cx.waker().wake_by_ref();
        }
        Poll::Pending
    }
}

#[tokio::main]
async fn main() {
    let (sender, receiver) = channel();
    let demo = Demo { receiver };
    let thread = std::thread::spawn(move || {
        sender.send(()).unwrap();
    });
    println!("before");
    demo.await;
    println!("after");
    thread.join().unwrap();
}

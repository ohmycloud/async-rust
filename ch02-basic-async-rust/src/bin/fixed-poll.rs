use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;
use tokio::sync::mpsc::{channel, Receiver};

struct Demo{
    receiver: Receiver<()>,
}

impl Future for Demo {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        println!("poll");

        if let Poll::Ready(result) = this.receiver.poll_recv(cx) {
            if let Some(_message) = result {
                println!("processed");
            } else {
                println!("channel closed");
            }
            return Poll::Ready(());
        }

        Poll::Pending
    }
}

#[tokio::main]
pub async fn main() {
    let (sender, receiver) = channel(10);
    let demo = Demo{receiver};
    let task = tokio::spawn(async move {
        sender.send(()).await.unwrap();
    });
    println!("before");
    demo.await;
    println!("after");
    task.await.unwrap();
}

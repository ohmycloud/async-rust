use std::{
    thread,
    time::{Duration, Instant},
};
use tokio::time::sleep;

async fn prep_coffee_mug() {
    sleep(Duration::from_millis(100)).await;
    println!("开始倒牛奶...");
    thread::sleep(Duration::from_secs(3));
    println!("牛奶倒好了.");
    println!("准备加入速溶咖啡...");
    thread::sleep(Duration::from_secs(3));
    println!("速溶咖啡加好了.")
}

async fn make_coffee() {
    println!("准备烧开水...");
    sleep(Duration::from_secs(10)).await;
    println!("水烧开了.");
    println!("倒入开水...");
    thread::sleep(Duration::from_secs(3));
    println!("开水倒好了.");
}

async fn make_toast() {
    println!("把面包放入烤面包机");
    sleep(Duration::from_secs(10)).await;
    println!("面包烤好了.");
    println!("给烤面包涂上黄油...");
    thread::sleep(Duration::from_secs(5));
    println!("涂了黄油的烤面包.");
}

#[tokio::main]
async fn main() {
    let start_time = Instant::now();
    let mug_step = prep_coffee_mug();
    let coffee_step = make_coffee();
    let toast_step = make_toast();

    tokio::join!(mug_step, coffee_step, toast_step);
    let elapsed_time = start_time.elapsed();
    println!(
        "It took: {} seconds to make breakfast",
        elapsed_time.as_secs()
    );
}

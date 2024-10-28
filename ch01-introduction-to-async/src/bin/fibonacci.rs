use std::thread;
use std::time::Instant;

fn fibonacci(n: u64) -> u64 {
    if n == 0 || n == 1 {
        return n;
    }
    fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    let start_time = Instant::now();
    let mut threads = Vec::new();

    for i in 0..4 {
        let handle = thread::spawn(move || {
            let result = fibonacci(50);
            println!("Thread {} result: {}", i, result);
        });
        threads.push(handle);
    }

    for handle in threads {
        handle.join().unwrap();
    }

    let elapsed_time = start_time.elapsed();
    println!(
        "4 threads fibonacci(50) took {} ms",
        elapsed_time.as_millis()
    );
}

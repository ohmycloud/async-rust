use std::process::Command;

#[tokio::main]
async fn main() {
    let mut threads = vec![];
    for _ in 0..4 {
        let handle = tokio::spawn(async {
            let output = Command::new("./connection").output();
            match output {
                Ok(output) => {
                    println!(
                        "Process completed with output: {}",
                        String::from_utf8_lossy(&output.stdout)
                    );
                    Ok(output.status.code().unwrap_or(-1))
                }
                Err(e) => {
                    eprint!("Failed to run process: {}", e);
                    Err(e)
                }
            }
        });
        threads.push(handle);
    }

    let mut results = Vec::with_capacity(threads.len());
    for handle in threads {
        results.push(handle.await.unwrap());
    }

    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(exit_code) => println!("Process {} exited with code {}", i + 1, exit_code),
            Err(e) => eprint!("Process {} failed: {}", i + 1, e),
        }
    }
}

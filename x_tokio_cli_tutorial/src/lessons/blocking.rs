// src/lessons/blocking.rs
use tokio::time::Instant;

fn fib(n: u64) -> u64 {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

pub async fn run(fb: u64) -> anyhow::Result<()> {
    let start = Instant::now();

    // CPU-heavy/blocking work must NOT run on a runtime worker threat, or it
    // freezes every other task on that thread.  spawn_blocking moves it to a
    // dedicated pool mean for blocking work.
    let handle = tokio::task::spawn_blocking(move || fib(fb));

    println!("runtime is still free while fib({fb}) computes...");
    let result = handle.await?;

    println!("fib({fb}) = {result} in {:?}", start.elapsed());
    Ok(())
}

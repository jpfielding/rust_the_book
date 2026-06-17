// src/lessons/timers.rs

use std::time::Duration;
use tokio::time::{interval, sleep, timeout};

pub async fn run() -> anyhow::Result<()> {
    println!("sleeping 200ms...");
    sleep(Duration::from_millis(200)).await;

    // interval ticks repeately. The FIRST tick fires immedately.
    let mut ticker = interval(Duration::from_millis(100));
    for n in 1..=3 {
        ticker.tick().await;
        println!("tick {n}");
    }

    // timeout bounds how long a future may run.
    let slow = sleep(Duration::from_secs(10));
    match timeout(Duration::from_millis(150), slow).await {
        Ok(_) => println!("finsihed in time"),
        Err(_) => println!("timed out (as expected)"),
    }
    Ok(())
}

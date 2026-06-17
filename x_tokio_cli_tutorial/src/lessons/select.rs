// src/lessons/select.rs
use std::time::Duration;
use tokio::time::sleep;

pub async fn run() -> anyhow::Result<()> {
    tokio::select! {
        _ = sleep(Duration::from_millis(100)) => println!("fast branch won"),
        _ = sleep(Duration::from_millis(300)) => println!("slow branch won"),
    }
    Ok(())
}

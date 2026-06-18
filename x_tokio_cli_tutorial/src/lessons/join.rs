// src/lessons/join.rs
use std::time::Duration;
use tokio::time::sleep;

async fn fetch(name: &str, ms: u64) -> String {
    sleep(Duration::from_millis(ms)).await;
    println!("fetched: {name} ({ms}ms)");
    format!("{name} ({ms}ms)")
}

pub async fn run() -> anyhow::Result<()> {
    let (a, b, c) = tokio::join!(fetch("a", 150), fetch("b", 100), fetch("c", 200),);
    println!("results: {a}, {b}, {c}");
    Ok(())
}

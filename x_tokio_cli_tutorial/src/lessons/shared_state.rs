// src/lessons/shared_state.rs
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run(tasks: u64) -> anyhow::Result<()> {
    let counter = Arc::new(Mutex::new(0u64));

    let mut handles = Vec::new();
    for _ in 0..tasks {
        let counter = Arc::clone(&counter);
        handles.push(tokio::spawn(async move {
            for _ in 0..1000 {
                let mut n = counter.lock().await;
                *n += 1;
            }
        }))
    }

    for handle in handles {
        let _ = handle.await?;
    }

    println!(
        "final = {} (expected = {})",
        *counter.lock().await,
        tasks * 1000
    );

    Ok(())
}

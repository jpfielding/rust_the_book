// src/lessons/cancel.rs
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

pub async fn run() -> anyhow::Result<()> {
    let token = CancellationToken::new();
    let child = token.clone();

    let worker = tokio::spawn(async move {
        let mut n = 0;
        loop {
            tokio::select! {
                _ = child.cancelled() => {
                    println!("worker: cancellation received, cleaning up");
                    break;
                }
                _ = sleep(Duration::from_millis(50)) => {
                    n += 1;
                    println!("worker: step {n}");
                }
            }
        }
    });

    sleep(Duration::from_millis(200)).await;
    println!("main: requesting cancellation");
    token.cancel();
    worker.await?;
    Ok(())
}

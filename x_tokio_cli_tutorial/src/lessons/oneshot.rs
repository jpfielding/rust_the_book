// src/lessons/oneshot.rs
use tokio::sync::oneshot;

pub async fn run() -> anyhow::Result<()> {
    let (tx, rx) = oneshot::channel::<u64>();

    tokio::spawn(async move {
        let _ = tx.send(42); // send consumes tx; can be called only once
    });

    let answer = rx.await?;
    println!("workder replied : {answer}");
    Ok(())
}

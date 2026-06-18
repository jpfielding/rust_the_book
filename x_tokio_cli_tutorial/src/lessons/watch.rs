// src/lessons/watch.rs
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::sleep;

pub async fn run() -> anyhow::Result<()> {
    let (tx, mut rx) = watch::channel("starting");

    let consuerm = tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            // borrow() yields a read guard; * deferences to the value
            println!("state -> {}", *rx.borrow());
        }
    });

    for state in ["running", "draining", "stopped"] {
        sleep(Duration::from_millis(50)).await;
        tx.send(state).unwrap();
    }
    drop(tx); // closes the channel so consumer's change() returns Err and it exits
    let _ = consuerm.await; // wait for consumer to finish
    Ok(())
}

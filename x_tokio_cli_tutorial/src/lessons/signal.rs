// src/lessons/signal.rs
use std::time::Duration;
use tokio::time::sleep;

pub async fn run() -> anyhow::Result<()> {
    let worker = tokio::spawn(async {
        let mut n = 0;
        loop {
            sleep(Duration::from_millis(300)).await;
            n += 1;
            println!("working... {n}");
        }
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => println!("got Ctrl-C, shutting down..."),
        _ = worker => {}
    }
    println!("shutting down gracefully...");
    Ok(())
}

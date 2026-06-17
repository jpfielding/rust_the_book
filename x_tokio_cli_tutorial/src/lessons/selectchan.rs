use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

pub async fn run() -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<String>(1);
    let shutdown = CancellationToken::new();

    let child = shutdown.clone();
    let worker = tokio::spawn(async move {
        loop {
            tokio::select! {
                maybe = rx.recv() => match maybe { // use maybe to handle the channel closing
                    Some(cmd) => println!("worker: handling '{cmd}'"),
                    None => {
                        println!("worker: channel closed, exiting");
                        break; // the loop
                    }
                },
                _ = child.cancelled() => {
                    println!("worker: shutdown requested, draining remaining...");
                    // try_recv() is non-blocking: empty the buffer instead of
                    // dropping in-flight work. The recv branch was canceled,
                    // but because recv() is cancel-safe, these are all still here
                    while let Ok(cmd) = rx.try_recv() {
                        println!("worker: drained '{cmd}'");
                    }
                }
            }
        }
    });

    // Feed it some commands, then shut it down
    for cmd in ["build", "test", "deploy"] {
        tx.send(cmd.to_string()).await?;
        sleep(Duration::from_millis(100)).await;
    }
    println!("main: requesting shutdown");
    shutdown.cancel();

    drop(tx); // close the channel, so the worker can exit after draining
    let _ = worker.await; // wait for the worker to finish
    Ok(())
}

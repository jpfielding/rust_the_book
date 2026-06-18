// src/lessons/broadcast.rs
use tokio::sync::broadcast;

pub async fn run() -> anyhow::Result<()> {
    let (tx, _) = broadcast::channel::<u64>(16);

    let mut handles = Vec::new();
    for id in 0..3 {
        let mut rx = tx.subscribe(); // subscribe BEFORE sending
        handles.push(tokio::spawn(async move {
            while let Ok(v) = rx.recv().await {
                println!("subscriber {id} recieved {v}");
            }
        }))
    }

    for v in 0..5 {
        tx.send(v).unwrap();
    }
    drop(tx); // closes the channel so subscribers' recv() returns Err and they exit

    for h in handles {
        let _ = h.await;
    }
    Ok(())
}

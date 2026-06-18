// src/lessons/mpsc.rs
use tokio::sync::mpsc;

pub async fn run(producers: u64) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<String>(32);

    for p in 0..producers {
        let tx = tx.clone(); // shadow the original tx with a clone for each producer
        tokio::spawn(async move {
            for i in 0..10 {
                let msg = format!("producer {p} msg {i}");
                tx.send(msg).await.unwrap();
            }
        });
    }

    drop(tx); // drop OUR sender; rx closes once all senders are gone

    while let Some(msg) = rx.recv().await {
        println!("got: {msg}");
    }
    println!("channel closed");
    Ok(())
}

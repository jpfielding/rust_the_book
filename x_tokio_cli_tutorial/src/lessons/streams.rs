// src/lesson/streams.rs
use tokio_stream::StreamExt;

pub async fn run() -> anyhow::Result<()> {
    let mut stream = tokio_stream::iter(0..=5)
        .map(|n| n * n)
        .filter(|n| n % 2 == 1);

    while let Some(v) = stream.next().await {
        println!("stream yielded {v}");
    }

    Ok(())
}

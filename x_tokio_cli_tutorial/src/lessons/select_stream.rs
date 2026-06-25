// src/lessons/select_stream.rs
use tokio_stream::{self as stream, StreamExt};

pub async fn run() -> anyhow::Result<()> {
    let mut s1 = stream::iter(vec![1, 2, 3]);
    let mut s2 = stream::iter(vec![4, 5, 6]);
    let mut s3 = stream::iter(vec![7, 8, 9]);

    // Use select! to await the next value from either stream
    let next = tokio::select! {
        v = s1.next() => v.unwrap(),
        v = s2.next() => v.unwrap(),
        v = s3.next() => v.unwrap(),
    };
    println!("next: {next}");

    // Use select! to await the next value from any of the three streams
    let mut values = vec![];
    loop {
        tokio::select! {
            biased; // prioritize the first branch
            Some(v) = s1.next() => values.push(v),
            Some(v) = s2.next() => values.push(v),
            Some(v) = s3.next() => values.push(v),
            else => break,
        };
    }

    values.sort();
    println!("values: {:?}", values);
    Ok(())
}

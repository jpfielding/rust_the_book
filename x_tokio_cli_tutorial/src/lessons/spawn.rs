// src/lessons/spawn.rs
use std::time::Duration;
use tokio::time::sleep;

pub async fn run(tasks: u64) -> anyhow::Result<()> {
    let mut handles = Vec::new();

    for id in 0..tasks {
        let handle = tokio::spawn(async move {
            // Finish in reverse order so you can see them interleave
            sleep(Duration::from_millis(100 * (tasks - id))).await;
            println!("task {id} done");
            id * id
        });
        handles.push(handle);
    }

    let mut sum = 0;
    for handle in handles {
        sum += handle.await?;
    }
    println!("sum of squares: {sum}");
    Ok(())
}

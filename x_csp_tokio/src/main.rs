//! CSP-style concurrency in Rust, on Tokio.
//!
//! CSP (Communicating Sequential Processes, Hoare 1978) says: don't coordinate
//! concurrent work by locking shared memory — instead, give each unit of work
//! its own private state and let them talk over **channels**. Or, as the Go
//! proverb puts it: *"Do not communicate by sharing memory; instead, share
//! memory by communicating."*
//!
//! In this crate a "sequential process" is a `tokio::spawn`ed task. The only
//! wiring between tasks is a channel. Ownership of a value *moves* through the
//! channel, so the compiler enforces that at most one task touches it at a time
//! — no `Mutex` around the data itself required.
//!
//! Run it:  `cargo run -p x_csp_tokio`
//!
//! Four self-contained demos, each a different CSP shape:
//!   1. pipeline        — source → transform → sink
//!   2. fan_out_fan_in  — one queue, N workers, one collector
//!   3. request_reply   — the actor pattern via a `oneshot` reply channel
//!   4. select_shutdown — race work against a cancellation signal

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, mpsc, oneshot, watch};
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    println!("== 1. pipeline ==");
    pipeline().await;

    println!("\n== 2. fan-out / fan-in ==");
    fan_out_fan_in().await;

    println!("\n== 3. request / reply (actor) ==");
    request_reply().await;

    println!("\n== 4. select + graceful shutdown ==");
    select_shutdown().await;
}

// ---------------------------------------------------------------------------
// 1. PIPELINE
//
// Three sequential processes chained by two channels. Each stage owns its
// loop, reads from its inbound channel, and writes to the next. When a stage's
// sender is dropped, the downstream `recv()` returns `None` and that stage
// shuts itself down — shutdown propagates for free, left to right.
// ---------------------------------------------------------------------------
async fn pipeline() {
    // Stage boundaries. Bounded channels give us backpressure: if a consumer
    // falls behind, `send().await` parks the producer instead of growing an
    // unbounded queue.
    let (nums_tx, mut nums_rx) = mpsc::channel::<u64>(8);
    let (sq_tx, mut sq_rx) = mpsc::channel::<u64>(8);

    // Source: emit 1..=5, then drop nums_tx by letting it fall out of scope.
    tokio::spawn(async move {
        for n in 1..=5 {
            if nums_tx.send(n).await.is_err() {
                break; // consumer went away
            }
        }
    });

    // Transform: square each number and forward it.
    tokio::spawn(async move {
        while let Some(n) = nums_rx.recv().await {
            let _ = sq_tx.send(n * n).await;
        }
        // sq_tx dropped here -> the sink's recv() will end.
    });

    // Sink: run on this task so main awaits completion.
    while let Some(sq) = sq_rx.recv().await {
        println!("  squared -> {sq}");
    }
}

// ---------------------------------------------------------------------------
// 2. FAN-OUT / FAN-IN
//
// One producer feeds a work queue. N worker processes pull from the *same*
// receiver (a single-consumer channel shared behind a Mutex — the lock guards
// the *queue handle*, not the work data). Each worker sends its result into a
// shared results channel (fan-in). This is how you parallelize CPU/IO work
// while keeping every worker's state private.
// ---------------------------------------------------------------------------
async fn fan_out_fan_in() {
    const WORKERS: usize = 3;

    let (jobs_tx, jobs_rx) = mpsc::channel::<u64>(32);
    let (results_tx, mut results_rx) = mpsc::channel::<(usize, u64)>(32);

    // mpsc::Receiver is single-consumer, so to share it across workers we wrap
    // it once and hand each worker an Arc clone. Whoever wins the lock takes
    // the next job. (An alternative is one channel per worker + a dispatcher.)
    let jobs_rx = Arc::new(Mutex::new(jobs_rx));

    for id in 0..WORKERS {
        let jobs_rx = Arc::clone(&jobs_rx);
        let results_tx = results_tx.clone();
        tokio::spawn(async move {
            loop {
                // Hold the lock only long enough to grab one job.
                let job = {
                    let mut rx = jobs_rx.lock().await;
                    rx.recv().await
                };
                let Some(n) = job else { break }; // queue drained & closed
                sleep(Duration::from_millis(10 * n)).await; // pretend work
                let _ = results_tx.send((id, n * 2)).await;
            }
        });
    }
    // Drop our extra sender so results_rx ends once all workers finish.
    drop(results_tx);

    // Producer.
    tokio::spawn(async move {
        for n in 1..=9 {
            let _ = jobs_tx.send(n).await;
        }
        // jobs_tx dropped -> workers see the queue close and exit their loops.
    });

    // Collector (fan-in).
    let mut total = 0;
    while let Some((worker, out)) = results_rx.recv().await {
        println!("  worker {worker} produced {out}");
        total += out;
    }
    println!("  fan-in total = {total}");
}

// ---------------------------------------------------------------------------
// 3. REQUEST / REPLY  (the actor pattern)
//
// A single "process" owns some state and is the *only* thing that touches it.
// Callers never lock the state — they send a command message and include a
// `oneshot` sender for the reply. This turns shared mutable state into a
// sequential process you talk to. No Mutex on the counter itself.
// ---------------------------------------------------------------------------
enum Cmd {
    Incr,
    Get(oneshot::Sender<u64>),
}

async fn request_reply() {
    let (tx, mut rx) = mpsc::channel::<Cmd>(16);

    // The actor: sole owner of `count`.
    let actor = tokio::spawn(async move {
        let mut count: u64 = 0;
        while let Some(cmd) = rx.recv().await {
            match cmd {
                Cmd::Incr => count += 1,
                Cmd::Get(reply) => {
                    // Ignore error: caller may have dropped the receiver.
                    let _ = reply.send(count);
                }
            }
        }
    });

    // Clients just message the actor.
    for _ in 0..5 {
        tx.send(Cmd::Incr).await.unwrap();
    }
    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(Cmd::Get(reply_tx)).await.unwrap();
    let value = reply_rx.await.unwrap();
    println!("  actor count = {value}");

    drop(tx); // close the mailbox so the actor loop ends
    actor.await.unwrap();
}

// ---------------------------------------------------------------------------
// 4. SELECT + GRACEFUL SHUTDOWN
//
// A worker that must react to more than one channel uses `tokio::select!` to
// wait on all of them at once and act on whichever fires first. Here it races
// real work against a `watch` shutdown signal, so a cancel is honored promptly
// instead of after the current batch.
// ---------------------------------------------------------------------------
async fn select_shutdown() {
    let (work_tx, mut work_rx) = mpsc::channel::<u64>(8);
    // watch = single value broadcast to many observers; perfect for a flag.
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    let worker = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Bias not required, but shows both arms are live at once.
                maybe = work_rx.recv() => match maybe {
                    Some(n) => println!("  handled {n}"),
                    None => break, // work channel closed
                },
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        println!("  shutdown signal -> draining and exiting");
                        break;
                    }
                }
            }
        }
    });

    // Feed a little work, then signal shutdown mid-stream.
    for n in 1..=3 {
        work_tx.send(n).await.unwrap();
    }
    sleep(Duration::from_millis(20)).await;
    shutdown_tx.send(true).unwrap();

    worker.await.unwrap();
}

# CSP-Style Concurrency in Rust (on Tokio)

A walkthrough of building concurrent programs the **CSP** way: instead of
sharing state behind locks, you give each concurrent process its own private
state and let processes **talk over channels**.

> **CSP** = *Communicating Sequential Processes* (Tony Hoare, 1978). The slogan,
> borrowed from Go:
> **"Do not communicate by sharing memory; instead, share memory by
> communicating."**

This tutorial is a companion to the runnable code in `src/main.rs`. Everything
here compiles and runs:

```bash
cargo run -p x_csp_tokio
```

---

## 0. What "CSP" actually buys you

The usual concurrency bug is two threads touching the same memory at the same
time. The usual fix is a `Mutex` — but locks are easy to forget, easy to hold
too long, and easy to deadlock.

CSP sidesteps the whole category. The rule:

- Each **process** (here: a `tokio::spawn`ed task) owns its own data.
- Processes never reach into each other's memory.
- The *only* interaction is **sending a value down a channel.**

Rust makes this especially strong because a value **moves** through a channel.
Once you `send(x)`, `x` is gone from the sender — the compiler guarantees the
sender can't keep poking at it. Ownership *is* the mutual exclusion. No lock on
the data required.

A "sequential process" is sequential *inside* — it does one thing at a time, so
its own logic is simple, single-threaded reasoning. Concurrency lives only in
the wiring *between* processes.

---

## 1. Picking a channel

Tokio ships four channels. Choosing the right one is 80% of CSP design:

| Channel                | Shape                        | Use it for |
|------------------------|------------------------------|------------|
| `mpsc`                 | many senders → one receiver  | work queues, pipelines, an actor's mailbox |
| `oneshot`              | one sender → one receiver, **one** value | a single reply to a request |
| `broadcast`            | many senders → many receivers, every receiver sees every value | fan-out events |
| `watch`                | one sender → many receivers, only the **latest** value | config/shutdown flags |

We use `mpsc`, `oneshot`, and `watch` in this crate. Two `mpsc` notes that
matter for CSP:

- **`Sender` is cloneable; `Receiver` is not.** "Multi-producer, single
  consumer" is literal. If you need multiple consumers, either share the one
  receiver behind a `Mutex` (§3) or give each consumer its own channel.
- **Prefer bounded (`mpsc::channel(N)`) over unbounded.** A bound gives you
  **backpressure**: when the consumer lags, `send().await` *parks the producer*
  rather than letting an in-memory queue grow without limit. Backpressure is a
  feature, not a nuisance — it's how a pipeline self-regulates.

> Reach for `tokio::sync::mpsc`, **not** `std::sync::mpsc`, inside async code.
> The std channel's `recv()` blocks the OS thread, which stalls a whole Tokio
> worker. The Tokio version's `recv().await` *suspends the task* and yields the
> thread to other work.

---

## 2. Pattern: pipeline (source → transform → sink)

The simplest CSP shape: a chain of stages, each its own task, connected by
channels. Each stage loops over its inbound channel and writes to the next.

```rust
let (nums_tx, mut nums_rx) = mpsc::channel::<u64>(8);
let (sq_tx,   mut sq_rx)   = mpsc::channel::<u64>(8);

// Source
tokio::spawn(async move {
    for n in 1..=5 {
        if nums_tx.send(n).await.is_err() { break; }
    }
}); // nums_tx dropped here

// Transform
tokio::spawn(async move {
    while let Some(n) = nums_rx.recv().await {
        let _ = sq_tx.send(n * n).await;
    }
}); // sq_tx dropped here

// Sink (on the current task)
while let Some(sq) = sq_rx.recv().await {
    println!("squared -> {sq}");
}
```

**The shutdown trick.** When a stage's `Sender` is dropped (falls out of
scope), the downstream `recv()` returns `None`, its `while let` ends, and *that*
stage drops *its* sender in turn. Close the source and the whole pipeline
drains and tears down, left to right, with no shutdown flag anywhere. Let the
type system end the program for you.

---

## 3. Pattern: fan-out / fan-in

To parallelize, put N worker processes on one queue (**fan-out**) and funnel
their results back into one channel (**fan-in**).

```rust
let (jobs_tx,    jobs_rx)        = mpsc::channel::<u64>(32);
let (results_tx, mut results_rx) = mpsc::channel::<(usize, u64)>(32);

// One receiver, shared by all workers. The Mutex guards the *queue handle*,
// not the work — each worker still owns its job privately once it has it.
let jobs_rx = Arc::new(Mutex::new(jobs_rx));

for id in 0..WORKERS {
    let jobs_rx    = Arc::clone(&jobs_rx);
    let results_tx = results_tx.clone();
    tokio::spawn(async move {
        loop {
            let job = { jobs_rx.lock().await.recv().await }; // lock held briefly
            let Some(n) = job else { break };
            let _ = results_tx.send((id, n * 2)).await;
        }
    });
}
drop(results_tx); // so results_rx ends once every worker finishes
```

Two things to internalize:

- **The `Mutex` here is not a retreat from CSP.** It guards only the *shared
  receiver handle* — "who gets the next job." The job itself moves to exactly
  one worker and is never shared. Contrast with lock-around-the-data, where
  every worker contends on the payload.
- **Sender-drop is your "all done" signal, twice.** Dropping `jobs_tx` closes
  the queue so workers exit; dropping every `results_tx` (including the extra
  one via `drop(results_tx)`) closes the results channel so the collector's
  loop ends. Forget that `drop` and the collector hangs forever waiting on a
  sender that still exists.

---

## 4. Pattern: request / reply (the actor)

This is how CSP replaces `Arc<Mutex<State>>`. One process **owns** the state and
is the only code that touches it. Callers don't lock — they mail a command, and
include a `oneshot` sender for the answer.

```rust
enum Cmd {
    Incr,
    Get(oneshot::Sender<u64>),   // <- caller ships a reply address
}

// The actor: sole owner of `count`.
tokio::spawn(async move {
    let mut count: u64 = 0;
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Cmd::Incr        => count += 1,
            Cmd::Get(reply)  => { let _ = reply.send(count); }
        }
    }
});

// A client:
tx.send(Cmd::Incr).await.unwrap();

let (reply_tx, reply_rx) = oneshot::channel();
tx.send(Cmd::Get(reply_tx)).await.unwrap();
let value = reply_rx.await.unwrap();   // await the reply
```

Because the actor handles commands **one at a time**, its logic is ordinary
single-threaded code — no locking, no data races possible, even under a
thousand concurrent clients. The `mpsc` mailbox serializes access for you. This
"turn shared state into a process you message" move is the beating heart of
CSP, and of actor systems (Erlang, Akka) built on top of it.

`oneshot` is the right reply channel: exactly one value, exactly one receiver,
and it's cheap. `reply.send()` returns a `Result` because the caller might have
given up and dropped `reply_rx` — ignore it or log it, never `unwrap()` it in
production.

---

## 5. Pattern: `select!` + graceful shutdown

A process often must watch **more than one** channel. `tokio::select!` waits on
several futures at once and runs the branch of whichever is ready first. The
classic use is racing real work against a cancellation signal.

```rust
let (work_tx,     mut work_rx)     = mpsc::channel::<u64>(8);
let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

tokio::spawn(async move {
    loop {
        tokio::select! {
            maybe = work_rx.recv() => match maybe {
                Some(n) => println!("handled {n}"),
                None    => break,                 // work channel closed
            },
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() { break } // cancel honored promptly
            }
        }
    }
});
```

`watch` is ideal for a shutdown flag: one writer flips it, every observer sees
the latest value, and `.changed().await` wakes them. Because both arms are live
each iteration, a shutdown is acted on *between* work items instead of after
the whole queue drains.

> **`select!` gotcha — cancellation safety.** When one branch fires, the other
> branch's future is **dropped mid-flight**. That's fine for `recv()` (safe to
> re-create next loop) but *not* for operations that lose data when dropped
> partway. When unsure, check the method's docs for "cancel safe," or move the
> risky work into a branch that runs to completion before looping.

---

## 6. Design checklist

When you sketch a concurrent feature the CSP way, ask:

1. **What are the processes?** Draw a box per independent unit of work.
2. **What flows between them?** Each arrow is a channel; pick its type from §1.
3. **Who owns each piece of state?** Exactly one process. If two want it, one
   of them should instead be an actor the other messages (§4).
4. **How does it end?** Trace sender-drop from the source outward — closing the
   upstream should cascade shutdown downstream (§2, §3). Add an explicit
   `watch`/`select!` signal only when you must interrupt mid-work (§5).
5. **Is anything unbounded?** Every `mpsc::channel(N)` should have a real `N`
   so backpressure protects you.

If you can answer these, you've replaced "where do I put the locks?" with
"where do I put the channels?" — and that's the whole point of CSP.

---

## Where this fits in the repo

- `x_axum_tokio_app_chans` — a channel shared between two HTTP endpoints
  (`POST /ping` → `GET /pong`). That's this actor/mailbox idea (§4) wired to a
  web server.
- `x_threads_ownership_tokio`, `x_tokio_canc` — ownership-across-tasks and
  cancellation, the raw materials §2 and §5 build on.

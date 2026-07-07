# Sharing a Channel Between Two HTTP Endpoints (ping → pong)

A walkthrough of wiring a **channel** through Axum so that one endpoint
*produces* messages and another *consumes* them:

- `POST /ping` — accepts a JSON body, deserializes it into a struct, and
  **sends** it into a channel.
- `GET /pong` — **long-polls** (awaits) on the channel until a message is
  available, then serializes it back out as JSON.

The channel is the shared "pipe." The POST side never blocks; the GET side
parks until something shows up.

> This tutorial is a companion to the code in `src/main.rs`. It does **not**
> modify your existing file — it explains the concepts and shows the finished
> shape you'd build toward.

---

## 1. Pick the right channel

Your current `main.rs` imports `std::sync::mpsc`. That's a **blocking**
channel: its `recv()` blocks the OS thread. Inside an async handler, blocking a
thread stalls the whole Tokio worker — exactly what we must avoid for a
long-poll.

Use Tokio's async channel instead:

```rust
use tokio::sync::mpsc;
```

`tokio::sync::mpsc` is **m**ulti-**p**roducer, **s**ingle-**c**onsumer:

- The `Sender` is cheap to clone — every `POST /ping` invocation can hold one.
- The `Receiver` is **not** cloneable — there is exactly one consumer end.
  Since Axum may run many `GET /pong` requests concurrently, we guard the single
  receiver behind a `Mutex` so only one poller pulls a given message.

`recv().await` is the key: it suspends the task (yielding the thread to other
work) and wakes up the instant a message is sent. That *is* our long-poll — the
HTTP request simply stays open until `recv()` resolves.

---

## 2. Add `serde` for JSON

Axum's `Json` extractor/response needs `serde`. Your `Cargo.toml` doesn't list
it yet, so add:

```toml
serde = { version = "1", features = ["derive"] }
```

(`axum` already pulls in `serde_json` internally for the `Json` type.)

---

## 3. Define the message type

One struct serves both directions — it derives `Deserialize` (to parse the
incoming ping) and `Serialize` (to emit the outgoing pong):

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Message {
    text: String,
}
```

Incoming JSON like `{"text":"hello"}` deserializes into `Message`; the same
struct serializes straight back out on the pong side.

---

## 4. Hold both ends in shared state

Axum shares data with handlers via `State`. We put the cloneable `Sender` and
the `Mutex`-wrapped `Receiver` in one struct. `Arc` lets every handler clone a
cheap handle to the same underlying state.

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    tx: mpsc::Sender<Message>,
    rx: Arc<Mutex<mpsc::Receiver<Message>>>,
}
```

Why the two wrappers differ:

| End      | Wrapper                 | Reason                                             |
|----------|-------------------------|----------------------------------------------------|
| `tx`     | bare `Sender` (cloneable) | many producers allowed; clone is cheap           |
| `rx`     | `Arc<Mutex<Receiver>>`  | only one consumer exists; `Mutex` serializes pollers |

> Use `tokio::sync::Mutex` (not `std::sync::Mutex`) because we hold the lock
> across an `.await` (`recv().await`). A std `Mutex` guard isn't `Send` across
> await points and would block the runtime.

---

## 5. Create the channel and build state

In `run()`, replace the current `std::sync::mpsc::channel()` line with the
Tokio channel, then bundle both ends into `AppState`. The argument to
`channel()` is the buffer capacity — how many messages can sit in the pipe
before `send().await` applies backpressure.

```rust
let (tx, rx) = mpsc::channel::<Message>(32);

let state = AppState {
    tx,
    rx: Arc::new(Mutex::new(rx)),
};
```

---

## 6. The POST /ping handler — produce

Axum's `Json<Message>` extractor deserializes the request body for us. We then
`send` it into the channel. `send().await` only waits if the buffer is full
(backpressure); normally it returns immediately.

We use `tokio::select!` to **race** the send against a timeout. If the buffer
stays full longer than the window — a slow/absent consumer — we bail out with
`408` instead of hanging the request.

```rust
use axum::extract::State;
use axum::Json;
use axum::http::StatusCode;
use std::time::Duration;
use tokio::time::sleep;

async fn ping(
    State(state): State<AppState>,
    Json(msg): Json<Message>,
) -> StatusCode {
    tokio::select! {
        // `select!` can pattern-match a branch's output. `Ok(())` binds only on
        // a successful send; the branch is skipped otherwise.
        Ok(()) = state.tx.send(msg) => StatusCode::ACCEPTED,     // 202: queued
        _ = sleep(Duration::from_secs(5)) => StatusCode::REQUEST_TIMEOUT, // 408
    }
}
```

**Read the semantics carefully.** With the pattern-binding branch
(`Ok(()) = ...`), a *failed* send — every `Receiver` dropped, so `send` returns
`Err` — does **not** match `Ok(())`. That branch is disabled and `select!`
falls through to the timeout, so a dead channel returns `408` after 5s rather
than `503` immediately. That's the tradeoff of filtering by pattern.

If you want to distinguish "queue full" from "no consumers," keep a `match`
*inside* the branch — this is the case where `match` is still the honest tool:

```rust
tokio::select! {
    result = state.tx.send(msg) => match result {
        Ok(_) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,  // 503: receiver gone, now
    },
    _ = sleep(Duration::from_secs(5)) => StatusCode::REQUEST_TIMEOUT,
}
```

The lesson: `select!` chooses *which* future wins the race; `match` decides what
that future's result means. They compose — one doesn't replace the other.

---

## 7. The GET /pong handler — long-poll and consume

This is where the "long poll" lives. We lock the receiver, then race
`recv().await` against a timeout with `tokio::select!`. The task suspends until
a `ping` arrives (or the window elapses); the moment a message lands, we get the
`Message` and return it as JSON.

```rust
async fn pong(
    State(state): State<AppState>,
) -> Result<Json<Message>, StatusCode> {
    let mut rx = state.rx.lock().await;
    tokio::select! {
        // Binds only when a message actually arrives.
        Some(msg) = rx.recv() => Ok(Json(msg)),               // serialized to JSON body
        _ = sleep(Duration::from_secs(30)) => Err(StatusCode::REQUEST_TIMEOUT), // 408
    }
}
```

Why `select!` fits here better than at `/ping`: a long-poll that can wait
*forever* is a liability — clients time out, connections pile up. Racing against
a `sleep` gives the wait a **bounded** ceiling and returns `408` so the client
knows to poll again.

Things worth understanding:

- **The request parks** until a message exists *or* the timeout fires. While
  parked on `recv()`, the connection stays alive but no CPU spins and no thread
  is blocked — that's the point of an async long-poll.
- **`recv()` returning `None`** (all senders dropped) does **not** match
  `Some(msg)`, so that branch is disabled and `select!` falls to the timeout.
  As with `/ping`, use a `match` inside the branch if you need to surface a
  distinct `503` for a closed channel instead of a delayed `408`.
- **Biased vs. random.** By default `select!` picks a *random* ready branch to
  avoid starvation. If a message and the timeout somehow become ready in the
  same poll, order isn't guaranteed. Prefix the macro with `biased;` to check
  branches top-to-bottom (message first) if you want deterministic priority.

> ⚠️ **Lock-held-across-await caveat.** Because we hold `rx.lock()` for the
> whole duration of `recv().await`, a *second* `GET /pong` will wait for the
> lock before it even starts polling. That serializes pollers — fine for a
> single-consumer demo. If you want several waiters to race for the next
> message, restructure so each waiter owns its own receiver (e.g. a broadcast
> channel), rather than sharing one behind a mutex.

---

## 8. Register the routes with state

Attach both routes and pass the state into the router with `.with_state(...)`.
Handlers that declare `State<AppState>` receive it automatically.

```rust
fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/ping", post(ping))
        .route("/pong", get(pong))
        .with_state(state)
}
```

Remember the extra import at the top:

```rust
use axum::routing::{get, post};
```

And update the `axum::serve(listener, router())` call to `router(state)`.

---

## 9. Try it

Run the server (uses your existing clap `--port`, default `8080`):

```bash
cargo run
```

In one terminal, start a long-poll on `/pong`. It will **hang** — nothing has
been sent yet:

```bash
curl http://localhost:8080/pong
```

In a second terminal, POST a ping:

```bash
curl -X POST http://localhost:8080/ping \
  -H 'content-type: application/json' \
  -d '{"text":"hello"}'
```

The instant the POST lands, the hanging `/pong` returns:

```json
{"text":"hello"}
```

That round trip — POST parks a message in the channel, GET wakes up and drains
it — is the whole idea.

---

## Mental model

```
   POST /ping                 channel (mpsc)              GET /pong
  ┌───────────┐   send().await   ┌──────────┐   recv().await   ┌───────────┐
  │ Json<Msg> │ ───────────────▶ │  buffer  │ ───────────────▶ │ Json<Msg> │
  └───────────┘                  └──────────┘   (parks until    └───────────┘
   deserialize                    Sender: clone   a message)      serialize
                                  Receiver: Arc<Mutex<..>>
```

- **Channel** = the shared pipe living in `AppState`.
- **`tx`** is cloned per request → multiple producers.
- **`rx`** is a single consumer behind `Arc<Mutex<..>>`.
- **`recv().await`** is the long-poll: cheap, non-blocking suspension until data
  arrives.

---

## Summary of changes vs. current `main.rs`

1. Swap `std::sync::mpsc` → `tokio::sync::mpsc`.
2. Add `serde` to `Cargo.toml`; define a `Serialize + Deserialize` message struct.
3. Introduce an `AppState { tx, rx: Arc<Mutex<Receiver>> }`.
4. Add `ping` (POST) and `pong` (GET) handlers.
5. Thread `state` through `router(state)` with `.with_state(state)`.

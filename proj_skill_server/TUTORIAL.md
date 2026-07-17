# proj_skill_server

## What you'll build

An HTTP service that discovers "skills" — self-describing directories on
disk, each containing a manifest and a command to run, in the same spirit
as Claude Code / Codex skills — and runs them as managed background jobs.
A client submits a run, polls its status, streams its output live as it
happens, and can cancel it mid-flight. This is the capstone: it composes
axum request handling, a tokio worker-pool job queue, shared mutable
state, cancellation, and SSE streaming into one service, then ties it off
with graceful shutdown.

## Concepts you'll practice

- axum routing, extractors (`Path`, `Json`, `State`), and response types
- `tokio::process::Command` — spawning, piping stdout, waiting, killing
- job-queue architecture: `mpsc` channel + a fixed pool of worker tasks
- per-run shared state behind `Arc<Mutex<HashMap<u64, RunState>>>`
- `tokio_util::sync::CancellationToken` + `tokio::select!` to race a
  child process against a cancel signal
- `tokio::sync::broadcast` + Server-Sent Events (SSE) for live fan-out
  to N subscribers
- graceful shutdown: stop accepting work, let (or force) in-flight work
  finish, exit clean

Maps to *The Rust Programming Language*, ch. 16 (Fearless Concurrency —
16.1 threads/message passing generalizes to tasks/channels, 16.3 shared
state and `Mutex<T>`) and ch. 17 (async/await, streams). Where this
builds on work you've already done:

- **x_axum_tokio_app_chans** — you've already wired axum handlers to an
  mpsc channel and a background consumer. M3 is that pattern again, at
  a slightly bigger scale (N workers instead of 1, a result map instead
  of a single reply channel).
- **x_csp_tokio** — your CSP/select! reflexes are M4's foundation:
  racing two futures and reacting to whichever finishes first is
  exactly `child.wait()` vs `token.cancelled()`.
- **x_tokio_cli_tutorial** — you've done process spawning and
  timeout-bounded waits there already; M2 is the same skill inside an
  axum handler instead of a CLI.

## The contract

This part is given outright — the *what*, not the *how*. Match it
exactly; the definition-of-done curl commands in each milestone assume
you did.

### Skill manifest

A skill is a directory under `skills/<name>/` containing a `skill.json`:

```json
{
  "name": "echo",
  "description": "Echoes the run's input back as output.",
  "command": ["bash", "-c", "echo \"$SKILL_INPUT\""],
  "timeout_secs": 5
}
```

- `command` is the literal argv to spawn (first element is the
  program). It is run with the skill's own directory as its **cwd**.
- The run's input is passed via the **`SKILL_INPUT` environment
  variable** — not appended to argv. (Both are legitimate designs; this
  project picks env var so `command` in the manifest never has to be
  templated or rewritten per run.)
- `timeout_secs` bounds how long the command is allowed to run before
  the server kills it and marks the run failed.

The three sample skills under `skills/` (`echo`, `word_count`,
`slow_count`) already conform to this — read them before you write any
code.

### Endpoints

| Method | Path                     | Request body           | Response |
|--------|--------------------------|-------------------------|----------|
| GET    | `/healthz`               | —                        | `200 "ok"` |
| GET    | `/skills`                | —                        | `200` JSON array of manifests |
| POST   | `/skills/{name}/runs`    | `{"input": "..."}`       | `202 {"run_id": 1}` |
| GET    | `/runs/{id}`             | —                        | `200` run status (below) |
| GET    | `/runs/{id}/events`      | —                        | `200` `text/event-stream` |
| DELETE | `/runs/{id}`             | —                        | `200` on success |

Run status JSON:

```json
{
  "status": "queued",
  "exit_code": null,
  "output": ["line one", "line two"]
}
```

`status` is one of `"queued" | "running" | "succeeded" | "failed" |
"cancelled"`. `exit_code` is `null` until the run finishes. `output` is
every line captured so far, in order.

SSE stream on `/runs/{id}/events`: replay every buffered line first (as
`event: data` lines, one per SSE `data:` frame), then continue with
lines as they arrive live. The stream ends when the run reaches a
terminal status.

Run ids are a `u64` counter, starting at 1, assigned server-side —
never client-supplied.

### Errors

- Unknown skill name (`POST /skills/{name}/runs`) → `404`
- Unknown run id (`GET`/`DELETE` on `/runs/{id}`) → `404`
- `DELETE /runs/{id}` on a run that's already `succeeded` / `failed` /
  `cancelled` → `409`

Pick whatever error body shape you like (plain text is fine) — it's not
specified because it doesn't matter for this project.

## Ground rules

- Every milestone below has three collapsible hints, escalating from
  "here's the concept" to "here's the exact type shape." Try to get
  through each milestone *without opening a hint*. If you're stuck for
  20+ minutes, open Hint 1 — not Hint 3. Skipping straight to Hint 3
  defeats the point; you're allowed to feel a little pained about a
  borrow-checker fight before you look.
- Test everything with `curl`, using the exact commands you're given
  in each Definition of Done. For the SSE endpoint you need `curl -N`
  (disables curl's output buffering, otherwise you won't see lines
  arrive incrementally).
- Run `cargo check` constantly. This project WILL produce lifetime and
  `Send`/`'static` errors that are annoying to read — get in the habit
  of checking after every few lines instead of writing a whole handler
  blind.

---

## Milestone 1 — skeleton + discovery

**Goal:** `GET /healthz` returns `200 "ok"`. At startup, scan the
`skills/` directory, parse every `skill.json` found with serde, and
serve the resulting list at `GET /skills`.

**Design questions**
- Where does the parsed skill list live once discovery is done — a
  `Vec`? a `HashMap<String, Skill>`? What do you index by?
- Why does this need to be behind an `Arc` at all, if it's built once
  at startup and every handler only reads it?
- Does the skill list ever need to mutate after startup? (Not in this
  milestone — but keep the question in mind; it'll come back.)
- What's your plan for a `skills/` subdirectory that has no
  `skill.json`, or one that fails to parse — hard error at startup, or
  skip and log?

**Definition of done**

```
$ curl -s localhost:PORT/healthz
ok

$ curl -s localhost:PORT/skills | jq .
[
  { "name": "echo", "description": "...", "command": [...], "timeout_secs": 5 },
  { "name": "word_count", ... },
  { "name": "slow_count", ... }
]
```

<details><summary>Hint 1 — nudge</summary>

Discovery is a one-shot, startup-time filesystem walk — it has nothing
to do with async yet. Do it before you build the router, with plain
synchronous `std::fs`, and hand the *result* to axum as shared state.
`State<T>` in axum needs `T: Clone`, which is exactly what `Arc` is
for — cloning an `Arc` clones a pointer, not the data.

</details>

<details><summary>Hint 2 — API pointers</summary>

Look up: `std::fs::read_dir`, `std::fs::read_to_string`,
`serde_json::from_str`, `axum::Router::route`, `axum::routing::get`,
`axum::extract::State`, `axum::Json` (as a return type — it implements
`IntoResponse`), `#[derive(serde::Deserialize, serde::Serialize)]` on
your manifest struct. For the 404 case in later milestones you'll want
`axum::http::StatusCode` too — get comfortable with it now.

</details>

<details><summary>Hint 3 — shape</summary>

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SkillManifest {
    name: String,
    description: String,
    command: Vec<String>,
    timeout_secs: u64,
}

#[derive(Clone)]
struct AppState {
    skills: Arc<HashMap<String, SkillManifest>>,
    // more fields join this struct in later milestones
}

fn discover_skills(dir: &std::path::Path) -> HashMap<String, SkillManifest>;

async fn healthz() -> &'static str;
async fn list_skills(State(state): State<AppState>) -> Json<Vec<SkillManifest>>;
```

</details>

---

## Milestone 2 — synchronous run

**Goal:** `POST /skills/{name}/runs` executes the skill's command with
`tokio::process::Command`, cwd set to the skill's directory, waits for
it (bounded by `timeout_secs`), and returns the captured stdout lines
and exit code directly in the HTTP response. No run ids, no queue yet
— the request just blocks until the child is done or times out.

**Design questions**
- If the child hangs past its timeout, what happens to the HTTP
  request that's waiting on it — and separately, what happens to the
  *child process itself* if you just stop awaiting it?
- Why pipe stdout (`Stdio::piped()`) instead of inheriting the
  parent's? What would inheriting even mean for an HTTP server with
  concurrent requests?
- How do you turn "N seconds have passed and the child hasn't exited"
  into an actual `Err` you can turn into a `500`/timeout response?
- Where does `SKILL_INPUT` get set — on the `Command` builder, or by
  mutating the current process's env? (Only one of these is safe with
  concurrent requests — why?)

**Definition of done**

```
$ curl -s -X POST localhost:PORT/skills/echo/runs -d '{"input":"hi"}' | jq .
{ "output": ["hi"], "exit_code": 0 }

$ curl -s -X POST localhost:PORT/skills/word_count/runs -d '{"input":"a b c"}' | jq .
{ "output": ["3"], "exit_code": 0 }

$ curl -s -X POST localhost:PORT/skills/nope/runs -d '{"input":""}' -o /dev/null -w '%{http_code}\n'
404
```

Try `slow_count` here too and watch the request hang for ~10 seconds —
that's the exact pain this milestone is supposed to make you feel,
before M3 fixes it.

<details><summary>Hint 1 — nudge</summary>

This is deliberately the "wrong" architecture — one request, one
blocking wait, no concurrency story. You're building it anyway because
M3's queue+workers only makes sense once you've felt why a synchronous
`POST` that runs an arbitrary child process is a bad idea for a real
server (one slow skill call ties up a request; nothing stops N
concurrent slow calls from spawning N children).

</details>

<details><summary>Hint 2 — API pointers</summary>

Look up: `tokio::process::Command`, `.current_dir()`, `.env()`,
`.stdout(std::process::Stdio::piped())`, `.spawn()` vs `.output()`,
`tokio::time::timeout`, `child.wait_with_output()`. For turning bytes
into lines: `String::from_utf8_lossy` then `.lines()`. `Json<T>` for the
request body extractor, matching the same for the response.

</details>

<details><summary>Hint 3 — shape</summary>

```rust
#[derive(serde::Deserialize)]
struct RunRequest {
    input: String,
}

#[derive(serde::Serialize)]
struct SyncRunResponse {
    output: Vec<String>,
    exit_code: Option<i32>,
}

async fn run_skill_sync(
    manifest: &SkillManifest,
    skill_dir: &std::path::Path,
    input: &str,
) -> Result<SyncRunResponse, RunError>;

async fn post_run(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<RunRequest>,
) -> Result<Json<SyncRunResponse>, StatusCode>;
```

</details>

---

## Milestone 3 — async runs + registry

**Goal:** `POST /skills/{name}/runs` now enqueues a job on an `mpsc`
channel and returns `202` + a `run_id` immediately. A fixed pool of
worker tasks (start with 2) pulls jobs off the channel and executes
them. `GET /runs/{id}` reports live status by reading a shared
`Arc<Mutex<HashMap<u64, RunState>>>`. Status transitions
`queued → running → succeeded/failed`.

**Design questions**
- Who allocates the run id — the handler that receives the POST, or
  the worker that eventually picks the job up? Why does it matter for
  the client getting a usable id back in the `202` response?
- What does a worker hold locked while the child process is actually
  running? (The answer you're aiming for: nothing — lock the map,
  write the new state, unlock, *then* await the child. Lock again only
  to write the final state.)
- What actually breaks if you `.await` while holding a `std::sync::
  Mutex` guard across that await point — is it a compile error or a
  runtime deadlock, and why the difference between `std::sync::Mutex`
  and `tokio::sync::Mutex`? (Book ch. 16.3 covers `Mutex<T>` for
  threads; the tokio docs for `tokio::sync::Mutex` explain why an async
  context needs its own version — read both and articulate the
  difference in your own words before moving on.)
- With N=2 workers and 3 slow_count runs queued, what does `GET /runs/3`
  say while runs 1 and 2 are still in flight?
- Does the channel need a bounded or unbounded size? What's the
  failure mode of each if producers outpace workers?

**Definition of done**

```
$ curl -s -X POST localhost:PORT/skills/slow_count/runs -d '{"input":""}'
{"run_id":1}

$ curl -s localhost:PORT/runs/1
{"status":"queued","exit_code":null,"output":[]}

# a few seconds later
$ curl -s localhost:PORT/runs/1
{"status":"running","exit_code":null,"output":["1","2","3"]}

# after it finishes
$ curl -s localhost:PORT/runs/1
{"status":"succeeded","exit_code":0,"output":["1","2","3","4","5","6","7","8","9","10"]}

$ curl -s localhost:PORT/runs/999 -o /dev/null -w '%{http_code}\n'
404
```

<details><summary>Hint 1 — nudge</summary>

This is the producer/consumer pattern from x_axum_tokio_app_chans,
scaled from one consumer to a small pool. The handler's whole job is:
allocate an id, record `queued` state, push a job onto the channel,
respond `202`. It never touches the child process. All process
management moves into the worker loop.

</details>

<details><summary>Hint 2 — API pointers</summary>

Look up: `tokio::sync::mpsc::channel` (bounded — think about why over
`unbounded_channel`), cloning an `mpsc::Sender` into `N` spawned
worker tasks vs. sharing one `Receiver` (a `Receiver` isn't `Clone` —
how do multiple workers pull from one channel? — `Arc<tokio::sync::
Mutex<Receiver<T>>>` is one answer, worth comparing to alternatives),
`std::sync::atomic::AtomicU64` for the id counter (or a `Mutex<u64>`),
`tokio::spawn` in a loop to start your worker pool at startup.

</details>

<details><summary>Hint 3 — shape</summary>

```rust
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum RunStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, serde::Serialize)]
struct RunState {
    status: RunStatus,
    exit_code: Option<i32>,
    output: Vec<String>,
}

struct Job {
    run_id: u64,
    manifest: SkillManifest,
    skill_dir: std::path::PathBuf,
    input: String,
}

#[derive(Clone)]
struct AppState {
    skills: Arc<HashMap<String, SkillManifest>>,
    runs: Arc<Mutex<HashMap<u64, RunState>>>,
    next_run_id: Arc<std::sync::atomic::AtomicU64>,
    job_tx: tokio::sync::mpsc::Sender<Job>,
}

async fn worker_loop(rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<Job>>>, runs: Arc<Mutex<HashMap<u64, RunState>>>);

#[derive(serde::Serialize)]
struct PostRunResponse {
    run_id: u64,
}
```

</details>

---

## Milestone 4 — cancellation

**Goal:** `DELETE /runs/{id}` cancels a queued or running run. Each
run gets its own `CancellationToken`. The worker races
`child.wait()` against `token.cancelled()` with `tokio::select!`;
losing means killing the child and setting status to `cancelled`.
Cancelling a *queued* (not yet started) run must also work — the
worker has to check the token before it bothers spawning anything.

**Definition of done** uses `slow_count`. Cancel it mid-run, then
verify with `ps` that the underlying `bash` process is actually gone —
not just that your HTTP status says `cancelled`.

```
$ curl -s -X POST localhost:PORT/skills/slow_count/runs -d '{"input":""}'
{"run_id":1}

$ sleep 3

$ curl -s -X DELETE localhost:PORT/runs/1 -o /dev/null -w '%{http_code}\n'
200

$ curl -s localhost:PORT/runs/1
{"status":"cancelled","exit_code":null,"output":["1","2","3"]}

$ ps aux | grep -c '[s]eq 1 10'
0

# cancelling a run that's already finished:
$ curl -s -X DELETE localhost:PORT/runs/1 -o /dev/null -w '%{http_code}\n'
409
```

**Design questions**
- Where does the `CancellationToken` for a run live so both the
  `DELETE` handler and the worker can reach it? (Same map as
  `RunState`, a field on it? A second map keyed by run id?)
- `tokio::select!` runs both branches' futures concurrently and takes
  whichever finishes first — but what happens to the *other* branch's
  future when one wins? Does `child.wait()` losing the race leave the
  child running?
- After `token.cancelled()` wins the race, what's the actual API call
  to make sure the OS process is gone, and do you still need to
  `.wait()` on it afterward? (Think about what a zombie process is.)
- How does a worker that's about to pop a *queued* job off the channel
  know it was already cancelled before it ever started? Does the
  `Job` need the token, or a lookup into `RunState` first?

<details><summary>Hint 1 — nudge</summary>

This is your CSP `select!` reflex from x_csp_tokio, applied to a real
child process instead of two toy futures. The core shape doesn't
change: race two futures, act on whichever resolves first, and make
sure the loser is actually stopped, not just ignored.

</details>

<details><summary>Hint 2 — API pointers</summary>

Look up: `tokio_util::sync::CancellationToken`, `.cancel()`,
`.cancelled()` (returns a future), `tokio::select!` branch syntax,
`tokio::process::Child::kill()` (async, look at its signature
carefully — does it also need a `.wait()` after?), `Child::id()` if
you want to cross-check with `ps` yourself.

</details>

<details><summary>Hint 3 — shape</summary>

```rust
struct RunHandle {
    state: RunState,
    token: tokio_util::sync::CancellationToken,
}

// runs map becomes Arc<Mutex<HashMap<u64, RunHandle>>>

async fn run_with_cancellation(
    child: tokio::process::Child,
    token: tokio_util::sync::CancellationToken,
) -> RunOutcome;

enum RunOutcome {
    Finished(std::process::ExitStatus),
    Cancelled,
}

async fn delete_run(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<StatusCode, StatusCode>;
```

</details>

---

## Milestone 5 — live output over SSE

**Goal:** the worker now reads the child's stdout line-by-line *while
it runs* (not after exit), appending each line to `RunState.output`
and publishing it to a per-run `broadcast` channel at the same time.
`GET /runs/{id}/events` returns an SSE stream: it first replays every
buffered line already in `RunState.output`, then continues with lines
as they arrive live from the broadcast channel.

**Design questions**
- Why `broadcast` and not `mpsc` for this? What property does a
  live-output subscriber need that `mpsc` (single consumer) doesn't
  give you?
- A client connects to `/runs/1/events` after line 5 has already been
  produced. What must the handler do before it starts forwarding
  broadcast messages, to avoid the client missing lines 1-5 or seeing
  them twice?
- A client connects *after* the run has already finished. What should
  the stream do — replay everything then close immediately, or error?
- `broadcast::Receiver::recv()` can return `Err(Lagged(n))` if the
  subscriber falls behind and the channel's ring buffer overwrites
  unread messages. Given you're also storing every line in
  `RunState.output`, does a lagged subscriber actually lose data, or
  just lose *live* delivery of some lines?

**Definition of done**

```
$ curl -s -X POST localhost:PORT/skills/slow_count/runs -d '{"input":""}'
{"run_id":1}

$ curl -N localhost:PORT/runs/1/events
data: 1

data: 2

data: 3
...
```

Lines should visibly arrive about one per second, not all at once at
the end. Open a second terminal and start a second `curl -N .../events`
against the same run_id partway through — it should immediately show
the buffered lines so far, then keep pace with the first stream.

<details><summary>Hint 1 — nudge</summary>

Two separate jobs are riding on the same string of output: durable
storage (append to `RunState.output`, always, so `GET /runs/{id}`
keeps working) and live fan-out (publish to `broadcast`, best-effort,
only for currently-connected SSE clients). Do both every time a line
arrives — don't try to make the broadcast channel double as your
source of truth.

</details>

<details><summary>Hint 2 — API pointers</summary>

Look up: `tokio::io::BufReader` + `AsyncBufReadExt::lines()` over
`child.stdout.take()`, `tokio::sync::broadcast::channel`, `Sender::
subscribe()`, `axum::response::sse::{Sse, Event}`, `futures::stream::
Stream` (or `tokio_stream::Stream`), `tokio_stream::wrappers::
BroadcastStream`, `futures::stream::iter` for the replay half, and
`.chain()` or a manual `async_stream`-style generator to combine
"replay buffered lines" then "forward live ones" into one `Stream`
item type axum's `Sse` can consume.

</details>

<details><summary>Hint 3 — shape</summary>

```rust
struct RunHandle {
    state: RunState,
    token: tokio_util::sync::CancellationToken,
    output_tx: tokio::sync::broadcast::Sender<String>,
}

async fn get_run_events(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>, StatusCode>;
```

(`tokio_stream::Stream` is a re-export of the same `Stream` trait axum's `Sse` expects —
no extra `futures` dependency needed.)

</details>

---

## Milestone 6 — graceful shutdown

**Goal:** `SIGINT` (ctrl-C) stops the server from accepting new work,
then either cancels every in-flight run's `CancellationToken` or gives
running jobs a bounded deadline to finish on their own — pick one and
be able to say why. Workers drain (finish or get cancelled), and the
process exits cleanly with no orphaned child processes.

**Design questions**
- "Cancel everything immediately" vs "give running jobs N seconds to
  finish, then cancel": what's the actual tradeoff, and which fits a
  skill-runner where a skill might be mid-write to a file?
- How do you enumerate "every currently running token" at shutdown
  time — iterate the runs map and pull each `RunHandle`'s token?
- What signals a worker loop to stop pulling *new* jobs from the queue
  versus what signals it to abandon the job it's currently running?
  Are those the same signal or two different ones?
- axum's graceful shutdown hook needs a future that resolves when you
  want to stop — where does ctrl-C detection plug into that?

**Definition of done**

```
$ cargo run &
$ curl -s -X POST localhost:PORT/skills/slow_count/runs -d '{"input":""}'
{"run_id":1}
$ sleep 2
$ kill -INT %1     # or ctrl-C if run in foreground
```

Expected: the server logs (or otherwise shows) that it's shutting
down, run 1 ends up `cancelled` (or finishes, if you chose the
deadline approach and it fit), the process exits with status 0, and
`ps aux | grep '[s]eq 1 10'` shows nothing left behind.

<details><summary>Hint 1 — nudge</summary>

This composes two things you already have: `CancellationToken` (M4)
and axum's `with_graceful_shutdown`. The new piece is *detecting*
ctrl-C and turning it into "cancel every token in the runs map," not
building a new cancellation mechanism.

</details>

<details><summary>Hint 2 — API pointers</summary>

Look up: `tokio::signal::ctrl_c()`, `axum::serve(...).with_graceful_
shutdown(future)`, iterating a `HashMap<u64, RunHandle>` under a
`Mutex` guard to call `.cancel()` on each token, and — if you want the
"stop accepting new work" half — a top-level `CancellationToken` that
gates the worker pool's "pull from queue" loop the same way a per-run
token gates a single child.

</details>

<details><summary>Hint 3 — shape</summary>

```rust
async fn shutdown_signal(state: AppState) {
    // waits on ctrl_c(), then walks state.runs and cancels every token
}

// wiring:
// axum::serve(listener, app).with_graceful_shutdown(shutdown_signal(state.clone()))
```

</details>

---

## Errors you'll probably meet

- **Holding a `std::sync::MutexGuard` across an `.await`.** With
  `std::sync::Mutex` this is either a compiler error (the guard isn't
  `Send`, so the whole async block/future isn't `Send`, and tokio's
  multi-threaded runtime requires spawned futures to be `Send`) or, if
  it does compile, a real deadlock risk — the lock stays held for the
  entire await, blocking every other task that wants it, including
  ones on other worker threads. The fix isn't a different mutex; it's
  restructuring so you lock, read/write, and drop the guard *before*
  any `.await` point.

- **`error[E0382]: use of moved value` on a `Sender` captured by a
  handler closure.** Each axum handler closure captures state by move.
  If you try to use the same `Sender` (or any non-`Clone`-cloned value)
  in two different route closures, the first one's capture moves it
  and the second can't see it anymore. The fix is always "clone before
  you move into the closure" — but *why* it happens is the borrow
  checker correctly refusing to let two independent futures share one
  non-`Sync`-shared owner.

- **A wall of trait errors saying your handler function doesn't
  implement `Handler`.** This is axum's extractor/return-type
  machinery failing to unify, and the real error is usually buried
  under generic noise. Debug by checking, in order: every extractor's
  type matches what the request actually sends (`Json<T>` needs a
  `Content-Type: application/json` body that deserializes into `T`),
  the *last* extractor is the only one allowed to consume the body,
  and your return type implements `IntoResponse`. Add
  `#[axum::debug_handler]` above the function temporarily — it gives a
  far more specific error pointing at the actual mismatched argument.

- **Zombie children.** Calling `Child::kill()` sends the kill signal
  but doesn't reap the process — the OS keeps a zombie entry until
  something calls `wait()` on it. If your select! loop kills the
  child on cancellation and doesn't also await its exit afterward,
  you'll accumulate zombies under load. `ps aux` showing `<defunct>`
  entries after cancelling several runs is this exact bug.

- **SSE stream type gymnastics.** `axum::response::sse::Sse` wants a
  single concrete `Stream<Item = Result<Event, E>>` type, but "replay
  buffered lines, then follow a broadcast channel" is naturally two
  different stream sources with two different concrete types. Rust
  won't let you return "either of two stream types" without erasing
  them to the same type — via `Box::pin(dyn Stream<...>)`, an enum
  wrapping both variants that itself implements `Stream`, or chaining
  them with `.chain()` if both sides already agree on `Item`.
  `tokio_stream::wrappers::BroadcastStream` gets you a `Stream` out of
  a `broadcast::Receiver`, but its item type is
  `Result<T, BroadcastStreamRecvError>`, not your SSE `Event` — you
  still need a `.map()` to convert.

## Stretch goals

- Add a `claude` skill whose command shells out to `claude -p
  "$SKILL_INPUT"`, making this a real agentic task runner instead of a
  toy — `SKILL_INPUT` was chosen as the input-passing mechanism
  specifically so this drops in without changing the contract.
- Per-skill concurrency limits with a `tokio::sync::Semaphore` (e.g.
  `slow_count` allows only 1 concurrent run, `echo` allows 10) —
  acquired by the worker before running a job for that skill, not by
  the queue itself.
- Persist finished runs to disk as JSON on completion and reload them
  at startup, so `GET /runs/{id}` survives a server restart for runs
  that already finished.
- Swap SSE for a WebSocket duplex — same live-output use case, but now
  the client could also send a cancel message down the same socket
  instead of a separate `DELETE`.

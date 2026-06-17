# A Tokio Tour — Learning Async Rust From Scratch

This is a complete, build-it-yourself walkthrough. It assumes you know how to
program but have **never written Rust**. We build a command-line program where
each subcommand demonstrates one major feature of **Tokio**, Rust's async
runtime. Every Rust concept is explained the first time it appears.

By the end you'll understand: ownership and `move`, `Result` and `?`, traits,
closures, `async`/`.await`, and the core Tokio building blocks (tasks, channels,
shared state, cancellation, I/O).

---

## Part 0 — The ideas before the code

### What problem does async solve?

A program often spends most of its time *waiting* — for the network, the disk, a
timer. A naive program waits by blocking: the whole thread stops until the data
arrives. If you want to handle 1,000 network connections, you'd need 1,000
threads, each mostly asleep. Threads are expensive (each needs its own stack,
~MBs of memory), so that doesn't scale.

**Async** lets a small number of threads juggle thousands of waiting tasks. When
a task hits a "wait here" point, it *yields* control back to a scheduler instead
of blocking the thread, and the thread goes to run some other ready task. When
the data is ready, the task resumes.

### What is Tokio?

Rust's standard library defines *what* an async task looks like (the `Future`
trait) but deliberately ships **no scheduler to run them**. Tokio is the most
popular library that provides that scheduler (the "runtime") plus async versions
of timers, channels, networking, and files. You bring Tokio in as a dependency.

### What we're building

A CLI called `tokio-tour`. You run a lesson by name:

```
cargo run -p x_tokio_cli_tutorial -- spawn 8
cargo run -p x_tokio_cli_tutorial -- echo 127.0.0.1:9000
```

(The `--` separates Cargo's own arguments from your program's arguments.)
Each lesson is a small, focused demo of one feature.

---

## Part 1 — Cargo, crates, and dependencies

Rust's build tool and package manager is **Cargo**. A *package* is a directory
with a `Cargo.toml` manifest. A *crate* is a compiled unit — our package builds
one binary crate. This project already exists as `x_tokio_cli_tutorial`.

Open `Cargo.toml` and make the `[dependencies]` section look like this:

```toml
[dependencies]
anyhow = "1"
clap = { version = "4.6.1", features = ["derive"] }
tokio = { version = "1.52.3", features = ["full"] }
tokio-stream = "0.1"
tokio-util = "0.7"
```

What each line means:

- **`anyhow`** — an easy error type so any function can return "some error" with
  a `?`. (More on `?` soon.)
- **`clap`** — command-line argument parser. `features = ["derive"]` turns on the
  ability to *derive* a parser from a struct (explained in Part 3).
- **`tokio`** — the runtime. `features = ["full"]` enables everything while
  learning. Tokio is modular; later you can list only the pieces you use.
- **`tokio-stream`** — async iterators (Lesson 12).
- **`tokio-util`** — extra utilities; we use its `CancellationToken` (Lesson 11).

The `"1"` / `"4.6.1"` strings are version requirements. `"1"` means "any 1.x".

> **Rust concept — the `{ ... }` form.** `clap = "4.6.1"` is shorthand. When you
> need to pass options like `features`, you switch to the table form
> `clap = { version = "...", features = [...] }`. They mean the same kind of
> thing: a dependency with settings.

---

## Part 2 — Modules: how the files connect

Rust code is organized into **modules**. By default a module is private; you make
items visible with `pub`. Our layout:

```
src/
  main.rs            # the binary's entry point; declares the `lessons` module
  lessons/
    mod.rs           # declares each lesson file as a submodule
    spawn.rs         # one lesson
    timers.rs        # ...
```

In `main.rs`, the line `mod lessons;` tells the compiler "there is a module named
`lessons`; find it in `lessons/mod.rs` (or `lessons.rs`)." Inside `lessons/mod.rs`
we list each file:

```rust
// src/lessons/mod.rs
pub mod spawn;
pub mod timers;
pub mod select;
pub mod join;
pub mod mpsc;
pub mod oneshot;
pub mod broadcast;
pub mod watch;
pub mod shared_state;
pub mod blocking;
pub mod cancel;
pub mod streams;
pub mod echo;
pub mod signal;
```

> **Rust concept — paths.** `lessons::spawn::run` means "the `run` item, inside
> the `spawn` module, inside the `lessons` module." `::` is the path separator
> (like `/` in a filesystem). `pub mod spawn;` makes `spawn` reachable from
> outside `lessons`.

Create `src/lessons/mod.rs` with the content above now. The individual lesson
files come in Part 4. Until they exist, the project won't compile — that's fine;
we'll add them one at a time.

---

## Part 3 — The entry point (`main.rs`)

Here is the complete `main.rs`. Read it once, then we'll dissect it.

```rust
//! A guided tour of Tokio's major features. Run `--help` for the menu.

mod lessons;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tokio-tour", about = "A guided tour of Tokio")]
struct Cli {
    #[command(subcommand)]
    lesson: Lesson,
}

#[derive(Subcommand)]
enum Lesson {
    /// Spawn concurrent tasks and await their results
    Spawn {
        #[arg(default_value_t = 5)]
        tasks: u64,
    },
    /// sleep, interval, and timeout
    Timers,
    /// Race futures with select!
    Select,
    /// Run futures concurrently with join!
    Join,
    /// Many-producer, single-consumer channel
    Mpsc {
        #[arg(default_value_t = 3)]
        producers: u64,
    },
    /// Single request -> response
    Oneshot,
    /// Fan-out to every subscriber
    Broadcast,
    /// Propagate the latest state to watchers
    Watch,
    /// Share a counter across tasks with Arc<Mutex>
    SharedState {
        #[arg(default_value_t = 8)]
        tasks: u64,
    },
    /// Offload blocking work with spawn_blocking
    Blocking,
    /// Graceful cancellation with CancellationToken
    Cancel,
    /// Async streams
    Streams,
    /// TCP echo server
    Echo {
        #[arg(default_value = "127.0.0.1:8080")]
        addr: String,
    },
    /// Graceful shutdown on Ctrl-C
    Signal,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.lesson {
        Lesson::Spawn { tasks } => lessons::spawn::run(tasks).await,
        Lesson::Timers => lessons::timers::run().await,
        Lesson::Select => lessons::select::run().await,
        Lesson::Join => lessons::join::run().await,
        Lesson::Mpsc { producers } => lessons::mpsc::run(producers).await,
        Lesson::Oneshot => lessons::oneshot::run().await,
        Lesson::Broadcast => lessons::broadcast::run().await,
        Lesson::Watch => lessons::watch::run().await,
        Lesson::SharedState { tasks } => lessons::shared_state::run(tasks).await,
        Lesson::Blocking => lessons::blocking::run().await,
        Lesson::Cancel => lessons::cancel::run().await,
        Lesson::Streams => lessons::streams::run().await,
        Lesson::Echo { addr } => lessons::echo::run(addr).await,
        Lesson::Signal => lessons::signal::run().await,
    }
}
```

Now the Rust, piece by piece.

### Comments and attributes

- `//! ...` is a *doc comment* for the enclosing item (here, the whole file).
  `/// ...` documents the item that follows it. Regular comments are `//`.
- `#[derive(Parser)]` and `#[command(...)]` are **attributes** — metadata
  attached to the item below them. `derive` asks the compiler (or a library) to
  auto-generate code. clap reads these to build the argument parser for us.

### Structs and enums

```rust
struct Cli {
    lesson: Lesson,
}
```

A `struct` groups named fields. `Cli` has one field, `lesson`, of type `Lesson`.

```rust
enum Lesson {
    Timers,
    Spawn { tasks: u64 },
    Echo { addr: String },
    // ...
}
```

An `enum` is a type that is **exactly one of several variants**. `Timers` is a
bare variant. `Spawn { tasks: u64 }` is a variant that *carries data* — a `u64`
(unsigned 64-bit integer) named `tasks`. `Echo` carries a `String` (an owned,
growable text buffer). This is how clap models "either the timers command, or
the spawn command with a task count, or …".

> Rust enums are far more powerful than enums in C/Java: each variant can hold
> different data. This is the same mechanism behind `Option` and `Result` below.

### clap attributes

- `#[command(subcommand)]` on the `lesson` field: "fill this field by parsing
  which subcommand the user typed."
- `#[arg(default_value_t = 5)]`: if the user omits this argument, use `5`.
  (`_t` means the default is a typed value; for strings clap uses
  `default_value = "..."`.)
- Each `///` doc comment becomes that subcommand's `--help` text. Free docs.

### `#[tokio::main]` and `async fn main`

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> { ... }
```

A normal Rust program's entry point is `fn main()`. Two changes here:

1. **`async fn`** marks a function as asynchronous. Calling it doesn't run it; it
   returns a *future* (a value representing "work that can be driven to
   completion"). Futures do nothing until something *awaits* or *runs* them.
2. **`#[tokio::main]`** is a macro that rewrites your `async fn main` into a
   normal `fn main` that (a) starts a Tokio runtime and (b) runs your async code
   on it to completion. Conceptually it expands to:
   ```rust
   fn main() -> anyhow::Result<()> {
       tokio::runtime::Runtime::new().unwrap().block_on(async { /* your body */ })
   }
   ```
   So `#[tokio::main]` is just convenience — there's no magic.

### Return type: `anyhow::Result<()>`

`Result<T, E>` is a built-in enum with two variants: `Ok(T)` (success, carrying a
value of type `T`) and `Err(E)` (failure, carrying an error). `anyhow::Result<()>`
is shorthand for `Result<(), anyhow::Error>`.

`()` is the **unit type** — "no meaningful value," like `void`. So this function
returns either `Ok(())` (it worked) or an error. Returning a `Result` from `main`
makes Rust print the error and exit non-zero if it's `Err`.

### The body

```rust
let cli = Cli::parse();
```

`let` binds a variable. `Cli::parse()` is a function clap generated for us (via
`#[derive(Parser)]`); it reads `std::env::args`, parses them, and returns a `Cli`
— or prints an error/help and exits. Variables are **immutable by default**; you'd
write `let mut x` to allow reassignment.

```rust
match cli.lesson {
    Lesson::Spawn { tasks } => lessons::spawn::run(tasks).await,
    Lesson::Timers => lessons::timers::run().await,
    // ...
}
```

`match` is pattern matching — like a `switch` that must be **exhaustive** (cover
every variant) and that can *destructure* data out of variants. The pattern
`Lesson::Spawn { tasks }` matches the `Spawn` variant and binds its inner field to
a local `tasks`. Each arm is `pattern => expression`.

Each arm calls a lesson's `run` and writes `.await` after it. `.await` drives that
future to completion and produces its result. Because each `run` returns
`anyhow::Result<()>`, the `match` evaluates to a `Result`, which becomes `main`'s
return value (the `match` is the last expression in the function, and a trailing
expression with no semicolon is the return value — Rust is expression-oriented).

---

## Part 4 — The lessons

Build them in order. After writing each file you can run it immediately, e.g.
`cargo run -p x_tokio_cli_tutorial -- spawn`. New Rust concepts are explained as
they first appear.

### Lesson 1 — `spawn`: running tasks concurrently

```rust
// src/lessons/spawn.rs
//! `tokio::spawn` hands a future to the runtime to run concurrently.

use std::time::Duration;
use tokio::time::sleep;

pub async fn run(tasks: u64) -> anyhow::Result<()> {
    let mut handles = Vec::new();

    for id in 0..tasks {
        let handle = tokio::spawn(async move {
            // Finish in reverse order so you can see them interleave.
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
    println!("sum of squares = {sum}");
    Ok(())
}
```

**Rust concepts here:**

- `use std::time::Duration;` brings a type into scope so you can write `Duration`
  instead of the full path. `std` is the standard library.
- `pub async fn run(tasks: u64) -> anyhow::Result<()>` — public, async, takes a
  `u64`, returns a `Result`. This is the shape *every* lesson shares.
- `let mut handles = Vec::new();` — `Vec<T>` is a growable array. `mut` because we
  push into it. Rust infers the element type from how we use it.
- `for id in 0..tasks` — `0..tasks` is a *range*; the loop runs `id = 0, 1, …`.
- **Closures.** `async move { ... }` is an *async block*: it produces a future.
  The `move` keyword forces the block to **take ownership** of the variables it
  uses from the surrounding scope (`id`, `tasks`). This matters because the task
  may outlive this loop iteration and even run on another thread, so it cannot
  borrow locals — it must own copies. (`u64` is `Copy`, so "taking ownership"
  here just copies the number.)
- `tokio::spawn(future)` schedules the future on the runtime and returns a
  `JoinHandle` immediately — it does **not** wait. That's the whole point: all
  tasks are launched, then run concurrently.
- `handle.await?` — `.await` waits for that task to finish and yields its return
  value, wrapped in a `Result` (a task could panic). The **`?` operator**: if the
  result is `Err`, return it from `run` immediately; if `Ok(v)`, evaluate to `v`.
  So `sum += handle.await?;` adds the task's returned number, propagating any
  failure up to `main`.
- `println!("task {id} done")` — `println!` is a macro (the `!` marks macros).
  `{id}` interpolates the variable `id` directly into the string.
- The async block's last expression `id * id` (no semicolon) is the task's return
  value.
- `Ok(())` at the end: success with the unit value.

> **The key lesson:** we `spawn` *all* tasks into `handles` first, then await them
> in a second loop. If you instead wrote `tokio::spawn(...).await` inside the
> first loop, each task would finish before the next started — fully sequential,
> no concurrency. Launch first, collect second.

Run it: `cargo run -p x_tokio_cli_tutorial -- spawn 5`. Notice tasks print out of
order but the sum is correct.

### Lesson 2 — `timers`: sleep, interval, timeout

```rust
// src/lessons/timers.rs
use std::time::Duration;
use tokio::time::{interval, sleep, timeout};

pub async fn run() -> anyhow::Result<()> {
    println!("sleeping 200ms...");
    sleep(Duration::from_millis(200)).await;

    // interval ticks repeatedly. The FIRST tick fires immediately.
    let mut ticker = interval(Duration::from_millis(100));
    for n in 1..=3 {
        ticker.tick().await;
        println!("tick {n}");
    }

    // timeout bounds how long a future may run.
    let slow = sleep(Duration::from_secs(10));
    match timeout(Duration::from_millis(150), slow).await {
        Ok(_) => println!("finished in time"),
        Err(_) => println!("timed out (as expected)"),
    }
    Ok(())
}
```

**New Rust:**

- `use tokio::time::{interval, sleep, timeout};` — the `{}` imports several names
  from one path at once.
- `1..=3` is an *inclusive* range (1, 2, 3); `1..3` would stop at 2.
- `match` again, here on the `Result` that `timeout` returns. `Ok(_)`/`Err(_)`:
  the `_` is a wildcard pattern — "I don't care about the inner value."

**Tokio points:** `sleep` suspends only *this* task; the thread stays free for
other tasks. `interval`'s first `tick()` returns instantly (a common surprise).
`timeout(dur, fut)` returns `Err` if `fut` doesn't complete within `dur`.

### Lesson 3 — `select`: race two futures

```rust
// src/lessons/select.rs
use std::time::Duration;
use tokio::time::sleep;

pub async fn run() -> anyhow::Result<()> {
    tokio::select! {
        _ = sleep(Duration::from_millis(100)) => println!("fast branch won"),
        _ = sleep(Duration::from_millis(300)) => println!("slow branch won"),
    }
    Ok(())
}
```

`tokio::select!` is a macro that runs several futures at once and proceeds with
**whichever finishes first**; the losing futures are dropped (cancelled) right
where they were. Each branch is `pattern = future => body`. Here we ignore each
future's output with `_`.

> This "drop the loser" behavior is how cancellation works throughout async Rust.
> A subtlety to learn later: a future dropped mid-`.await` loses partial progress,
> so branches should be "cancel-safe." Fine for sleeps.

### Lesson 4 — `join`: run concurrently, wait for all

```rust
// src/lessons/join.rs
use std::time::Duration;
use tokio::time::sleep;

async fn fetch(name: &str, ms: u64) -> String {
    sleep(Duration::from_millis(ms)).await;
    format!("{name} ({ms}ms)")
}

pub async fn run() -> anyhow::Result<()> {
    let (a, b, c) = tokio::join!(
        fetch("a", 150),
        fetch("b", 100),
        fetch("c", 200),
    );
    println!("{a}, {b}, {c}");
    Ok(())
}
```

**New Rust:**

- A helper `async fn fetch`. `name: &str` is a **string slice** — a *borrowed*
  view into text the caller owns (no copy). `&` means "a reference / borrow."
- `format!` builds a `String` (like `println!` but returns the text).
- `let (a, b, c) = ...;` destructures a **tuple** (a fixed-size group of values)
  into three variables.

**Tokio point:** `join!` polls all the futures on the *current* task concurrently
and returns when all are done, as a tuple of their outputs. Total time ≈ the
slowest (200ms), not the sum (450ms). Unlike `spawn`, nothing moves to another
thread, so there are no `Send`/`'static` requirements. Use `try_join!` to stop
early if one returns `Err`.

### Lesson 5 — `mpsc`: many producers, one consumer

```rust
// src/lessons/mpsc.rs
use tokio::sync::mpsc;

pub async fn run(producers: u64) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<String>(32);

    for p in 0..producers {
        let tx = tx.clone();
        tokio::spawn(async move {
            for i in 0..3 {
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
```

**Tokio:** `mpsc` = *multi-producer, single-consumer* channel. `channel(32)`
returns a sender `tx` and receiver `rx`; `32` is the buffer capacity. Cloning
`tx` lets each producer task own its own sender. `tx.send(..).await` may wait if
the buffer is full (backpressure). `rx.recv().await` yields `Some(msg)` for each
message and `None` once **all** senders have dropped.

**New Rust:**

- `mpsc::channel::<String>(32)` — the `::<String>` is a *turbofish*: it explicitly
  states the generic type (messages are `String`). Often inferred; shown here for
  clarity.
- `let tx = tx.clone();` *shadows* the outer `tx` with a per-task clone. Shadowing
  (reusing a name) is idiomatic in Rust.
- `.unwrap()` takes a `Result` and either returns the `Ok` value or **panics** on
  `Err`. Fine in a demo; in real code prefer `?`.
- `drop(tx)` explicitly destroys our remaining sender. **This is the classic
  gotcha:** if you forget it, a live sender still exists, so `rx.recv()` never
  returns `None` and the `while let` loops forever.
- `while let Some(msg) = rx.recv().await` loops as long as the pattern matches
  `Some(..)`, stopping at `None`.

### Lesson 6 — `oneshot`: a single reply

```rust
// src/lessons/oneshot.rs
use tokio::sync::oneshot;

pub async fn run() -> anyhow::Result<()> {
    let (tx, rx) = oneshot::channel::<u64>();

    tokio::spawn(async move {
        let _ = tx.send(42); // send consumes tx; can be called only once
    });

    let answer = rx.await?;
    println!("worker replied: {answer}");
    Ok(())
}
```

`oneshot` carries **exactly one value, once** — the request/response idiom.
`rx` itself is awaitable (no `.recv()`). `let _ = ...` deliberately ignores the
returned `Result` (send fails only if the receiver was dropped).

### Lesson 7 — `broadcast`: fan-out to all

```rust
// src/lessons/broadcast.rs
use tokio::sync::broadcast;

pub async fn run() -> anyhow::Result<()> {
    let (tx, _) = broadcast::channel::<u64>(16);

    let mut handles = Vec::new();
    for id in 0..3 {
        let mut rx = tx.subscribe(); // subscribe BEFORE sending
        handles.push(tokio::spawn(async move {
            while let Ok(v) = rx.recv().await {
                println!("subscriber {id} received {v}");
            }
        }));
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
```

Every subscriber receives **every** value. Each calls `tx.subscribe()` to get its
own receiver. If a subscriber falls behind, `recv()` can return a `Lagged` error
(messages dropped) — in real code you'd match on it; here any `Err` ends the loop.

### Lesson 8 — `watch`: latest value wins

```rust
// src/lessons/watch.rs
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::sleep;

pub async fn run() -> anyhow::Result<()> {
    let (tx, mut rx) = watch::channel("starting");

    let consumer = tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            // borrow() yields a read guard; * dereferences to the value
            println!("state -> {}", *rx.borrow());
        }
    });

    for state in ["running", "draining", "stopped"] {
        sleep(Duration::from_millis(50)).await;
        tx.send(state).unwrap();
    }
    drop(tx);
    let _ = consumer.await;
    Ok(())
}
```

`watch` keeps only the **most recent** value — perfect for config/state. Consumers
`changed().await` then read the current value with `borrow()`.

**New Rust:** `*rx.borrow()` — `borrow()` returns a temporary read *guard*; the
`*` (dereference) reads the value through it. `.is_ok()` asks a `Result` "are you
`Ok`?" returning a `bool`.

> **watch vs broadcast:** broadcast delivers every message; watch *coalesces* — a
> slow watcher may skip intermediate states and only see the latest. That's the
> design intent.

### Lesson 9 — `shared-state`: `Arc<Mutex<T>>`

```rust
// src/lessons/shared_state.rs
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run(tasks: u64) -> anyhow::Result<()> {
    let counter = Arc::new(Mutex::new(0u64));

    let mut handles = Vec::new();
    for _ in 0..tasks {
        let counter = Arc::clone(&counter);
        handles.push(tokio::spawn(async move {
            for _ in 0..1000 {
                let mut n = counter.lock().await;
                *n += 1;
            }
        }));
    }
    for h in handles {
        h.await?;
    }

    println!("final = {} (expected {})", *counter.lock().await, tasks * 1000);
    Ok(())
}
```

This is the heart of Rust's "fearless concurrency," so it gets the most detail.

- **`Arc<T>`** = *Atomically Reference-Counted* pointer. Ownership in Rust is
  normally singular — one owner frees the value. To share one value among many
  tasks/threads, you wrap it in `Arc`. Each `Arc::clone` makes another handle and
  bumps a counter; the value is freed when the last handle drops. `Arc::clone` is
  cheap (it copies a pointer, not the data).
- **`Mutex<T>`** provides *interior mutability* with mutual exclusion: only one
  task holds the lock at a time. `lock().await` waits for the lock and returns a
  *guard*; `*n += 1` mutates through it; the lock releases when the guard goes out
  of scope. We need this because `Arc` only gives *shared* (read-only) access —
  the `Mutex` is what makes mutation safe.
- `for _ in 0..tasks` — `_` as the loop variable means "I don't use it."
- `let counter = Arc::clone(&counter);` — shadow with a per-task clone so each
  task owns a handle to move into its `async move` block.

**Candid, important guidance:** here we lock, do a trivial `+= 1`, and unlock —
**no `.await` happens while the lock is held.** In that situation
`std::sync::Mutex` is actually the better choice: it's faster and simpler. Use
**`tokio::sync::Mutex` only when you must hold the lock across an `.await`** (e.g.
you make a network call while holding it). Beginners reach for the Tokio mutex by
reflex; the rule above is what separates correct async code from cargo-culted
code. Try rewriting this lesson with `std::sync::Mutex` (you'll use
`.lock().unwrap()` instead of `.lock().await`) and confirm it still works.

### Lesson 10 — `blocking`: don't stall the runtime

```rust
// src/lessons/blocking.rs
use tokio::time::Instant;

fn fib(n: u64) -> u64 {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

pub async fn run() -> anyhow::Result<()> {
    let start = Instant::now();

    // CPU-heavy/blocking work must NOT run on a runtime worker thread, or it
    // freezes every other task on that thread. spawn_blocking moves it to a
    // dedicated pool meant for blocking work.
    let handle = tokio::task::spawn_blocking(|| fib(40));

    println!("runtime is still free while fib(40) computes...");
    let result = handle.await?;

    println!("fib(40) = {result} in {:?}", start.elapsed());
    Ok(())
}
```

**Why this matters:** an `async` task that runs a long CPU loop never hits an
`.await`, so it never yields — it hogs its thread and starves other tasks.
`spawn_blocking(closure)` runs the closure on Tokio's separate blocking-thread
pool and gives you a `JoinHandle` to await. Use it for heavy compute, `std::fs`,
or any blocking library.

**New Rust:** `|| fib(40)` is a closure taking no arguments. `{:?}` in the format
string uses the *debug* formatting (here for a `Duration`), versus `{}` which uses
*display* formatting; types opt into each.

### Lesson 11 — `cancel`: graceful shutdown of a task

```rust
// src/lessons/cancel.rs
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

pub async fn run() -> anyhow::Result<()> {
    let token = CancellationToken::new();
    let child = token.clone();

    let worker = tokio::spawn(async move {
        let mut n = 0;
        loop {
            tokio::select! {
                _ = child.cancelled() => {
                    println!("worker: cancellation received, cleaning up");
                    break;
                }
                _ = sleep(Duration::from_millis(50)) => {
                    n += 1;
                    println!("worker: step {n}");
                }
            }
        }
    });

    sleep(Duration::from_millis(220)).await;
    println!("main: requesting cancellation");
    token.cancel();
    let _ = worker.await;
    Ok(())
}
```

A `CancellationToken` (from `tokio-util`) is a clonable flag. The worker `select!`s
between "the token was cancelled" and "do one step of work." When `main` calls
`token.cancel()`, the worker's `cancelled()` branch wins, lets it break out of the
loop and run cleanup. This is the standard cooperative-shutdown pattern. `loop`
is an infinite loop you exit with `break`.

### Lesson 12 — `streams`: the async iterator

```rust
// src/lessons/streams.rs
use tokio_stream::StreamExt;

pub async fn run() -> anyhow::Result<()> {
    let mut stream = tokio_stream::iter(1..=5)
        .map(|n| n * n)
        .filter(|n| n % 2 == 1);

    while let Some(v) = stream.next().await {
        println!("stream yielded {v}");
    }
    Ok(())
}
```

A **`Stream`** is the async analogue of an `Iterator`: values arrive over time
and you `.next().await` for each. `tokio_stream::iter` turns a normal range into a
stream so we can demo the adapters.

**New Rust — traits.** `StreamExt` is a *trait*: a set of methods (`.map`,
`.filter`, `.next`) that a type can implement. You must `use` the trait to call
its methods — that's why the import is required even though we never name
`StreamExt` directly. `.map(|n| n * n)` transforms each item; `.filter(|n| ...)`
keeps items where the closure returns `true`. Traits are Rust's interfaces.

### Lesson 13 — `echo`: a real async TCP server

```rust
// src/lessons/echo.rs
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub async fn run(addr: String) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    println!("echo server on {addr} (connect with: nc {})", addr.replace(':', " "));

    loop {
        let (mut socket, peer) = listener.accept().await?;

        // One task per connection: the accept loop is never blocked by a client.
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                match socket.read(&mut buf).await {
                    Ok(0) => break,           // 0 bytes = peer closed
                    Ok(n) => {
                        if socket.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            println!("connection {peer} closed");
        });
    }
}
```

This ties everything together. `TcpListener::bind(&addr).await?` opens the port.
`listener.accept().await?` waits for a client and returns its socket plus address.
We `spawn` a task per connection so one slow client can't block others.

**New Rust:**

- `&addr` passes a *reference* (borrow) to `bind` instead of giving away the
  `String` — `addr` is still usable on the next line.
- `let mut buf = [0u8; 1024];` is a fixed-size **array** of 1024 bytes, all zero.
  `u8` is an 8-bit byte.
- `socket.read(&mut buf)` needs `&mut buf` — a *mutable* borrow, because it writes
  into the buffer. Rust's rule: many shared (`&`) borrows **or** one mutable
  (`&mut`) borrow, never both at once. This is what prevents data races.
- `&buf[..n]` is a *slice* of the first `n` bytes — only the data actually read.
- `match socket.read(...)` with `Ok(0)` / `Ok(n)` / `Err(_)`: handle peer-closed,
  data, and error distinctly. `read`/`write_all` come from the `AsyncReadExt` /
  `AsyncWriteExt` traits we imported.

**Tokio:** `AsyncReadExt`/`AsyncWriteExt` are the async counterparts of std's
`Read`/`Write`. Note how close the code looks to blocking I/O — that's
deliberate. Test it: run `... -- echo 127.0.0.1:9000`, then in another terminal
`nc 127.0.0.1 9000` and type; the server echoes you back.

### Lesson 14 — `signal`: shut down on Ctrl-C

```rust
// src/lessons/signal.rs
use std::time::Duration;
use tokio::time::sleep;

pub async fn run() -> anyhow::Result<()> {
    let worker = tokio::spawn(async {
        let mut n = 0;
        loop {
            sleep(Duration::from_millis(300)).await;
            n += 1;
            println!("working... {n}");
        }
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => println!("\nshutdown signal received"),
        _ = worker => {}
    }
    println!("shutting down gracefully");
    Ok(())
}
```

`tokio::signal::ctrl_c()` is a future that completes when the user presses
Ctrl-C. We `select!` it against the worker: whichever happens first wins, and we
print a shutdown message. Press Ctrl-C while it runs to see it react instead of
dying abruptly. (Note `async { ... }` here has no `move` because it captures
nothing from the surrounding scope.)

---

## Part 5 — Recap and where to go next

You now have a working tour of:

- **Tasks:** `spawn`, `JoinHandle`, `join!`, `select!`, `spawn_blocking`.
- **Channels:** `mpsc`, `oneshot`, `broadcast`, `watch`.
- **Shared state:** `Arc<Mutex<T>>` — and *when not to use the Tokio mutex*.
- **Lifecycle:** `CancellationToken`, `ctrl_c`.
- **I/O & streams:** `TcpListener`, async read/write, `Stream`.

And the Rust fundamentals underneath: ownership and `move`, references (`&` /
`&mut`) and the borrow rules, `Result`/`Option` enums, the `?` operator, structs,
enums, pattern matching, closures, and traits.

Good next steps:

1. **Trim Tokio's features.** Replace `features = ["full"]` with only what you
   use (e.g. `["rt-multi-thread", "macros", "net", "time", "sync", "signal"]`) and
   fix the compile errors — it teaches you which module provides what.
2. **Rewrite Lesson 9** with `std::sync::Mutex` and feel why it's the right
   default when no `.await` is held.
3. **Extend the echo server** into a tiny line-based chat using `broadcast`.
4. Read the official **Tokio Tutorial** (tokio.rs) and the **Rust Book**
   (doc.rust-lang.org/book) for depth on ownership and futures.

To add a lesson: create `src/lessons/<name>.rs` with `pub async fn run(...)`, add
`pub mod <name>;` to `lessons/mod.rs`, then add an enum variant and a match arm in
`main.rs`. That three-step pattern is the whole architecture.

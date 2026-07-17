# proj_grep_threads

## What you'll build

A parallel mini-grep: `cargo run -- <pattern> <dir>` walks every file in
`testdata/`, searches each line for `<pattern>`, and prints
`file:line_no: line` for every match — searched by a pool of real OS
threads, not tokio tasks. No runtime, no `.await`. `std::thread::spawn`
gives you a genuine OS thread; `std::sync::mpsc` gives you a channel with
zero async machinery underneath. The payoff: a gut feeling for what tokio
saves you from — manual thread lifetimes, `move` closures, and ownership
fights the borrow checker won't let you skip.

## Concepts you'll practice

- Threads & `move` closures — Book ch. 16.1
- Message passing with `mpsc` — Book ch. 16.2
- Shared state with `Arc<Mutex<T>>` — Book ch. 16.3
- `Send` and `Sync` — Book ch. 16.4

## Ground rules

- Hints are collapsed. Timer for 20 minutes before opening Hint 1.
- `cargo check` constantly — ownership errors here are the lesson.
- Async habits mislead you. No `.await`, no `Future`. A thread blocks or
  it doesn't.

## Milestone 1 — Single-threaded baseline

**Goal:** `cargo run -- horizon testdata` prints every matching line
across every file, sequentially, no threads. Correctness first — you
need a known-good baseline before judging the parallel version.

**Design questions**
1. How do you pull pattern and directory from `std::env::args`? Behavior
   on a missing arg?
2. What does `std::fs` give you for listing a flat directory?
3. Which API hands you line number *and* text together, or do you derive
   the number yourself?
4. Fix the exact output format now — every later milestone must match it.

**Definition of done**
```
$ cargo run -- horizon testdata
testdata/ocean.txt:1: The old sailor watched the horizon for hours, waiting for a change in the
testdata/astronomy.txt:1: Just before sunrise, the horizon glows faintly orange while the rest of
...
```

<details><summary>Hint 1 — nudge</summary>
Three nested loops: files, lines, pattern check. No concurrency yet.
</details>
<details><summary>Hint 2 — API pointers</summary>
`std::env::args`, `std::fs::read_dir`, `std::fs::read_to_string`,
`str::lines`, `Iterator::enumerate` (0- vs 1-indexing).
</details>
<details><summary>Hint 3 — shape</summary>
```
fn search_file(path: &Path, pattern: &str) -> Vec<(usize, String)> { ... }
```
</details>

## Milestone 2 — Two fixed threads, direct printing

**Goal:** Split the file list in half. Thread A searches half 1 and
prints its own matches; Thread B does half 2. `main` spawns both, joins
both.

**Design questions**
1. What must the spawned closure *own* rather than borrow, and why can't
   it borrow `&file_list`?
2. Two threads both calling `println!`, uncoordinated — what could the
   interleaving look like across runs? Bug, or inherent?
3. What breaks if you never `.join()` a `JoinHandle`?
4. Pass a *borrowed* file list into both closures without `move`, on
   purpose. Read the error before fixing it — what does the compiler
   think the closure might outlive?

**Definition of done**
```
$ cargo run -- thread testdata
[... all matches from both halves, interleaved unpredictably ...]
```
Run it several times — the *set* is identical; the *order* isn't.

<details><summary>Hint 1 — nudge</summary>
Splitting a `Vec` in half is ordinary slicing. The hard part: convincing
the compiler that data crossing into a spawned thread outlives `main`'s
current stack frame. That's what `move` is for.
</details>
<details><summary>Hint 2 — API pointers</summary>
`thread::spawn`, `JoinHandle::join`, `move`, slicing (`&v[..mid]`) plus
`.to_vec()`. `join()` returns a `Result` — what's `Err` on a panic?
</details>
<details><summary>Hint 3 — shape</summary>
```
let a = thread::spawn(move || { /* search half_a, println! matches */ });
let b = thread::spawn(move || { /* search half_b, println! matches */ });
a.join().unwrap();
b.join().unwrap();
```
</details>

## Milestone 3 — Channel refactor: one printer, program terminates

**Goal:** Same two threads, but each worker sends a `Match` through an
`mpsc::channel` instead of printing directly. `main` holds the single
`Receiver` and prints in one place. The loop must end **on its own** once
both workers finish — the process must exit, not hang.

**Design questions**
1. `mpsc` is multi-producer, single-consumer. `mpsc::channel()` gives one
   `Sender` — how do two workers each get one?
2. `main` typically iterates `rx` directly (`for msg in rx`). What
   condition stops that?
3. If `main` keeps its own original `Sender` alive after cloning it for
   both workers, does the loop still end? Why or why not?
4. What fields does `Match` need — path, line number, text? The pattern
   too?

**Definition of done**
```
$ cargo run -- channel testdata
testdata/ocean.txt:5: slow down and pick a careful line.
testdata/networking.txt:1: A network engineer thinks about a channel differently than a sailor does.
...
$ echo $?
0
```
No Ctrl-C required. If it hangs, a `Sender` never got dropped.

<details><summary>Hint 1 — nudge</summary>
A channel receiver doesn't know "workers are done" — only "no `Sender`
exists anymore." Make sure every clone drops when its worker finishes,
and no extra clone survives in `main` by accident.
</details>
<details><summary>Hint 2 — API pointers</summary>
`mpsc::channel`, `Sender::clone`, `Sender::send`, `Receiver::recv`,
iterating `rx` with a `for` loop.
</details>
<details><summary>Hint 3 — shape</summary>
```
struct Match { file: PathBuf, line_no: usize, text: String }
let (tx, rx) = mpsc::channel::<Match>();
let tx_a = tx.clone();
let tx_b = tx.clone();
// where does the original `tx` go once cloned twice?
```
</details>

## Milestone 4 — Worker pool of N threads, shared queue

**Goal:** Spawn N threads (CLI arg or hardcoded) that pull from one
shared queue of file paths until empty, sending `Match`es through the
same channel as Milestone 3.

**Design questions**
1. `mpsc::Receiver` can't be cloned — only `Sender` can. How do N workers
   share one queue of file paths? Weigh: (a) `Arc<Mutex<Vec<PathBuf>>>`
   as a locked queue, vs (b) pre-chunking into N pieces like Milestone 2,
   generalized. Which degrades better with uneven file sizes?
2. With the locked queue: what happens inside the lock, and how fast is
   it released? What breaks if a worker holds it during the (slow)
   search?
3. Does "queue empty" unambiguously mean "no more work" here?
4. Must all N result-`Sender` clones exist before you spawn threads, or
   can they be created after?

**Definition of done**
```
$ cargo run -- horizon testdata
[same complete match set as Milestone 1]
$ echo $?
0
```
Try N=1, N=4, N greater than the file count — same set, all terminate.

<details><summary>Hint 1 — nudge</summary>
Both designs in question 1 are legitimate. Pre-chunking avoids a lock but
a slow file can leave one worker grinding while others idle. A locked
queue load-balances at the cost of contention.
</details>
<details><summary>Hint 2 — API pointers</summary>
`Arc`, `Mutex`, `Mutex::lock`, `Vec::pop` for the queue; `slice::chunks`
for pre-chunking. `Arc::clone` for each thread's handle.
</details>
<details><summary>Hint 3 — shape</summary>
```
let queue: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(all_files));
// worker: loop { match queue.lock().unwrap().pop() { Some(p) => ..., None => break } }
```
</details>

## Milestone 5 — Shared match counter

**Goal:** After all workers finish, print `N matches across M files`. The
count is tracked via `Arc<Mutex<usize>>` shared across workers — not
derived by counting messages in `main` afterward.

**Design questions**
1. Increment per match found, or accumulate locally per worker and add
   once at the end? Cost in lock contention either way?
2. `pattern: String` must reach every worker. Clone per worker, wrap in
   `Arc<String>`, or borrow `&str` across the thread boundary? Try the
   borrow on purpose — what does the compiler say about the closure's
   lifetime vs. `'static`?
3. Does the counter need a `Mutex`, or does `std::sync::atomic` offer
   something built for exactly this?
4. If the counter's lock and Milestone 4's queue lock are ever held at
   the same time in different orders across threads, could that
   deadlock?

**Definition of done**
```
$ cargo run -- thread testdata
[... individual match lines ...]
5 matches across 3 files
```

<details><summary>Hint 1 — nudge</summary>
Small code, big "why": you're deciding how finely to slice a lock.
Locking per match maximizes contention; locking once per worker at the
end is cheaper and equally correct here.
</details>
<details><summary>Hint 2 — API pointers</summary>
`Arc<Mutex<usize>>`, `Mutex::lock().unwrap()`, `*guard += n`. For the
pattern: `Arc<String>` + `Arc::clone`, or plain `.clone()`.
</details>
<details><summary>Hint 3 — shape</summary>
```
let match_count: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
let pattern: Arc<String> = Arc::new(pattern);
// per worker: Arc::clone(&match_count), Arc::clone(&pattern)
```
</details>

## Milestone 6 — Errors don't kill a worker

**Goal:** Point the search at a path that can't be read (nonexistent
file, or permissions-restricted). The program keeps going: the failing
worker sends an `Error` message through the channel instead of
panicking; every other file still gets searched.

**Design questions**
1. What does `std::fs::read_to_string` return on failure? What's your
   worker currently doing with it — `.unwrap()`?
2. If `Match` and errors share a channel, does the message type become
   an enum with `Found`/`Error` variants, or an optional field on `Match`?
3. Should `main` print errors to stderr instead of stdout? Does that
   change whether the loop still terminates?
4. If a *different* worker panics outright instead of handling its
   error, what happens to that worker's `Sender` clone during unwind —
   does the receive loop still end correctly?

**Definition of done**
```
$ cargo run -- thread testdata
[... matches from every readable file ...]
error reading testdata/does_not_exist.txt: No such file or directory (os error 2)
5 matches across 3 files
$ echo $?
0
```

<details><summary>Hint 1 — nudge</summary>
A targeted refactor of Milestone 3's message type, not new architecture.
An `Err` from `fs` becomes data flowing through the channel, not a panic
unwinding the worker's stack.
</details>
<details><summary>Hint 2 — API pointers</summary>
`std::io::Result`, `std::io::Error`'s `Display` (already produces
"No such file or directory"). Replace `.unwrap()` on the read with a
`match` that sends an `Error` variant.
</details>
<details><summary>Hint 3 — shape</summary>
```
enum WorkerMsg {
    Found(Match),
    Error { path: PathBuf, message: String },
}
```
</details>

## Compiler errors you'll probably meet

- **E0373 — closure may outlive the current function.** Hits when you
  `thread::spawn` a closure capturing a local by reference (no `move`).
  The spawned thread isn't tied to `main`'s stack frame — it could still
  run after `main` moves past the spawn line — so the compiler refuses a
  borrow that might not outlive it. `move` hands over an owned value
  instead.
- **E0382 — use of moved value.** You `move` the original `Sender`
  straight into the *first* worker's closure, then try `.clone()` for the
  second worker afterward. Once moved into a closure it's gone from the
  surrounding scope — clone before you move, not after.
- **"the trait `Send` is not implemented for `Rc<...>`."** Swap your
  shared pattern or counter into `Rc<RefCell<...>>` instead of
  `Arc<Mutex<...>>` on purpose (Milestone 5) and try moving it into a
  spawned thread. `Rc`'s refcount isn't atomic — two threads bumping it
  concurrently would race — so the compiler refuses to let it cross a
  thread boundary. `Arc` uses atomic ops so this is safe.
- **Program hangs forever, no panic, nothing after the last match.** Not
  a compiler error — a runtime symptom. `main`'s receive loop is still
  waiting because a `Sender` clone is alive somewhere: often `main` kept
  the *original* `tx` after cloning it per worker, or a worker hung
  without finishing (never dropping its clone). The receiver stops only
  once the *last* `Sender` anywhere is dropped.

## Stretch goals

- Recursive directory walk using only `std::fs` (no crates) — recursion
  or an explicit stack of directories to visit.
- Line numbers plus a colored match highlight via raw ANSI escapes
  (`\x1b[31m...\x1b[0m`), no crate.
- Swap `mpsc::channel` for `mpsc::sync_channel(1)` and watch a fast
  producer block when the consumer falls behind — real backpressure.
- Benchmark 1 worker vs. N workers on a larger synthetic `testdata/` with
  `std::time::Instant`; find where the speedup plateaus on your machine.

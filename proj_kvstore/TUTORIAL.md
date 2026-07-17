# proj_kvstore

## What you'll build

A persistent key-value store driven from the command line: `set`, `get`,
`rm`, `list`. Every write appends a line to a log file (`kv.log`); reads
replay that log into a `HashMap`. No serde, no external crates — you
hand-roll the file format and the error handling. This is the exercise
where the program has to name and handle every way it can fail, not just
the happy path.

## Concepts you'll practice

- Enums as a closed set of commands — Book ch. 6
- Recoverable errors with `Result` — Book ch. 9.2
- `?` and `From` conversions between error types — Book ch. 9.2
- Traits: `Display` / `std::error::Error` — Book ch. 10
- Command-line args (`env::args`) — Book ch. 12.1
- File I/O: `OpenOptions`, `BufReader`, `BufWriter` — std docs
- `HashMap` as the in-memory index — Book ch. 8.3
- Returning `Result` from `main` — Book ch. 9.3

## Ground rules

Hints below are collapsible, weakest-to-strongest. Spend 20+ minutes
actually stuck — read the full compiler error, check the std docs for the
type it names — before opening Hint 1. Opening Hint 3 immediately turns
this into typing, not learning. Stuck even after Hint 3? Back up a
milestone, don't ask for the code.

---

## Milestone 1 — Parse the command line, no file I/O

**Goal:** Turn `["set","name","jp"]` / `["get","name"]` / `["rm","name"]` /
`["list"]` into a `Command` enum. Anything else is a usage error: message
to stderr, nonzero exit. No file reads or writes yet.

**Design questions**
- What fields does each `Command` variant carry?
- Is bad usage reported via `Result`, or an immediate exit from the parser?
- `env::args()` yields the binary name first — how do you skip it cleanly?
- What's your exit-code convention for bad input vs. success?

**Definition of done**
- `cargo run -- set name jp`, `get name`, `rm name`, `list` all parse.
- `cargo run -- bogus` and `cargo run -- set onlyonearg` print a usage
  message to stderr and exit nonzero — check with `echo $?`.

<details><summary>Hint 1 — nudge</summary>
You're translating `Vec<String>` into a strongly-typed enum. "Invalid" here
is the caller's fault, not an I/O failure — that distinction matters again
in Milestone 4.
</details>
<details><summary>Hint 2 — API pointers</summary>
`std::env::args()`, `Iterator::skip`, `.collect::<Vec<_>>()`, slice pattern
matching (`match args.as_slice() { [a, b] => ..., _ => ... }`),
`std::process::exit(i32)`, `eprintln!`.
</details>
<details><summary>Hint 3 — shape</summary>

```rust
enum Command {
    Set { key: String, value: String },
    Get { key: String },
    Rm { key: String },
    List,
}
fn parse_args(args: &[String]) -> Result<Command, String>;
```
</details>

---

## Milestone 2 — `set` appends, `list` dumps the raw file

**Goal:** `set` opens `kv.log` (create-if-missing, append) and writes one
line per call. `list` prints the raw lines — no map parsing yet.

**Design questions**
- Exact on-disk line format (given below) — where does that logic live?
- Buffered or unbuffered writes — what's the cost difference in a loop?
- Key/value containing your delimiter or a newline — guard now, or punt and
  note it as a known limitation?
- Where does the file path live — constant, flag, always cwd?

**Definition of done**
- `set name jp` then `cat kv.log` shows one line; three more `set`s add
  three more lines in order.
- `list` prints the same lines to stdout.
- Deleting `kv.log` first, `set` still works (creates it).

<details><summary>Hint 1 — nudge</summary>
Append-or-create is one flag combination, not two branches. Buffering
matters here — think about what an unbuffered write costs per call, and
what "flush" means when a `BufWriter` gets dropped.
</details>
<details><summary>Hint 2 — API pointers</summary>
`OpenOptions::new().append(true).create(true).open(path)`,
`std::io::BufWriter`, `writeln!`, `std::io::BufReader`,
`BufRead::lines()`.
</details>
<details><summary>Hint 3 — shape</summary>

Line format (tab-separated):
```
set<TAB>key<TAB>value\n
rm<TAB>key\n
```
```rust
const LOG_PATH: &str = "kv.log";
fn append_set(path: &str, key: &str, value: &str) -> std::io::Result<()>;
fn print_raw(path: &str) -> std::io::Result<()>;
```
</details>

---

## Milestone 3 — `get` replays the log into a HashMap

**Goal:** Replay the whole log top-to-bottom into `HashMap<String, String>`:
later `set`s overwrite earlier ones, `rm` lines remove the key
(tombstones). `get` looks the key up in the result.

**Design questions**
- `rm` needs to append a tombstone line now — same pattern as `append_set`?
- Replay is O(file size) per `get` — deferring the cost, or addressing it
  now? (See Stretch goals.)
- Key not found after replay — silent, message, distinct exit code? Decide
  before Milestone 4 formalizes it as `KeyNotFound`.
- A line with the wrong number of tab-separated fields — what happens?
  (Formalized in Milestone 5 — for now, don't panic silently.)

**Definition of done**
- `set a 1`, `set b 2`, `set a 3`, `get a` → `3`.
- `set c 5`, `rm c`, `get c` → reports "not found" (your wording).
- `list` still shows the raw log including the `rm` line.

<details><summary>Hint 1 — nudge</summary>
Replay is a fold: start with an empty map, walk lines in order, apply each
as a mutation. The map only ever reflects the latest state.
</details>
<details><summary>Hint 2 — API pointers</summary>
`HashMap::insert`, `HashMap::remove`, `HashMap::get`, `str::split('\t')`,
pulling fields off a split iterator with `.next()`.
</details>
<details><summary>Hint 3 — shape</summary>

```rust
use std::collections::HashMap;
fn replay(path: &str) -> std::io::Result<HashMap<String, String>>;
fn append_rm(path: &str, key: &str) -> std::io::Result<()>;
```
</details>

---

## Milestone 4 — Replace every `unwrap()` with a real error type

**Goal:** Define `KvError { Io, KeyNotFound(String), BadCommand(String) }`,
implement `From<io::Error> for KvError`, `Display`, and `Error`. Every
fallible function returns `Result<_, KvError>`; use `?` instead of
`unwrap`/`expect`; `main() -> Result<(), KvError>`.

**Design questions**
- List *every* way this program can fail — file missing, permission
  denied, disk full, bad CLI args, key not found, malformed log line. Which
  are the caller's fault vs. the environment's? Does that change the exit
  code or message?
- Why does `?` need a `From` impl to convert error types — what is `?`
  actually doing, and what trait bound does it require?
- What does the runtime print when `main` returns `Err`? Good enough, or do
  you want to catch it and print something nicer with a specific exit code?
- Should Milestone 1's usage error fold into `KvError` now that one unified
  type exists?
- Is `KeyNotFound` a hard error, or an expected outcome `get` handles
  gracefully — how does `grep` or `git config` treat a missing match?

**Definition of done**
- `grep -rn "unwrap\|expect(" src/` finds nothing in command-handling code.
- `get missing-key` prints a clear message, exits nonzero (`echo $?`).
- `get name` (exists) exits 0, prints just the value.
- `bogus` still exits nonzero with a friendly message.
- `chmod 000 kv.log`, then any command touching it — readable I/O error,
  no panic/backtrace.

<details><summary>Hint 1 — nudge</summary>
An error enum names everything that can go wrong. `?` means "if `Err`,
convert to my function's error type and return early" — the conversion is
what `From` provides.
</details>
<details><summary>Hint 2 — API pointers</summary>
`impl Display for KvError`, `impl std::error::Error for KvError` (default
methods suffice), `impl From<io::Error> for KvError`,
`std::process::ExitCode` as an alternative to `process::exit`.
</details>
<details><summary>Hint 3 — shape</summary>

```rust
#[derive(Debug)]
enum KvError {
    Io(std::io::Error),
    KeyNotFound(String),
    BadCommand(String),
}
impl std::fmt::Display for KvError { /* ... */ }
impl std::error::Error for KvError {}
impl From<std::io::Error> for KvError { /* ... */ }
fn main() -> Result<(), KvError>;
```
</details>

---

## Milestone 5 — Detect and report corrupt log lines

**Goal:** A hand-edited or truncated `kv.log` must not panic replay. Add
`CorruptRecord { line: usize }` to `KvError`; report the 1-indexed line
number of the first line that fails to parse.

**Design questions**
- What counts as corrupt — wrong field count, unrecognized command word,
  both?
- Stop at the first corrupt line, or collect all and report a list?
- `lines()` is 0-indexed — how do you get 1-indexed line numbers?
- Should `list` (no parsing) still show a corrupt line raw while `get`
  refuses to proceed?

**Definition of done**
- `echo "garbage" >> kv.log`.
- `get anykey` reports something like `corrupt record at line 5`, exits
  nonzero, no panic.
- `list` still shows the garbage line raw.
- Removing the garbage line restores normal `get`.

<details><summary>Hint 1 — nudge</summary>
This is validation added to the Milestone 3 parsing step — give the
"wrong shape" case an explicit `Err` instead of assuming success.
</details>
<details><summary>Hint 2 — API pointers</summary>
`Iterator::enumerate()` to pair line index with content; `match` on the
split fields' slice to catch missing-field cases explicitly.
</details>
<details><summary>Hint 3 — shape</summary>

```rust
enum KvError {
    Io(std::io::Error),
    KeyNotFound(String),
    BadCommand(String),
    CorruptRecord { line: usize },
}
fn replay(path: &str) -> Result<HashMap<String, String>, KvError>;
```
</details>

---

## Milestone 6 (stretch-ish) — Compaction

**Goal:** `compact` rewrites `kv.log` to hold only live key/value pairs,
written atomically (temp file + rename) so a crash mid-compaction never
corrupts data.

**Design questions**
- Why does rename-over give atomicity where a direct overwrite wouldn't?
- Temp file same directory as `kv.log` — does that matter for the rename
  guarantee?
- Interrupted after the temp write but before rename — what state is
  `kv.log` left in? Acceptable?
- Auto-compact after N writes, or only on explicit command?

**Definition of done**
- Build up dead records with several `set`/`rm` calls, note file size.
- `compact` — log shrinks to live keys only.
- `get <live-key>` still correct after compaction.
- `list` after compaction shows one line per live key, no history.

<details><summary>Hint 1 — nudge</summary>
`replay` already produces the final map — compaction serializes that map
back out as fresh `set` lines, landed via temp file + rename instead of a
direct overwrite.
</details>
<details><summary>Hint 2 — API pointers</summary>
`std::fs::rename`, writing the `.tmp` file in the same directory as the
target (rename across filesystems isn't atomic — why `env::temp_dir()` is
probably the wrong choice here).
</details>
<details><summary>Hint 3 — shape</summary>

```rust
fn compact(path: &str) -> Result<(), KvError>;
```
</details>

---

## Compiler errors you'll probably meet

- **E0277 — `?` couldn't convert the error type**: used `?` on a
  `Result<_, io::Error>` inside a function returning `Result<_, KvError>`
  with no `From<io::Error> for KvError` yet.
- **E0308 — mismatched types**: `expected Result<T, KvError>, found
  Result<T, io::Error>` — same root cause, different call site, usually
  from a signature you forgot to update in Milestone 4.
- **E0507 — cannot move out of borrowed content**: pulling a `String` out
  of a `HashMap` via `.get()` (returns `Option<&String>`, not
  `Option<String>`) and trying to return/store it by value.
- **"value moved here, value used here after move"**: passed a `String`
  by value into a function, then tried to use it again. Decide if the
  function needs ownership or just `&str`, and whether `.clone()` is
  actually warranted.
- **"cannot borrow `map` as mutable because it is also borrowed as
  immutable"**: holding a reference from `map.get(key)` while also calling
  `map.insert`/`.remove` in the same scope.

## Stretch goals

- **Log compaction** (Milestone 6, if skipped) — temp file + rename.
- **In-memory offset index** — `HashMap<String, u64>` of byte offsets,
  updated incrementally on write, seek directly on `get` instead of full
  replay. Compare startup cost vs. per-`get` cost against plain replay.
- **Generic over value type** — replace `HashMap<String, String>` with a
  small trait for round-tripping any type to one line of text; think
  through what happens if a value contains your delimiter.
- **Tempfile-based tests** — integration tests using a unique log path per
  test (env var or `--data-file` flag) asserting `set`/`get`/`rm`/`compact`
  behavior end-to-end.
- **Concurrent access** — what happens if two processes `set` against the
  same log at once? Investigate OS-level advisory file locking even without
  implementing it.

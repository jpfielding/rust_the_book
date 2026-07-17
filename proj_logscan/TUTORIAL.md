# proj_logscan — a zero-copy log analyzer

## What you'll build

A CLI that reads a web-server log file (`sample.log` is provided) and reports
stats on it: line counts, hits per path, status code breakdown, and a top-N
report of the busiest paths. The catch: you're not allowed to allocate a
`String` for every field of every log line. You'll define a struct that
*borrows* slices out of each line instead of copying them, and you'll feel
exactly where the borrow checker draws the line between "this works" and
"this doesn't compile" — that boundary is the whole point of the exercise.

## Concepts you'll practice

- Lifetimes and borrowed struct fields — Book ch. 10.3
- Ownership and the stack/heap distinction (`String` vs `&str`) — Book ch. 4.1
- References and borrowing — Book ch. 4.2
- Reading files and iterating lines — Book ch. 12 (I/O project)
- `Option` / `Result` and `filter_map` for graceful error handling — Book ch. 9, ch. 6
- `HashMap` and the `entry` API — Book ch. 8.3
- Sorting with closures (`sort_by`, `sort_by_key`) — Book ch. 13 (iterators), std docs
- Organizing code into library modules and writing unit tests — Book ch. 7, ch. 11

## Ground rules

- All hints below are collapsed (`<details>` blocks). Do not open Hint 2 until
  you've actually tried and failed with Hint 1's direction. Do not open Hint 3
  unless Hint 2 genuinely didn't get you unstuck.
- Spend at least 20 minutes stuck on a milestone before opening anything.
  Being stuck is where the learning happens — the compiler is trying to teach
  you something every time it rejects your code. Read its message in full.
- Nobody is grading elegance on the first pass. Get it green, then clean it up.

---

## Milestone 1 — read the file, count the lines

**Goal:** `cargo run -- sample.log` prints the number of lines in the file.

**Design questions**
- What does `std::fs::read_to_string` give you ownership of, and how long
  does that value need to live relative to your program?
- Should the file path be hardcoded or taken from `std::env::args()`? What
  happens if the user gives no argument, or a bad one?
- What's the difference between counting bytes, counting `\n` characters, and
  counting the items yielded by `.lines()`?
- What should happen on a missing file — panic, or a handled error printed
  to the user?

**Definition of done**
```
cargo run -- sample.log
# -> something like: "42 lines"
cargo run -- does_not_exist.log
# -> a clean error message, not a panic backtrace
```

<details><summary>Hint 1 — nudge</summary>

You need to get the whole file into memory as text, then ask something in
`std` to split it into lines for you. Think about who owns that text buffer
and how long it needs to stick around while you're iterating.

</details>

<details><summary>Hint 2 — API pointers</summary>

Look at `std::fs::read_to_string`, `str::lines`, `std::env::args`,
`Iterator::count`. For error handling, look at the `Result` returned by
`read_to_string` and how `match` or `if let Err(...)` lets you print and
exit instead of unwrap-panicking.

</details>

<details><summary>Hint 3 — shape</summary>

```
fn main() {
    // args: Vec<String> or similar
    // contents: String
    // line_count: usize
}
```

No struct needed yet. Just get comfortable with `String` ownership and
`.lines()` before you touch lifetimes.

</details>

---

## Milestone 2 — parse one line into a borrowing `LogEntry<'a>`

This is the milestone. Everything before it was warm-up.

**Goal:** define `struct LogEntry<'a>` with fields like `ip: &'a str`,
`method: &'a str`, `path: &'a str`, `status: u16`, `bytes: u64` — and a
function that parses one log line `&str` into a `LogEntry<'a>`. Print the
parsed fields for the first few lines of `sample.log`.

**Design questions**
- Should `LogEntry` own `String`s or borrow `&str`? What changes about *who*
  must stay alive, and for how long, once you pick one?
- If your parsing function is `fn parse(line: &str) -> LogEntry` — where do
  the lifetimes in that signature actually come from, and does the compiler
  let you leave them implicit?
- `status` and `bytes` are numbers, not text — do those fields need a
  lifetime parameter at all? Why would mixing borrowed and owned/copy fields
  in the same struct still require you to declare `'a` on the struct?
- What happens if you read lines with a reused `String` buffer (say, via
  `BufRead::read_line` in a loop that clears and refills the same buffer) and
  try to stash the resulting `LogEntry`s into a `Vec` for later? Walk through,
  on paper, why the compiler would reject that — before you write it and let
  the compiler tell you.
- Given the answer to the question above, what's different about calling
  `.lines()` on a `String` you read once and kept alive for the whole
  program, versus the reused-buffer approach?

**Definition of done**
```
cargo run -- sample.log
# -> prints something like:
# LogEntry { ip: "203.0.113.42", method: "GET", path: "/index.html", status: 200, bytes: 2326 }
# ...
```
It should compile with a real `'a` lifetime parameter on `LogEntry`, not
`&'static str` and not owned `String` fields as a workaround.

<details><summary>Hint 1 — nudge</summary>

A log line has a fixed rough shape: an IP, some dashes, a bracketed
timestamp, a quoted `"METHOD path HTTP/x.x"`, a status code, and a byte
count. You don't need a general parser — you need to split the line on
spaces and quotes and grab the pieces you care about. Every piece you grab
should be a *slice* of the original line, not a new allocation.

</details>

<details><summary>Hint 2 — API pointers</summary>

Look at `str::split_whitespace`, `str::split`, `str::splitn`,
`str::trim_matches` (for stripping quotes/brackets), and `str::parse::<u16>()`
/ `str::parse::<u64>()` for the numeric fields. For the struct itself, look at
how a generic lifetime parameter is declared on a struct (`struct Foo<'a> { field: &'a str }`)
and how it's threaded through an `impl` block and a function signature.

</details>

<details><summary>Hint 3 — shape</summary>

```
struct LogEntry<'a> {
    ip: &'a str,
    method: &'a str,
    path: &'a str,
    status: u16,
    bytes: u64,
}

fn parse_line(line: &str) -> Option<LogEntry<'_>> {
    // ...
}
```

Note the return type is `Option`, not a bare `LogEntry` — you'll need that
for Milestone 5, and it costs you nothing to plan for it now.

</details>

---

## Milestone 3 — aggregate stats with `HashMap`

**Goal:** after parsing every line, print hit counts per path and a
breakdown of counts per status code.

**Design questions**
- What's the key type and value type for "hits per path"? Does the key need
  to own its data, or can it borrow from the line buffer — and does your
  answer change depending on how long the underlying `String` lives?
- `HashMap::entry` combined with `or_insert` or `or_insert_with` avoids a
  double lookup. What's the double lookup you'd otherwise write, and why is
  it wasteful?
- Should the status-code map use `u16` as the key, or a `String`/`&str`?
  Which is more useful for the person reading your report?
- Do you need one pass over the entries per map, or can you build both maps
  in a single pass?

**Definition of done**
```
cargo run -- sample.log
# -> ...previous output...
# -> Status breakdown: 200: 27, 404: 3, 500: 3, 401: 1
# -> Hits per path: /index.html: 4, /api/users: 3, ...
```

<details><summary>Hint 1 — nudge</summary>

You're building a frequency table. For each entry, look up its key in a map;
if it's the first time you've seen that key, start a counter at zero; either
way, add one. That "look up or insert a default" motion has a dedicated API
so you don't write it by hand with `if map.contains_key(...)`.

</details>

<details><summary>Hint 2 — API pointers</summary>

`HashMap::entry`, `Entry::or_insert`, then use the `+= 1` operator on what
`or_insert` gives back. Think about `HashMap<&str, u32>` vs
`HashMap<String, u32>` for the path map, and what borrowing that key implies
about the map's own lifetime relative to the source text.

</details>

<details><summary>Hint 3 — shape</summary>

```
use std::collections::HashMap;

fn hits_per_path<'a>(entries: &[LogEntry<'a>]) -> HashMap<&'a str, u32> {
    // ...
}

fn status_breakdown(entries: &[LogEntry<'_>]) -> HashMap<u16, u32> {
    // ...
}
```

</details>

---

## Milestone 4 — top-N paths, sorted

**Goal:** print the top 5 most-hit paths, sorted descending by hit count.

**Design questions**
- A `HashMap` has no order. What intermediate collection do you turn it into
  before you can sort it?
- `sort_by` takes a comparator over two items; `sort_by_key` takes a function
  producing a sortable key from one item. Which fits "sort tuples of
  `(path, count)` by `count` descending" better, and why does descending
  order need a small twist either way?
- If two paths tie on count, does your output order become nondeterministic?
  Does that matter for this exercise?
- Should "top N" be a hardcoded `5` or a CLI argument? What's the honest
  answer for how much extra flexibility this project actually needs?

**Definition of done**
```
cargo run -- sample.log
# -> ...previous output...
# -> Top 5 paths:
#      1. /index.html (4 hits)
#      2. /api/users (3 hits)
#      ...
```

<details><summary>Hint 1 — nudge</summary>

You already have counts in a map. Dump them into a `Vec` of pairs, sort that
vector, then take a prefix of it.

</details>

<details><summary>Hint 2 — API pointers</summary>

`HashMap::into_iter` or `.iter()`, `Vec::from_iter`/`.collect()`,
`Vec::sort_by` or `Vec::sort_by_key`, `std::cmp::Reverse` (a neat trick for
turning ascending sorts into descending ones), `Iterator::take`.

</details>

<details><summary>Hint 3 — shape</summary>

```
fn top_n_paths<'a>(hits: &HashMap<&'a str, u32>, n: usize) -> Vec<(&'a str, u32)> {
    // ...
}
```

</details>

---

## Milestone 5 — handle malformed lines gracefully

`sample.log` has a few lines that aren't well-formed log entries (truncated,
garbage text, a blank line). Right now your parser probably panics or
produces nonsense on those.

**Goal:** malformed lines are skipped, and the program reports how many were
skipped alongside the real stats.

**Design questions**
- Where should "this line is malformed" surface — as a `panic!`, a
  `Result::Err`, or a `None`? What does each choice cost the caller?
- `filter_map` exists specifically to turn "parse each item, drop the ones
  that don't parse" into a one-liner. What would the equivalent look like
  without it?
- Do you need to know *why* a line failed to parse, or is a plain "it
  failed" (`Option`) enough for this project's goals?
- How do you count both the successes and the failures if `filter_map` only
  gives you the successes?

**Definition of done**
```
cargo run -- sample.log
# -> Parsed 38 entries, skipped 4 malformed lines
# -> ...rest of the report, unaffected by the bad lines...
```

<details><summary>Hint 1 — nudge</summary>

Your Milestone 2 parser should already return `Option<LogEntry<'_>>` if you
took the Hint 3 shape. Now use that `None` case on purpose instead of
`.unwrap()`-ing past it.

</details>

<details><summary>Hint 2 — API pointers</summary>

`Iterator::filter_map`, or `Iterator::partition` if you want both the kept
and dropped counts from a single expression. `Iterator::filter` +
`Iterator::map` also works but makes you handle the `Option` twice.

</details>

<details><summary>Hint 3 — shape</summary>

```
fn parse_all<'a>(text: &'a str) -> (Vec<LogEntry<'a>>, usize /* skipped */) {
    // ...
}
```

</details>

---

## Milestone 6 — move parsing into a library module with unit tests

**Goal:** `LogEntry` and the parsing function live in `src/lib.rs` (or a
`src/parser.rs` module declared from `lib.rs`), `main.rs` only calls into it,
and you have unit tests covering at least: a well-formed line, a malformed
line, and one edge case of your choosing (empty line, weird status code,
whatever you think is worth guarding).

**Design questions**
- What's the minimum public surface `lib.rs` needs to expose for `main.rs`
  to still work — the struct, the parse function, the aggregation functions,
  all of it?
- Tests need their own log lines as string literals. Do those literals need
  a lifetime annotation anywhere in your test code? Why or why not, given
  where `&'static str` comes from?
- Where do `#[cfg(test)] mod tests` and `#[test]` fit relative to the code
  they test — same file, different file?
- If you have both `src/main.rs` and `src/lib.rs` in one crate, how does
  Cargo decide what's a binary vs a library, and how does `main.rs` refer to
  the library's items?

**Definition of done**
```
cargo test
# -> running N tests
# -> test result: ok. N passed; 0 failed
cargo run -- sample.log
# -> same report as before, now driven by lib code
```

<details><summary>Hint 1 — nudge</summary>

This milestone is almost entirely mechanical relocation, with one real
question buried in it: string literals in your test functions are
`&'static str`, which satisfies any `&'a str` requirement, so you won't
fight lifetimes here — you'll just be reorganizing files and adding
`#[test]` functions that call your existing logic and assert on its output.

</details>

<details><summary>Hint 2 — API pointers</summary>

`#[cfg(test)] mod tests { use super::*; ... }`, the `#[test]` attribute,
`assert_eq!`, `assert!`. For crate layout, look at how a package can define
both a `[[bin]]` target and a library target from `src/main.rs` +
`src/lib.rs` without any special `Cargo.toml` entries (Cargo infers both from
file presence).

</details>

<details><summary>Hint 3 — shape</summary>

```
// src/lib.rs
pub struct LogEntry<'a> { /* ... */ }
pub fn parse_line(line: &str) -> Option<LogEntry<'_>> { /* ... */ }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_well_formed_line() { /* ... */ }

    #[test]
    fn rejects_malformed_line() { /* ... */ }
}
```

</details>

---

## Compiler errors you'll probably meet

- **E0106 — missing lifetime specifier.** Any time you write `struct
  LogEntry { ip: &str, ... }` without `<'a>`, the compiler can't tell how
  long that reference is valid for relative to the struct itself. A struct
  holding a reference is only as long-lived as the thing it points into, and
  Rust needs that relationship spelled out, not inferred, because it's part
  of the struct's public contract.
- **E0716 — temporary value dropped while borrowed.** This shows up if you
  try something like `parse_line(&read_to_string(path).unwrap())` inline —
  the `String` from `read_to_string` is a temporary with no name, it gets
  dropped at the end of the statement, and your `LogEntry` borrowing from it
  would be a reference to freed memory. The fix is conceptual (bind the
  `String` to a variable that outlives the entries), not something to look
  up here.
- **E0502 — cannot borrow as mutable because it is also borrowed as
  immutable.** Likely if you try to mutate a `HashMap` (e.g. via `entry`)
  while also holding an immutable reference into it, or if you try to push
  into a `Vec<LogEntry<'a>>` while `'a` is tied to a buffer you're
  simultaneously trying to refill and read from — the reused-buffer scenario
  from Milestone 2's design questions.
- **E0499 / E0382 — mutable borrow issues / use of moved value.** Can appear
  if you try to reuse a `String` buffer across loop iterations (à la
  `BufRead::read_line`) while entries from earlier iterations still borrow
  from it — the borrow checker is refusing to let you overwrite memory that
  something else still considers readable.
- **"borrowed value does not live long enough" (E0597).** The general form
  of the above: some `LogEntry<'a>` outlives the `'a` it was tied to. This is
  the error that's actually teaching you the lesson of the whole project —
  read the "note: ... dropped here while still borrowed" line carefully, it
  names the exact value and the exact line where its lifetime ends.

## Stretch goals

- Add a `--top N` CLI flag instead of hardcoding 5.
- Track bytes served per path (total and average), not just hit counts.
- Parse the timestamp bracket and report hits-per-minute or busiest minute.
- Support both Apache-combined and a simpler custom format, dispatching on
  which one a line matches, using an enum instead of guessing.
- Replace the `Vec<LogEntry<'a>>` with an iterator-based pipeline that never
  materializes the full vector, and measure whether it actually changes
  anything observable for a file this small.
- Add a `--status 404` filter flag that only reports entries matching a given
  status code.

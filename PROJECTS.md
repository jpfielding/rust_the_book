# Project Catalog

This workspace has three kinds of crates:

- **`rb_*`** — exercises following [The Rust Book](https://doc.rust-lang.org/book/) chapter by chapter
- **`x_*`** — freestanding experiments (type system, STL/temporal logic, signals, tokio/axum concurrency)
- **`proj_*`** — practice projects with tutorials and *no solution code* (see [Practice Projects](#practice-projects) below)

---

## The Book (`rb_*`)

| Crate | What it does | Concepts | Book ch. |
|-------|--------------|----------|----------|
| `rb_1_2_hello_world` | prints hello world | `println!` | 1.2 |
| `rb_2_0_guessing_game` | number-guessing game vs random secret | loops, `Ordering` match, `Result`, stdin parsing, `expect` | 2 |
| `rb_3_1_variables` | mutability & shadowing demos | mutability, shadowing (incl. type-changing), scoped blocks, arithmetic | 3.1 |
| `rb_3_2_data_types` | tuples, arrays, enum iteration | tuple destructuring, fixed arrays, derive macros, closures (strum) | 3.2 |
| `rb_3_3_functions` | functions & expression blocks | expression-valued blocks, `match` on `Result`, `unwrap_or`, `if let`, `map`/`sum` | 3.3 |
| `rb_3_5_control_flow` | if/loop/while/for variations | `if` as expression, `loop` break-with-value, labeled breaks, `.rev()` ranges | 3.5 |
| `rb_4_1_ownership` | copy vs move vs clone walkthrough | `Copy`, move semantics, scope-based drop, ownership through fn calls, `Box<T>` | 4.1 |
| `rb_4_2_refs_borrow` | immutable & mutable borrows of a `String` | `&T` vs `&mut T`, borrowing rules | 4.2 |

## Experiments (`x_*`)

### Type system & pattern matching

| Crate | What it does | Concepts | Deps |
|-------|--------------|----------|------|
| `x_card_payment` | models cash/check/card payments | enums with data, tuple structs, type aliases, custom traits, `match` on enum refs | std only |
| `x_top_5_traits` | User/Role struct exercising core derives | `derive(Debug, Clone, Default, PartialEq)`, `Arc<T>`, feature flags / `cfg_attr`, serde round-trip, `Send+Sync` bounds | serde (optional) |
| `x_trie` | character trie with insert/lookup | `HashMap::entry().or_default()`, owned recursive tree structs, `&mut` traversal | std only |
| `x_match_examples` | CLI showcasing every `match` form | literals, or-patterns, guards, ranges, tuple destructuring with `..`, `@` bindings | clap |

### Signal Temporal Logic (STL)

| Crate | What it does | Concepts | Deps |
|-------|--------------|----------|------|
| `x_stl_demo` | STL robustness evaluator over timestamped traces | recursive enum AST + `Box`, fold (min/max), `let-else`, `partition_point` | chrono |
| `x_stl_ast_demo` | same evaluator, lighter design | newtype + operator overloading (`impl Add`), newtypes vs type aliases, `impl Iterator` returns, lazy evaluation | std only |
| `x_stl_horizon_dichotomy` | past/future window operators as one fold; decidability wavefront | `Option`-as-decidability with `?`, recursive horizon analysis, match guards | std only |
| `x_stl_rnn` | LSTM trained to approximate an STL monitor | generics with backend trait bounds, tensor ops, training loop, custom PRNG | burn |

### Unix signals & FFI

| Crate | What it does | Concepts | Deps |
|-------|--------------|----------|------|
| `x_sig_tokio` | waits for SIGINT, then counts | `tokio::signal::unix` streams, `Box<dyn Error>` | tokio |
| `x_sig_nix` | blocks SIGINT for 5s via raw libc | `unsafe`, FFI (`sigprocmask` et al.), `mem::zeroed` | libc |

### Threads, tokio & CSP

| Crate | What it does | Concepts | Deps |
|-------|--------------|----------|------|
| `x_threads_ownership_tokio` | OS threads vs async tasks side by side | `thread::spawn`, move closures, `Arc<Mutex<T>>`, `spawn_blocking`, mpsc drop-to-close | tokio |
| `x_tokio_cli_tutorial` | multi-lesson concurrency tutorial CLI | `select!` (biased/else), mpsc/oneshot/broadcast/watch, `CancellationToken`, streams, TCP echo, ctrl_c | clap, tokio-stream, tokio-util |
| `x_tokio_http_get` | fetches 3 URLs concurrently | `tokio::join!`, runtime `worker_threads` config | reqwest |
| `x_csp_tokio` | four CSP demos: pipeline, fan-out/fan-in, actor, shutdown | bounded mpsc backpressure, `Arc<Mutex<Receiver>>` work queue, oneshot request/reply, watch shutdown | tokio |
| `x_tokio_canc` | **stub only** — cancellation demo never written | (finish me: `CancellationToken`, `select!`, cooperative cancellation) | std only |

### Web servers

| Crate | What it does | Concepts | Deps |
|-------|--------------|----------|------|
| `x_web_server` | sync thread-per-connection static file server | `std::net::TcpListener`, hand-rolled HTTP parsing, `TryFrom`, tracing | anyhow, tracing |
| `x_web_server_async` | async port of the above | `tokio::net`, task-per-connection, `AsyncRead`/`BufStream`, `tokio::fs` | tokio, tracing |
| `x_tokio_axum_cli_chans` | axum service: healthz/readyz + graceful shutdown | `select!` on SIGINT/SIGTERM, `with_graceful_shutdown`, clap | axum, clap |
| `x_axum_tokio_app_chans` | axum ping/pong queue over bounded mpsc | `State`/`Json` extractors, `Arc<Mutex<Receiver>>`, sleep-based timeout, serde | axum, serde |

---

## Practice Projects

Seven projects (2026-07), each with its own `TUTORIAL.md`. No solution code anywhere —
each tutorial gives milestones, design questions, and *collapsible* hints that escalate:

1. **Hint 1 — nudge**: conceptual direction only
2. **Hint 2 — API pointers**: which std types/methods to look up
3. **Hint 3 — shape**: type signatures / struct definitions only, never function bodies

Rule of thumb: fight each milestone for at least 20 minutes before opening a hint.

These target the gaps between the tables above: lifetimes on structs, error-type design,
smart pointers (`Rc`/`RefCell`/`Weak`), hand-rolled iterators, and *raw* `std::thread`
concurrency (everything above reaches for tokio).

| # | Project | Core concepts | Book ch. |
|---|---------|---------------|----------|
| 1 | [proj_expr_calc](proj_expr_calc/TUTORIAL.md) | enums with data, `Box` + recursive types, exhaustive `match`, `Result` | 6, 8, 15.1 |
| 2 | [proj_kvstore](proj_kvstore/TUTORIAL.md) | custom error enums, `From` + `?`, `Display`, file I/O | 9, 10 |
| 3 | [proj_logscan](proj_logscan/TUTORIAL.md) | lifetimes on structs, `&str` slices vs `String`, iterator chains | 10.3, 13 |
| 4 | [proj_iterkit](proj_iterkit/TUTORIAL.md) | `Iterator` trait, associated types, generic adapters, closure bounds | 13, 10.1 |
| 5 | [proj_orgtree](proj_orgtree/TUTORIAL.md) | `Rc`, `RefCell`, `Weak`, cycles/leaks, runtime borrow panics | 15.4–15.6 |
| 6 | [proj_grep_threads](proj_grep_threads/TUTORIAL.md) | `std::thread`, `mpsc`, `Arc`/`Mutex`, `Send`/`Sync` — *no tokio* | 16 |
| 7 | [proj_skill_server](proj_skill_server/TUTORIAL.md) | **capstone**: axum service running Claude-style skills as managed jobs — `tokio::process`, worker queue, `CancellationToken`, SSE streaming, graceful drain | 16, 17 |

1–2 are the gentlest on-ramps; 3–4 sharpen the type system; 5–6 are where the borrow
checker and the runtime both push back hard. 7 composes your existing axum/tokio
crates (`x_axum_tokio_app_chans`, `x_csp_tokio`, `x_tokio_cli_tutorial`) into one
real architecture — its tutorial hands you the exact API contract up front and
hides only the implementation.

### Workflow per project

```sh
cd proj_<name>
$EDITOR TUTORIAL.md        # read milestone 1 only
cargo run -p proj_<name>   # (or cargo test -p proj_iterkit — that one is test-driven)
```

Projects 1–6 are std-only on purpose — the point is the language, not the ecosystem.
The capstone (`proj_skill_server`) breaks that rule deliberately: axum + tokio + serde,
because its point *is* composing the ecosystem.

---

## Coverage map

Where each Book chapter gets exercised, and what's still untouched:

| Book ch. | Topic | Covered by |
|----------|-------|------------|
| 1–4 | basics, ownership, borrowing | `rb_*` |
| 5 | structs & methods | incidentally everywhere; no dedicated crate |
| 6, 18 | enums & patterns | `x_card_payment`, `x_match_examples`, `proj_expr_calc` |
| 7 | modules & crates | `proj_logscan` milestone 6 (lib + bin split) |
| 8 | collections | `x_trie`, `proj_logscan`, `proj_kvstore` |
| 9 | error handling | `proj_kvstore` (the deep dive), `proj_expr_calc` |
| 10 | generics, traits, lifetimes | `x_top_5_traits`, `proj_iterkit`, `proj_logscan` (lifetimes) |
| 11 | testing | `proj_iterkit` (test-driven throughout) |
| 13 | closures & iterators | `x_stl_ast_demo`, `proj_iterkit` |
| 15 | smart pointers | `proj_orgtree` (`Rc`/`RefCell`/`Weak`), `x_stl_*` (`Box`) |
| 16 | fearless concurrency (threads) | `proj_grep_threads`; async variants in `x_tokio_*`/`x_csp_tokio` |
| 17 | async/await | `x_tokio_cli_tutorial`, `x_csp_tokio`, `x_axum_*`, `x_web_server_async`, `proj_skill_server` |
| 19 | unsafe, advanced traits | `x_sig_nix` (unsafe/FFI) — otherwise open territory |
| 14, 12, 20 | cargo/workspaces, an I/O project, building for production | this workspace itself; `x_web_server*` |

Untouched so far: declarative/proc macros (ch. 19.5), trait objects in depth
(`dyn Trait` beyond `Box<dyn Error>`), `no_std` — good candidates for the next
round of `proj_*` crates. `x_tokio_canc` is still an empty stub if you want a
small cancellation kata.

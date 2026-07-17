# Practice Projects

Six projects, each a workspace member with its own `TUTORIAL.md`. No solution code anywhere —
each tutorial gives milestones, design questions, and *collapsible* hints that escalate:

1. **Hint 1 — nudge**: conceptual direction only
2. **Hint 2 — API pointers**: which std types/methods to look up
3. **Hint 3 — shape**: type signatures / struct definitions only, never function bodies

Rule of thumb: fight each milestone for at least 20 minutes before opening a hint.

## Suggested order

These target the gaps between the Book chapters you've done (1–4) and the async work
you've already explored (`x_tokio_*`, `x_axum_*`, `x_csp_tokio`):

| # | Project | Core concepts | Book ch. |
|---|---------|---------------|----------|
| 1 | [proj_expr_calc](proj_expr_calc/TUTORIAL.md) | enums with data, `Box` + recursive types, exhaustive `match`, `Result` | 6, 8, 15.1 |
| 2 | [proj_kvstore](proj_kvstore/TUTORIAL.md) | custom error enums, `From` + `?`, `Display`, file I/O | 9, 10 |
| 3 | [proj_logscan](proj_logscan/TUTORIAL.md) | lifetimes on structs, `&str` slices vs `String`, iterator chains | 10.3, 13 |
| 4 | [proj_iterkit](proj_iterkit/TUTORIAL.md) | `Iterator` trait, associated types, generic adapters, closure bounds | 13, 10.1 |
| 5 | [proj_orgtree](proj_orgtree/TUTORIAL.md) | `Rc`, `RefCell`, `Weak`, cycles/leaks, runtime borrow panics | 15.4–15.6 |
| 6 | [proj_grep_threads](proj_grep_threads/TUTORIAL.md) | `std::thread`, `mpsc`, `Arc`/`Mutex`, `Send`/`Sync` — *no tokio* | 16 |

1–2 are the gentlest on-ramps; 3–4 sharpen the type system; 5–6 are where the borrow
checker and the runtime both push back hard.

## Workflow per project

```sh
cd proj_<name>
$EDITOR TUTORIAL.md        # read milestone 1 only
cargo run -p proj_<name>   # (or cargo test -p proj_iterkit — that one is test-driven)
```

All crates are std-only on purpose — the point is the language, not the ecosystem.

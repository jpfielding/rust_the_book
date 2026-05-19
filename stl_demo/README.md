# `stl_demo` — Learn Rust by Building a Signal Temporal Logic Library

A guided follow-along for `../../agentstate.rs/docs/stl-tutorial.md`. The
upstream tutorial teaches *what STL is*; this README teaches *the Rust* you
need to type the upstream tutorial in by hand. Read the source tutorial open
in one pane, this README in another, and `stl_demo/src/` in a third.

> **How to use this.** Each section maps 1:1 to a section in the source
> tutorial. Type the code from the source into `src/` as you go. The notes
> below are the language-level "why" — what Rust feature each line is
> exercising, what would go wrong without it, and what to read in *The Rust
> Programming Language* (TRPL) for more.

---

## 0. Setup

The Cargo manifest starts dependency-less. You'll add `chrono` in §2 — leave
it out until you actually need it, so you feel the moment a missing crate
forces you to edit `Cargo.toml`.

```bash
cd stl_demo
cargo check        # baseline: should compile the stub main.rs
```

The crate is part of the workspace root (`../Cargo.toml`), so `cargo` commands
also work from the repo root with `-p stl_demo`.

**Rust to lock in before you start.**

- [TRPL ch. 5](https://doc.rust-lang.org/book/ch05-00-structs.html) — structs
- [TRPL ch. 6](https://doc.rust-lang.org/book/ch06-00-enums.html) — enums and `match`
- [TRPL ch. 10](https://doc.rust-lang.org/book/ch10-00-generics.html) — generics, traits, lifetimes
- [TRPL ch. 13](https://doc.rust-lang.org/book/ch13-00-functional-features.html) — closures and iterators
- [TRPL ch. 17](https://doc.rust-lang.org/book/ch17-00-oop.html) — trait objects (`dyn Trait`)

The biggest Rust-shaped surprises in this tutorial are **trait objects**
(§4), **borrow-checker on `&mut self`** (§7), and **lifetime annotations on
structs** (§8.3). If those three feel foreign, skim those chapters first.

---

## 1. Why STL? — (no code)

Pure prose. Skip ahead.

---

## 2. Signals and Traces

You're typing in `Sample`, `Signal(pub Vec<Sample>)`, and `Trace(pub
HashMap<String, Signal>)`. This is mostly an exercise in *Rust's data
modelling vocabulary*.

### The `#[derive(...)]` line on `Sample`

```rust
#[derive(Debug, Clone, Copy)]
pub struct Sample { pub t: DateTime<Utc>, pub v: f64 }
```

`derive` is a macro that auto-generates trait impls. You're asking the
compiler to write four free functions for you:

- `Debug` — `println!("{:?}", sm)` works. Always derive this on data types.
- `Clone` — `sm.clone()` returns a deep copy. Required because…
- `Copy` — `let b = sm;` does NOT move out of `sm`. `Copy` is a marker trait
  that says "bitwise copy is the right semantic." A type can only be `Copy`
  if all its fields are. `DateTime<Utc>` and `f64` both are, so we qualify.

**Why this matters.** If `Sample` weren't `Copy`, code like
`times.iter().map(|sm| sm.t)` would have to write `sm.t.clone()` or use
`copied()`. Whenever you write a tiny aggregate of plain data, ask "could
this be `Copy`?"

### The newtype pattern

```rust
pub struct Signal(pub Vec<Sample>);
```

This is a **tuple struct** with one field. Same memory layout as `Vec<Sample>`,
zero runtime cost, but the type system now distinguishes `Signal` from "any
old vec of samples." You can also hang `impl Signal { ... }` blocks off it —
which you couldn't do on bare `Vec<Sample>` because of the orphan rule (you
can only `impl` a trait/type if you own at least one of the two; `Vec` is
foreign).

The `pub` before `Vec<Sample>` exposes the inner field. The book and this
tutorial keep it public so examples can write `Signal(vec![...])`. Production
APIs usually keep the inner field private and expose constructors —
`Signal::from_samples(...)`, `signal.samples()`, etc. Pick deliberately.

### `HashMap<String, Signal>` — why `String`, not `&str`?

```rust
pub struct Trace(pub HashMap<String, Signal>);
```

A `HashMap` *owns* its keys; the alternative would be `HashMap<&str, Signal>`,
but then every key would carry a borrow tied to whoever produced the string,
and you'd need an explicit lifetime parameter on `Trace`. Reach for `String`
keys until you have a concrete reason not to (e.g., interning).

### `Default` derive

`#[derive(Default)]` on a struct generates `Default::default()` returning
"all fields at their defaults." `Vec::default()` is `vec![]`,
`HashMap::default()` is empty. That's why `Trace::default()` produces a
fresh empty trace — no manual constructor needed.

### Adding `chrono`

The source code uses `chrono::DateTime<Utc>` and `chrono::Duration`. Add it:

```toml
# stl_demo/Cargo.toml
[dependencies]
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
```

**Rust feature flags.** `default-features = false` opts out of crates' default
feature sets, then you opt back into only the ones you need. `chrono`'s
default set includes `serde` integration and a few platform-specific things
you don't need here. This is how a Rust project keeps its dependency surface
honest.

### 2.1 `at(t)` — `Option<T>`, slices, `partition_point`

```rust
impl Signal {
    pub fn at(&self, t: DateTime<Utc>) -> Option<f64> { ... }
}
```

**Read the signature one token at a time.** Every piece is load-bearing.

- `impl Signal { ... }` — the surrounding block. `at` is a *method on `Signal`*,
  not a free function. Methods only exist inside an `impl` block; outside
  one, `&self` is a syntax error. (If you see `pub fn at(&self, ...)` floating
  at module scope in your `src/main.rs`, that's the compiler error you're
  about to hit — wrap it in `impl Signal { ... }`.)
- `pub` — visibility. Without it, the method is crate-private; callers
  outside `stl_demo` couldn't reach it. `pub` on a method *and* `pub` on the
  enclosing type are both needed to expose it across crate boundaries.
- `fn at` — `fn` declares a function; `at` is the name. Methods live in the
  same namespace as the type, so callers write `signal.at(t)` (method-call
  sugar) or `Signal::at(&signal, t)` (fully-qualified, useful when the
  compiler can't infer which `at` you meant).
- `&self` — the receiver, shorthand for `self: &Signal`. The leading `&`
  means **borrow, don't consume**: `at` reads the signal and gives it back.
  Three alternatives, each with different semantics:
  - `self` (no `&`) — consume the signal; caller can't use it after.
  - `&mut self` — exclusive borrow; needed to *modify* fields. Disallows any
    other borrow for the duration.
  - `&self` — shared borrow; many readers at once, no writers. This is what
    a pure lookup wants.
- `t: DateTime<Utc>` — the query time. Taken **by value** (no `&`) because
  `DateTime<Utc>` is `Copy` (or close to it — 12 bytes, cheap to copy). The
  rule of thumb: pass small `Copy` types by value, pass everything else by
  `&`.
- `-> Option<f64>` — the return type. *Why `Option` and not just `f64`?*
  Because the signal might be empty, or `t` might fall before the first
  sample. In C you'd return a sentinel like `NaN` and hope the caller checks;
  in Rust the type system forces the caller to handle the "no value" branch.

Now the bit `Option<T>` itself deserves the headline.

`Option<T>` is *the* way Rust represents "might be absent." It's an enum:

```rust
pub enum Option<T> { Some(T), None }
```

There is no `null`. A function that might fail to return a value returns
`Option<T>` (or `Result<T, E>` if there's an error reason). Callers either
`match`, use `?` to propagate, or call combinators like `.unwrap_or(default)`.

Inside `at`:

```rust
let s = &self.0;
```

`&self` is an immutable borrow of the whole struct; `&self.0` borrows the
inner `Vec<Sample>`. `s` is now `&Vec<Sample>` — but Rust auto-derefs `Vec`
to `[Sample]` (a slice) for most operations, so `s.is_empty()`, `s[0]`,
`s.len()`, `s.partition_point(...)` all work on the slice view. You almost
never write `&Vec<T>` in argument types in idiomatic Rust — you write
`&[T]` — but inside a method it's fine.

```rust
let idx = s.partition_point(|sm| sm.t <= t);
```

`partition_point` is *the* binary search you actually want. Given a slice
that's partitioned by a predicate (all-true followed by all-false), it
returns the first index where the predicate is false. O(log n). `|sm| sm.t <= t`
is a **closure**; the `|args|` syntax is Rust's lambda. Closures capture by
reference by default — here it captures `t`, which is `Copy`, so there's no
borrow trouble.

The contrast with the alternative — `binary_search_by_key` — is worth
noticing: `partition_point` doesn't require an exact match, only a
predicate. For "what's the rightmost sample with `s[i].t ≤ t`?" it's
exactly the right tool.

**Exercise.** Replace zero-order hold with linear interpolation between
adjacent samples. Notice that the function signature doesn't change — only
the body. That decoupling is what makes the rest of the library stand on
this one function.

---

## 3. Atomic Predicates — `enum`, traits, `dyn`

The first proper enum:

```rust
pub enum Op { Gt, Lt, Ge, Le }
```

Four variants, no data. Plays the role of an `int` enum in C, but with
exhaustive `match`: if you add a `Eq` variant later, the compiler will flag
every `match self.op { ... }` that doesn't cover it. That's the headline
feature.

### The `Formula` trait

```rust
pub trait Formula {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64;
}
```

A trait is Rust's interface / abstract-method mechanism. Any type that
implements `Formula` can be passed where `&dyn Formula` or `Box<dyn Formula>`
is expected. Traits are also the bedrock of generics — `fn foo<F: Formula>(f: F)`
takes any type satisfying the trait, monomorphised at compile time.

We use `&dyn Formula` (dynamic dispatch via vtable) instead of `<F: Formula>`
(static dispatch via monomorphisation) because a formula AST owns *children
of varying concrete types*. An `And` holds atoms, other `And`s, `Eventually`s,
and so on — you can't pick one type parameter for that.

**Static vs dynamic dispatch:**

| Style                    | Where the type is known        | Cost                  |
|--------------------------|--------------------------------|-----------------------|
| `<F: Formula>` generic   | At compile time                | Zero (inlined)        |
| `&dyn Formula` trait obj | At runtime (vtable lookup)     | One indirect call     |

You'll see both styles in the wild. The decision rule for this library: any
node that *contains other formulas* uses `Box<dyn Formula>` because the
shape of the tree is data-dependent.

### `let-else`

```rust
let Some(sig) = tr.0.get(&self.channel) else {
    return f64::NEG_INFINITY;
};
```

`let-else` (stable since 1.65) destructures or early-returns. Equivalent to:

```rust
let sig = match tr.0.get(&self.channel) {
    Some(s) => s,
    None => return f64::NEG_INFINITY,
};
```

…but flat instead of pyramidal. Use it whenever you've got a bunch of "if
this isn't `Some` / `Ok`, bail" guards at the top of a function. It keeps
the *happy path* unindented, which is a real win for readability.

### Pattern matching on `Op`

```rust
match self.op {
    Op::Gt | Op::Ge => v - self.bound,
    Op::Lt | Op::Le => self.bound - v,
}
```

`match` is an expression — it returns a value, no `return` keyword. The `|`
in patterns is "or". The compiler will yell if you miss a variant — go ahead,
delete `Op::Le =>` and watch the error. That exhaustiveness check is the
single biggest payoff of using `enum`.

---

## 4. Boolean Operators — `Box<dyn Formula>`

```rust
pub struct And { pub children: Vec<Box<dyn Formula>> }
```

This is the line that crystallises trait objects. A `Box<dyn Formula>` is:

- `dyn Formula` — an *unsized* type meaning "some value that implements
  `Formula`, concrete type erased."
- `Box<...>` — a heap allocation that owns its contents and has a known
  size (one pointer), so it can go in a `Vec`.

You cannot have `Vec<dyn Formula>` directly because `dyn Formula` has no
known size at compile time. The `Box` (or `&dyn`, or `Rc<dyn>`, or `Arc<dyn>`)
gives the indirection that makes a sized handle to an unsized thing.

A vtable lives next to the data. Every method call goes through it. That
*does* cost something — but for an STL formula evaluated thousands of times
per second, it's vanishingly cheap compared to the work each leaf does.

### Iterator folds

```rust
self.children.iter()
    .map(|c| c.robustness(tr, t))
    .fold(f64::INFINITY, f64::min)
```

A few things at once:

- `iter()` — iterator of `&Box<dyn Formula>`. Use `iter_mut()` for `&mut`,
  `into_iter()` to consume.
- `map(closure)` — lazy transform. Nothing happens until consumed.
- `fold(init, f)` — Rust's `reduce` with explicit starting accumulator.
- `f64::min` is a function (`fn(f64, f64) -> f64`), passed directly. Rust
  treats functions as first-class values when their signature matches the
  expected `FnMut`. No closure wrapper needed.

The initial value `f64::INFINITY` is "the identity element for min over
finite f64s." `min(INFINITY, x) = x` for any finite `x`. Picking the right
identity for `fold` is a common little puzzle.

**Trap.** `f64::min` returns the non-NaN argument if exactly one is NaN, and
NaN if both are. If your data can produce NaN, switch to a `partial_cmp`-based
fold and decide how to handle it.

### A function that *constructs* a formula

```rust
pub fn implies(p: Box<dyn Formula>, q: Box<dyn Formula>) -> Or { ... }
```

Sugar built from the primitives, not a new trait impl. Free functions that
build common formula patterns are how a real library scales — the user
writes `implies(...)`, the library does the De Morgan rewrite internally.

---

## 5. Temporal Operators — `chrono::Duration`, borrowing across the AST

```rust
pub struct Always {
    pub a: Duration, pub b: Duration, pub f: Box<dyn Formula>,
}
```

**Why `chrono::Duration` and not `std::time::Duration`?** The std one is
unsigned. Internal arithmetic on time windows — `t + self.a` where `self.a`
might end up negative if you allowed it — would panic. `chrono::Duration` is
a signed nanosecond count. The source tutorial calls this out; lock the
reasoning in.

```rust
impl Formula for Always {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        let times = tr.sample_times_in_window(t + self.a, t + self.b);
        ...
    }
}
```

Notice the borrow pattern: `self.f.robustness(tr, tau)` calls into the child
formula, passing the same `&Trace` reference along. Because `&Trace` is an
immutable borrow, you can pass it through any depth of formula tree without
the borrow checker complaining. If you tried this with `&mut Trace`, you'd
hit "cannot borrow as mutable more than once" the moment two siblings tried
to look at the trace concurrently. **Immutable read paths through deeply
nested data are essentially free in Rust.** Save the mutability for the
monitor in §7.

### Vacuous truth and the right identity

```rust
if times.is_empty() {
    return f64::INFINITY; // vacuously true
}
```

If there are no points in the window, `Always` returns `+∞` (trivially
satisfied) and `Eventually` returns `−∞` (trivially violated). These are the
identities of `min` and `max`, and they're the right semantic too — read the
source tutorial's prose. Reusing math constants as semantic anchors is a
nice trick worth noticing.

---

## 6. Robustness as a Signal — `&dyn Formula` in a function signature

```rust
pub fn evaluate_along(spec: &dyn Formula, tr: &Trace, times: &[DateTime<Utc>]) -> Signal {
    let samples = times.iter()
        .map(|&t| Sample { t, v: spec.robustness(tr, t) })
        .collect();
    Signal(samples)
}
```

`&dyn Formula` vs `Box<dyn Formula>`: the function only needs to *read* the
formula. It doesn't own it, doesn't store it. So it takes a borrow. The
caller might be holding a `Box<dyn Formula>`, an `&Always`, or a stack-local
`Atom` — all coerce to `&dyn Formula` at the call site.

`|&t|` is **pattern binding in a closure parameter** — destructuring the
`&DateTime<Utc>` that `iter()` yields. Equivalent to `|t_ref| { let t = *t_ref; ... }`
but one line. Works because `DateTime<Utc>` is `Copy`.

The struct-literal shorthand `Sample { t, v: ... }` — when a local has the
same name as a field, you can drop `t: t`. Tiny, but everywhere.

---

## 7. Sliding Windows and Streaming Monitors — the borrow checker

This is the chapter where the borrow checker becomes a teacher.

```rust
pub struct Monitor {
    pub spec: Box<dyn Formula>,
    pub horizon: Duration,
    pub margin: Duration,
    buf: Trace,
}

impl Monitor {
    pub fn push(&mut self, channel: &str, sm: Sample) -> Option<f64> { ... }
}
```

Two `mut` things to watch.

**`&mut self`** says: "this method needs exclusive access to the monitor for
its duration." Rust's invariant is "you may have any number of `&` OR
exactly one `&mut`, never both." That's what makes data races a compile
error. Inside `push`, you're free to mutate `self.buf` because no one else
can hold a reference to it.

**`Entry` API:**

```rust
let sig = self.buf.0.entry(channel.to_string()).or_default();
sig.0.push(sm);
```

`entry` is `HashMap`'s "insert if missing, return a handle either way"
pattern. Saves you the two-lookup `if !contains_key { insert } else { get_mut }`
dance, and crucially the borrow checker is happy because there's only one
borrow at a time. `or_default()` inserts `Signal::default()` if the key was
absent.

**`partition_point` + `drain`:**

```rust
let drop = s.0.partition_point(|x| x.t < cutoff);
if drop > 0 {
    s.0.drain(..drop);
}
```

`drain(..drop)` removes the first `drop` elements *and* returns an iterator
over them. We don't need the iterator — `drain` is called for its side
effect of shrinking the `Vec`. Equivalent to `vec.splice(..drop, [])` but
clearer.

**The `?` operator on `Option`:**

```rust
let first_t = self.buf.0.get(channel)?.0.first()?.t;
```

`?` after an `Option` (or `Result`) is "if `None`, return `None` from the
enclosing function; otherwise unwrap." Two `?` here means "if either the
channel doesn't exist *or* its signal is empty, give up and return `None`."
The function's return type is `Option<f64>`, which is why this works — the
`?` propagates a `None`-shaped failure cleanly.

### 7.2 `VecDeque` and the sliding-window deque

```rust
let mut dq: VecDeque<usize> = VecDeque::new();
```

`VecDeque<T>` is a ring buffer — O(1) push/pop at *both* ends. `Vec` only
gives O(1) at the back. The monotonic-deque algorithm needs both ends (push
back, pop back to maintain monotonicity, pop front to evict from the
window), so `VecDeque` is the right pick.

```rust
while let Some(&back) = dq.back() {
    if rs[back] >= v { dq.pop_back(); } else { break; }
}
```

`while let` is "loop while the pattern matches." `Some(&back)` destructures
*and* derefs (because `back()` returns `Option<&usize>`). Without the `&`,
`back` would be `&usize`, not `usize`, and the subsequent `rs[back]` would
need a deref.

`dq.front().unwrap()` — `unwrap` panics on `None`. Use it when you've
*proven* the value is `Some` (here, the window-fill check `i + 1 >= w`
guarantees the deque is non-empty). Don't use it when you haven't.

---

## 8. Aggregation, Corpus Monitor — lifetimes show up

§8.1 (`WindowStats`) is mostly plain. §8.2 introduces the first ambiguous
lifetime, and §8.3 forces an explicit one.

### `partial_cmp(...).unwrap()` and the `Ord` story

```rust
vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
```

`f64` is `PartialOrd` but not `Ord` because of NaN. NaN compared to anything
is unordered. `sort` requires `Ord`, so you can't write `vals.sort()`. The
two common workarounds:

1. `vals.sort_by(|a, b| a.partial_cmp(b).unwrap())` — panics on NaN. Fine if
   you know your data is finite.
2. `vals.sort_by(|a, b| a.total_cmp(b))` — stable since 1.62, gives a total
   order that places NaN consistently. Safer.

The source tutorial uses option 1. Be aware option 2 exists.

### Explicit lifetime parameter

```rust
pub struct CorpusMonitor<'a> {
    pub spec: &'a dyn Formula,
    pub window_size: Duration,
}

impl<'a> CorpusMonitor<'a> { ... }
```

This is the first time you'll need to write `'a` yourself. The reason: the
struct stores a borrow (`&dyn Formula`), and Rust needs to know "for how
long is this borrow valid?" — so the struct itself is parameterised by that
lifetime. Any `CorpusMonitor<'a>` is tied to the same `'a` as the formula it
points at.

If you hate writing lifetimes, you can instead store `Box<dyn Formula>`
(owned) and drop the `'a`. Pick based on whether the caller has a long-lived
formula they want to reuse without giving up.

---

## 9. End-to-End Example

By the time you've typed §9 in, you've used: structs, enums, traits, trait
objects, generics implicitly (via iterators), pattern matching, closures,
`Option`, the `?` operator, lifetimes, `&` vs `&mut`, `Box`, `Vec`,
`HashMap`, `VecDeque`, derive macros, and feature flags on a dependency.
That's most of the working vocabulary of day-to-day Rust.

Run it:

```bash
cargo run -p stl_demo
```

…and watch the output. Compare it against the worked numbers in §4 and §6.

---

## 10. Where to Go — Rust-side stretch goals

The source §10 lists STL extensions. Add to those the following
*Rust-shaped* exercises:

- **Replace `Box<dyn Formula>` with an `enum`.** A formula is one of a fixed
  set of variants (`Atom`, `Not`, `And`, `Or`, `Always`, `Eventually`, …).
  Enums dispatch via `match` instead of vtable, often faster and lets the
  compiler exhaustiveness-check formula traversals. The source tutorial
  flags this as a refactor path; doing it will teach you when to choose
  enum-AST vs trait-AST.
- **Wire up `serde` derives** for `Sample`, `Signal`, `Trace`. Add the
  `serde` feature flag to `chrono` and the `serde_json` dep. You'll learn
  how `#[derive(Serialize, Deserialize)]` interacts with newtypes and
  generic timestamp params.
- **Write a doctests.** Every `///` doc comment can hold a runnable example
  fenced as ```` ```rust ```` — `cargo test` runs them. Convert one of the
  worked examples in §4 or §6 into a doctest.
- **Add an integration test.** Make a `tests/` directory at the crate root
  (sibling to `src/`); each file there is a separate binary test. Build one
  that loads a CSV of telemetry into a `Trace` and asserts a spec passes.
- **Try `Rc<dyn Formula>` instead of `Box<dyn Formula>`.** Multiple owners,
  cheap clone. You'll trip on `Rc<T>` not being `Send`, and that's the
  doorway to learning when you reach for `Arc`.
- **Benchmark with `criterion`.** Add `criterion` as a dev-dependency, write
  a bench for the naive Always vs. the monotonic-deque version, watch the
  asymptotic gap show up.

The mental model to carry away on the Rust side: *types describe ownership
as much as they describe data.* `Sample` is `Copy` because it's plain data.
`Signal` is a newtype because we want methods. `Trace` owns its strings.
`Formula` is a trait because the AST is heterogeneous. `Box<dyn>` is the
sized handle to the unsized erased type. `&dyn` is the borrow form for
read-only traversal. `&mut self` is exclusive access during a mutation.
Every choice is the same question repeated: *who owns this, for how long,
and what can they do with it?*

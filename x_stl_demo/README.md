# `stl_demo` — Build an STL Library, Learn Rust

A from-scratch walkthrough that grows `src/main.rs` from `println!("Hello,
world!")` into a working **Signal Temporal Logic** library: signals, traces,
atomic predicates, boolean operators, temporal operators (`Always`,
`Eventually`), and a robustness-over-time evaluator.

You type the code; each section explains the Rust pieces as they appear. By
the end of §6 you'll have a runnable demo that prints a robustness signal for
"temperature stays below 30 °C over the next 5 s."

---

## How to use this tutorial

1. Open three panes: this README, `src/main.rs`, and a terminal in
   `stl_demo/`.
2. Work through the sections **in order**. Each one says exactly what to add
   to `main.rs`.
3. After each section, run `cargo check -p stl_demo` to confirm it still
   compiles. Don't move on until it does — Rust is much friendlier when you
   fix one error at a time.
4. The **Rust notes** under each code block explain *why* every new piece of
   syntax is there. If a concept feels unfamiliar, the noted TRPL chapter
   covers it in depth.

> TRPL = *The Rust Programming Language* book — https://doc.rust-lang.org/book/

---

## §0  Setup

The workspace root (`../Cargo.toml`) already lists `stl_demo` as a member,
and `stl_demo/Cargo.toml` already exists:

```toml
[package]
name = "stl_demo"
version = "0.1.0"
edition = "2024"

[dependencies]
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
```

Your starting `src/main.rs` is the stub:

```rust
fn main() {
    println!("Hello, world!");
}
```

Baseline check:

```bash
cargo run -p stl_demo
# → Hello, world!
```

### Rust notes — the manifest

- **`edition = "2024"`** picks the language edition. Editions are non-breaking
  opt-ins (closures, async, pattern syntax tweaks). The compiler still builds
  older-edition crates in the same workspace.
- **`default-features = false`** opts out of `chrono`'s default feature set;
  `features = ["clock", "std"]` opts back in only what we need (system clock,
  `std` types). This keeps the dependency surface honest — Rust crates often
  bundle optional integrations (serde, wasm, etc.) you don't want unless you
  ask.

---

## §1  What is STL? (one minute of theory)

Three vocabulary items:

- **Signal**: a time series, `(t, v)` samples sorted by time.
- **Trace**: a bundle of named signals — `{"temp": Signal, "pressure": Signal, …}` — representing a system's observed state over time.
- **Formula**: a predicate over a trace at a query time `t`. Atomic
  predicates compare a channel against a constant (`temp < 30`); compound
  formulas combine them with boolean and *temporal* operators
  (`Always[0,5s](temp < 30)` — "stays under 30 °C for the next 5 s").

The key payoff is **robustness**: a formula evaluated at `t` returns not just
`true`/`false`, but a **signed margin** — how positive or negative the
satisfaction is. `temp < 30` when `temp = 27` gives robustness `+3`; when
`temp = 33` it gives `-3`. This makes STL useful for optimisation,
monitoring, and gradient-style search.

That's enough theory. Everything else falls out of the code.

---

## §2  Signals and Traces

We need three data types and one lookup method.

### 2.1  Replace `main.rs` with this

```rust
use std::collections::HashMap;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub t: DateTime<Utc>,
    pub v: f64,
}

pub struct Signal(pub Vec<Sample>);

pub struct Trace(pub HashMap<String, Signal>);

fn main() {
    println!("Hello, world!");
}
```

`cargo check -p stl_demo` should pass.

### Rust notes — `use` and modules

- `use std::collections::HashMap;` — `use` brings a name into scope so you
  can write `HashMap` instead of the full path. `std::collections` is part
  of the standard library, automatically available in every binary crate.
- `use chrono::{DateTime, Utc};` — `chrono` is a third-party crate listed in
  `Cargo.toml`. The braces `{...}` import multiple names from the same
  module in one line.

### Rust notes — `#[derive(...)]` on `Sample`

```rust
#[derive(Debug, Clone, Copy)]
pub struct Sample { ... }
```

`derive` is an attribute that asks the compiler to **auto-generate trait
implementations** for the struct. Without `derive`, you'd write each
`impl Debug for Sample { ... }` by hand. Here:

- **`Debug`** — enables `println!("{:?}", sm)` and `dbg!(sm)`. Always derive
  this on data types; it makes debugging painless.
- **`Clone`** — enables `sm.clone()` (deep copy). Required before you can
  also derive `Copy`.
- **`Copy`** — a *marker trait* that says "bitwise copy is the right
  semantic." Once a type is `Copy`, `let b = sm;` does **not** move out of
  `sm` — both names remain usable. A type can only be `Copy` if every field
  is `Copy`. `DateTime<Utc>` is `Copy` (12 bytes of nanoseconds-since-epoch);
  `f64` is `Copy`. So `Sample` qualifies.

**Why this matters for STL.** Sample values flow through iterators and
closures constantly. If `Sample` weren't `Copy`, you'd be writing `.clone()`
or `.copied()` everywhere. Making your "plain old data" types `Copy` is one
of Rust's quiet superpowers.

**TRPL ch. 4.1** covers the move/copy semantics this rests on.

### Rust notes — the newtype pattern

```rust
pub struct Signal(pub Vec<Sample>);
pub struct Trace(pub HashMap<String, Signal>);
```

These are **tuple structs** with a single field. Memory-wise, `Signal` *is*
a `Vec<Sample>` — zero runtime overhead. But the type system now treats them
as distinct, which buys you two things:

1. **Methods you couldn't otherwise write.** Rust's *orphan rule* says you
   can only `impl SomeTrait for SomeType` (or add `impl SomeType { ... }`
   blocks) when you own at least one of the trait or the type. `Vec` is
   defined in `std`, so you can't add methods to it directly. Wrap it in
   `Signal`, and `impl Signal { ... }` is fine.
2. **Domain meaning in the type system.** A function taking
   `signal: Signal` rather than `samples: Vec<Sample>` communicates intent
   *and* prevents accidental passing of "any old vec of samples."

The `pub` before `Vec<Sample>` exposes the inner field — readers can write
`Signal(vec![...])` to construct one and `sig.0` to reach inside. Production
APIs often hide the field and expose constructors instead
(`Signal::from_samples(...)`); we keep it public to keep the tutorial code
short.

**TRPL ch. 5** covers structs and tuple structs.

### Rust notes — `HashMap<String, Signal>`, not `HashMap<&str, Signal>`

```rust
pub struct Trace(pub HashMap<String, Signal>);
```

Why owned `String` keys? Because a `HashMap` **owns** its keys. The
alternative `HashMap<&str, Signal>` would mean every key is a borrow tied to
some other allocation. That would force `Trace` to carry a lifetime
parameter (`Trace<'a>`), and every value that produces a `Trace` would have
to outlive the `Trace`. For an STL trace that channels live through a long
monitoring session, owning the strings is the right call.

Rule of thumb: **prefer `String` until you have a concrete reason not to**
(e.g., zero-copy parsing, string interning).

**TRPL ch. 4.3** — slices, including `&str` vs `String`.

### 2.2  `Signal::at` — looking up a value at a time

Add this `impl` block **above** `fn main()`:

```rust
impl Signal {
    pub fn at(&self, t: DateTime<Utc>) -> Option<f64> {
        let s = &self.0;
        if s.is_empty() {
            return None;
        }
        let idx = s.partition_point(|sm| sm.t <= t);
        if idx == 0 {
            None
        } else {
            Some(s[idx - 1].v)
        }
    }
}
```

What this does: given a query time `t`, return the value of the most recent
sample at-or-before `t`. This is **zero-order hold** — between samples, the
signal "holds" its last value. If `t` is before the first sample (or the
signal is empty), there's nothing to return: `None`.

`cargo check -p stl_demo` should still pass.

### Rust notes — read the signature one token at a time

Every piece of `pub fn at(&self, t: DateTime<Utc>) -> Option<f64>` is
load-bearing.

- **`impl Signal { ... }`** — methods *only* exist inside an `impl` block.
  Outside one, `&self` is a syntax error.
- **`pub`** — visible outside this crate. The enclosing type also needs to
  be `pub` for the method to be reachable. Without `pub`, the method is
  crate-private.
- **`&self`** — the receiver. Shorthand for `self: &Signal`. The leading `&`
  means **borrow, don't consume**. Three variants:
  - `self` (no `&`) — consume the signal; caller loses it.
  - `&self` — shared borrow; many readers at once, no writers. *This is what
    a pure lookup wants.*
  - `&mut self` — exclusive borrow; needed to *modify* fields. Disallows any
    other borrow at the same time.
- **`t: DateTime<Utc>`** — taken **by value** (no `&`) because `DateTime<Utc>`
  is `Copy` and cheap. Rule of thumb: pass small `Copy` types by value; pass
  everything else by `&`.
- **`-> Option<f64>`** — *why not just `f64`?* Because the function might
  legitimately have nothing to return: empty signal, or `t` before the first
  sample. In C you'd return `NaN` and hope callers check; in Rust the type
  system forces them to.

### Rust notes — `Option<T>` is *the* way to say "might be absent"

```rust
pub enum Option<T> { Some(T), None }
```

There is no `null` in Rust. Anywhere you'd reach for null in another
language, you use `Option<T>`. Callers handle it with `match`, the `?`
operator (which you'll see in §7), or combinators like
`.unwrap_or(default)` and `.map(|x| ...)`.

**TRPL ch. 6** covers enums and `Option`.

### Rust notes — `partition_point` and closures

```rust
let idx = s.partition_point(|sm| sm.t <= t);
```

- **`s.partition_point(...)`** is the binary search you actually want.
  Given a slice partitioned by a predicate (all-true followed by all-false),
  it returns the first index where the predicate becomes false. O(log n).
  Here the predicate is "this sample's time is at-or-before `t`", so the
  index it returns is the first sample *strictly after* `t`. The sample we
  want is `idx - 1`.
- **`|sm| sm.t <= t`** is a **closure** — Rust's lambda. `|args| body`. The
  closure captures `t` from the enclosing scope. Since `t` is `Copy`, the
  capture is by copy; no borrow trouble.

Why not `binary_search_by_key`? Because we don't need an exact match — we
want "the latest sample at-or-before `t`," which may not equal `t`.
`partition_point` is the right shape.

**TRPL ch. 13** covers closures and iterators.

**Optional variant.** If you'd rather extend the *first* sample's value
backward (treating the signal as constant before its first observation),
add a branch before the `partition_point`:

```rust
if t < s[0].t {
    return Some(s[0].v);
}
```

Both choices are valid ZOH variants. The `None`-before-first-sample version
keeps "I don't know" honest; "extend backward" is more forgiving. Pick
deliberately.

---

## §3  Atomic Predicates

We now express "`temp < 30`" as a value. Two pieces: an `Op` enum for the
comparison, and a `Formula` trait so every formula — atomic or compound —
exposes the same `robustness` method.

### 3.1  Add `Op` and `Formula`

Place these *between* the `impl Signal { ... }` block and `fn main()`:

```rust
pub enum Op { Gt, Lt, Ge, Le }

pub trait Formula {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64;
}
```

### Rust notes — enums without data

```rust
pub enum Op { Gt, Lt, Ge, Le }
```

An enum with four variants, none carrying data. Plays the role of a C `enum`
or a Java enum without fields. The headline feature you get over a plain
`int`: **exhaustive `match`**. When you add a fifth variant later
(`Op::Eq`?), the compiler will flag every `match self.op { ... }` that
doesn't cover it. No silent fall-through. No catch-all bugs.

Enum variants can also carry data (`Some(T)` carries a `T`), but you don't
need that here.

**TRPL ch. 6** — enums and `match`.

### Rust notes — traits are Rust's interface mechanism

```rust
pub trait Formula {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64;
}
```

A **trait** declares a set of method signatures any implementing type must
provide. It's the equivalent of an interface (Java) or a protocol (Swift),
with two important differences:

- A trait can also include **default method bodies**, **associated types**,
  and **generic methods**.
- Traits are how Rust does both **static polymorphism** (`fn run<F: Formula>(f: F)`,
  monomorphised at compile time, zero-cost) **and dynamic polymorphism**
  (`fn run(f: &dyn Formula)`, vtable dispatch). You'll see both styles in
  this tutorial.

We choose `&Trace` (shared borrow) for the trace argument because evaluating
a formula doesn't mutate the trace — it just reads. Sharing reads across an
arbitrarily deep formula tree is essentially free in Rust because no writer
can co-exist with the readers.

**TRPL ch. 10** — traits.

### 3.2  The `Atom` struct

```rust
pub struct Atom {
    pub channel: String,
    pub op: Op,
    pub bound: f64,
}

impl Formula for Atom {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        let Some(sig) = tr.0.get(&self.channel) else {
            return f64::NEG_INFINITY;
        };
        let Some(v) = sig.at(t) else {
            return f64::NEG_INFINITY;
        };
        match self.op {
            Op::Gt | Op::Ge => v - self.bound,
            Op::Lt | Op::Le => self.bound - v,
        }
    }
}
```

`cargo check -p stl_demo` should pass.

### Rust notes — `let ... else` (the early-exit pattern)

```rust
let Some(sig) = tr.0.get(&self.channel) else {
    return f64::NEG_INFINITY;
};
```

`let-else` (stable since Rust 1.65, 2022) destructures **or diverges**. If
the pattern matches, the bindings (`sig` here) are in scope for the rest of
the function. If it doesn't, the `else` block must `return`, `panic!`,
`continue`, etc. — it cannot fall through.

Without `let-else`, you'd write:

```rust
let sig = match tr.0.get(&self.channel) {
    Some(s) => s,
    None => return f64::NEG_INFINITY,
};
```

…flat instead of pyramidal. Reach for `let-else` whenever you've got a
stack of "if this isn't `Some` / `Ok`, bail" guards at the top of a
function. The happy path stays at the left margin.

### Rust notes — pattern matching with `|`

```rust
match self.op {
    Op::Gt | Op::Ge => v - self.bound,
    Op::Lt | Op::Le => self.bound - v,
}
```

`match` is an **expression** — it returns a value. No `return` keyword
needed. The `|` between patterns means "or" (this arm matches either). The
compiler **exhaustively** checks: drop `Op::Le =>` and watch the error.
That's the single biggest payoff of using `enum`.

### Rust notes — why `f64::NEG_INFINITY` for "no data"?

When the channel doesn't exist or has no value at `t`, we return `-∞`
("maximally violated"). This matches the **identity of `max`**: any other
robustness combined with `-∞` via `Or` (which uses `max`) is unaffected by
the missing data. And `Or(Atom(missing), Atom(present))` reduces to the
present one's robustness — sensible semantics. Reusing math identities as
semantic anchors is a trick worth noticing.

---

## §4  Boolean Operators

`Not`, `And`, `Or` — plus an `implies()` helper built from primitives.

### 4.1  Add these *before* `fn main()`

```rust
pub struct Not {
    pub f: Box<dyn Formula>,
}

impl Formula for Not {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        -self.f.robustness(tr, t)
    }
}

pub struct And {
    pub children: Vec<Box<dyn Formula>>,
}

impl Formula for And {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        self.children
            .iter()
            .map(|c| c.robustness(tr, t))
            .fold(f64::INFINITY, f64::min)
    }
}

pub struct Or {
    pub children: Vec<Box<dyn Formula>>,
}

impl Formula for Or {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        self.children
            .iter()
            .map(|c| c.robustness(tr, t))
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

pub fn implies(p: Box<dyn Formula>, q: Box<dyn Formula>) -> Or {
    Or {
        children: vec![Box::new(Not { f: p }), q],
    }
}
```

`cargo check -p stl_demo` should pass.

### Rust notes — `Box<dyn Formula>`, the trait object

```rust
pub struct And { pub children: Vec<Box<dyn Formula>> }
```

This is the single most important Rust line in the tutorial.

**Why can't we just write `Vec<Formula>`?** Because `Formula` is a trait,
not a type. A trait has no fixed size — different implementors (`Atom`,
`And`, `Always`) have different sizes. Rust needs to know the size of every
element in a `Vec`. We get around that with **indirection**.

`dyn Formula` is an **unsized type**: "some value that implements
`Formula`, concrete type erased." You can't have `Vec<dyn Formula>` because
the elements would have unknown size. But you *can* have a `Box<dyn Formula>`
— a pointer to a heap allocation, which is itself sized (one machine word).
`Vec<Box<dyn Formula>>` is fine.

At runtime, a `Box<dyn Formula>` is a **fat pointer**:

```
┌──────────────────┬──────────────────┐
│ data ptr         │ vtable ptr       │
│ (→ the value)    │ (→ method table) │
└──────────────────┴──────────────────┘
```

The vtable is a static table the compiler builds per `(concrete type,
trait)` pair. Calling `c.robustness(tr, t)` on a `Box<dyn Formula>` is an
indirect call through the vtable — one extra pointer hop, no inlining
across the call. For STL, that's vanishingly cheap compared to the work in
each leaf.

**Why not generics (`<F: Formula>`)?** Because the AST is *heterogeneous*.
An `And`'s children might be `Atom`, `Or`, `Always`, ... — you can't pick a
single concrete type parameter for the `Vec`. Trait objects let you mix.
(For homogeneous performance-critical loops, prefer generics. For
heterogeneous data, prefer trait objects.)

**TRPL ch. 17** — trait objects and `dyn`.

### Rust notes — `fold` and math identities

```rust
self.children.iter()
    .map(|c| c.robustness(tr, t))
    .fold(f64::INFINITY, f64::min)
```

- `.iter()` — borrowing iterator. Yields `&Box<dyn Formula>` here.
- `.map(closure)` — lazy transform. Nothing happens until something
  consumes the iterator.
- `.fold(init, f)` — Rust's `reduce` with an explicit starting accumulator.
  `init` is the **identity** of the combining function.
- `f64::min` is a free function (`fn(f64, f64) -> f64`) passed directly.
  Rust treats `fn` items as values when the signature matches the expected
  `FnMut`. No closure wrapper needed.

Why `f64::INFINITY` as the identity for `min`?  `min(INFINITY, x) = x` for
any finite `x`. So folding over zero elements gives `INFINITY` — which
matches the STL convention that an empty `And` is *vacuously true* (`+∞`
robustness). The math identity and the semantics agree. (`And` of zero
things being true is the same logic as "the empty product is 1.")

Likewise for `Or`: identity of `max` is `-∞`, and an empty `Or` is
*vacuously false*.

**Trap.** `f64::min` returns the non-NaN argument if exactly one operand
is NaN, NaN if both are. If your data can produce NaN, switch to a
`partial_cmp`-based fold and decide explicitly.

### Rust notes — building formulas from formulas

```rust
pub fn implies(p: Box<dyn Formula>, q: Box<dyn Formula>) -> Or {
    Or { children: vec![Box::new(Not { f: p }), q] }
}
```

`implies(p, q)` is sugar for `Or(Not(p), q)` — the De Morgan / material
implication rewrite. It returns the concrete type `Or` (not `Box<dyn
Formula>`) so callers see exactly what they got. A real library would offer
many such constructors; the user writes the readable form, the library
expands to the primitives.

---

## §5  Temporal Operators

`Always[a,b](φ)` says "for every sample time in `[t+a, t+b]`, φ holds";
`Eventually[a,b](φ)` says "for some sample time in `[t+a, t+b]`, φ holds."
Robustness is `min` (resp. `max`) of φ's robustness across those times.

### 5.1  Pull `Duration` into scope

Update the chrono import:

```rust
use chrono::{DateTime, Duration, Utc};
```

### 5.2  Add a helper on `Trace`

We need: "list every distinct sample time in `[from, to]` across all
channels." Add this `impl Trace` block somewhere above `fn main()`:

```rust
impl Trace {
    pub fn sample_times_in_window(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<DateTime<Utc>> {
        let mut out: Vec<DateTime<Utc>> = self
            .0
            .values()
            .flat_map(|sig| sig.0.iter().map(|sm| sm.t))
            .filter(|&t| t >= from && t <= to)
            .collect();
        out.sort();
        out.dedup();
        out
    }
}
```

### Rust notes — iterator chains

- **`self.0.values()`** — `HashMap` iterator over values only. Yields
  `&Signal`.
- **`.flat_map(|sig| sig.0.iter().map(|sm| sm.t))`** — for each signal,
  yield each sample's timestamp. `flat_map` flattens "iterator of
  iterators" into a single stream.
- **`.filter(|&t| t >= from && t <= to)`** — drop times outside the
  window. The `|&t|` pattern destructures the `&DateTime<Utc>` that
  `filter` provides into a copy (works because `DateTime<Utc>` is `Copy`).
- **`.collect()`** — materialise into a `Vec<DateTime<Utc>>`. The target
  type comes from the `let` annotation; Rust uses *type inference* to pick
  the right `collect` implementation.
- **`out.sort()` then `out.dedup()`** — temporal operators evaluate at each
  *distinct* sample time. `dedup` only removes *consecutive* duplicates, so
  we sort first.

This whole pipeline allocates one `Vec` at the end. Everything before
`.collect()` is lazy — no intermediate vectors.

### Rust notes — `chrono::Duration`, not `std::time::Duration`

```rust
use chrono::{DateTime, Duration, Utc};
```

`std::time::Duration` is **unsigned**. Adding a negative offset would
underflow and panic. `chrono::Duration` is a **signed** nanosecond count —
exactly what time-window arithmetic needs (`t + self.a` where `self.a`
might be `Duration::seconds(-3)`).

This is the kind of detail you only learn the hard way; lock the reasoning
in now.

### 5.3  Add `Always` and `Eventually`

```rust
pub struct Always {
    pub a: Duration,
    pub b: Duration,
    pub f: Box<dyn Formula>,
}

impl Formula for Always {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        let times = tr.sample_times_in_window(t + self.a, t + self.b);
        if times.is_empty() {
            return f64::INFINITY; // vacuously true
        }
        times
            .into_iter()
            .map(|tau| self.f.robustness(tr, tau))
            .fold(f64::INFINITY, f64::min)
    }
}

pub struct Eventually {
    pub a: Duration,
    pub b: Duration,
    pub f: Box<dyn Formula>,
}

impl Formula for Eventually {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        let times = tr.sample_times_in_window(t + self.a, t + self.b);
        if times.is_empty() {
            return f64::NEG_INFINITY; // vacuously false
        }
        times
            .into_iter()
            .map(|tau| self.f.robustness(tr, tau))
            .fold(f64::NEG_INFINITY, f64::max)
    }
}
```

`cargo check -p stl_demo` should pass.

### Rust notes — `iter()` vs `into_iter()`

- **`vec.iter()`** yields `&T` — borrows from the vec, doesn't consume it.
  Use when you'll need the vec again.
- **`vec.into_iter()`** yields `T` — *consumes* the vec; the items are
  moved out. Use when you're done with the vec.

In `Always` and `Eventually`, we built `times` locally and don't need it
afterward, so `into_iter()` is the right call. (For `Copy` types like
`DateTime<Utc>` the practical difference is zero — but the choice signals
intent and matters for non-`Copy` types.)

### Rust notes — recursive trait calls and the borrow checker

```rust
.map(|tau| self.f.robustness(tr, tau))
```

`self.f` is `Box<dyn Formula>`. The call `self.f.robustness(tr, tau)`
dispatches through the vtable to whatever concrete formula sits inside.
That formula might itself be an `And` with children, including another
`Always` — full recursive descent through the AST.

The borrow checker is happy because:

- `&self` is a shared borrow of the `Always`.
- `tr: &Trace` is a shared borrow of the trace.
- Both are *shared*, and shared borrows compose freely. Any depth of
  recursion is fine.

If `robustness` took `&mut Trace`, this whole approach would explode at the
first nested call — you can't have two simultaneous mutable borrows.
**Immutable read paths through deeply nested data are essentially free in
Rust.**

---

## §6  Robustness as a Signal

A formula evaluated at one `t` gives one number. Evaluated at a series of
`t`s, you get a robustness *signal* — a curve you can plot or threshold.

### 6.1  Add `evaluate_along`

```rust
pub fn evaluate_along(
    spec: &dyn Formula,
    tr: &Trace,
    times: &[DateTime<Utc>],
) -> Signal {
    let samples = times
        .iter()
        .map(|&t| Sample { t, v: spec.robustness(tr, t) })
        .collect();
    Signal(samples)
}
```

### Rust notes — `&dyn Formula` in a function signature

```rust
pub fn evaluate_along(spec: &dyn Formula, ...)
```

`&dyn Formula` is a **borrowed trait object**: same vtable indirection as
`Box<dyn Formula>`, but no ownership. The function only needs to read the
formula, so it borrows.

At the call site, any of these coerce automatically to `&dyn Formula`:

- a `Box<dyn Formula>` (via `&*the_box`)
- an `&Always`, `&And`, `&Atom` (any concrete `&T` where `T: Formula`)
- a stack-local `Atom { ... }` via `&atom`

This is the right signature for a *consumer* function. `Box<dyn Formula>`
would force the caller to hand over ownership of the formula, which is
usually wrong if the function just reads it.

### Rust notes — `|&t|`, the destructuring closure pattern

```rust
.map(|&t| Sample { t, v: spec.robustness(tr, t) })
```

`times.iter()` yields `&DateTime<Utc>`. The closure parameter `|&t|`
*destructures* that reference into a value `t: DateTime<Utc>` (works
because `DateTime<Utc>` is `Copy`). Equivalent to `|t_ref| { let t = *t_ref; ... }`
but one line shorter. You'll see this pattern constantly in iterator code.

### Rust notes — struct-literal field punning

```rust
Sample { t, v: spec.robustness(tr, t) }
```

When a local binding has the same name as a field (`t`), you can write `t`
instead of `t: t`. Small but everywhere.

### 6.2  Wire up `main()` with a worked example

Replace `fn main() { println!("Hello, world!"); }` with:

```rust
fn main() {
    let t0 = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
    let sec = |n: i64| t0 + Duration::seconds(n);

    // Temperature ramps 20.0 → 33.5 °C over 10 s.
    // Crosses the 30 °C threshold at t = 7 s.
    let temp = Signal(
        (0..10)
            .map(|i| Sample { t: sec(i), v: 20.0 + (i as f64) * 1.5 })
            .collect(),
    );

    let mut tr = Trace { 0: HashMap::new() };
    tr.0.insert("temp".into(), temp);

    let safe = Atom {
        channel: "temp".into(),
        op: Op::Lt,
        bound: 30.0,
    };

    // "For the next 5 s, temp stays below 30 °C."
    let always_safe = Always {
        a: Duration::seconds(0),
        b: Duration::seconds(5),
        f: Box::new(safe),
    };

    let times: Vec<_> = (0..10).map(sec).collect();
    let r = evaluate_along(&always_safe, &tr, &times);

    for sm in &r.0 {
        println!("t={:>2}s  robustness={:+.2}", sm.t.timestamp(), sm.v);
    }
}
```

Run it:

```bash
cargo run -p stl_demo
```

Expected output:

```
t= 0s  robustness=+2.50
t= 1s  robustness=+1.00
t= 2s  robustness=-0.50
t= 3s  robustness=-2.00
t= 4s  robustness=-3.50
t= 5s  robustness=-3.50
t= 6s  robustness=-3.50
t= 7s  robustness=-3.50
t= 8s  robustness=-3.50
t= 9s  robustness=-3.50
```

### Reading the output

At **`t = 0 s`**, the window covers `[0, 5]` s. Temperatures there are
20.0, 21.5, 23.0, 24.5, 26.0, 27.5 — all below 30. The *tightest* margin
is at `t = 5 s` (`30 - 27.5 = 2.5`). `Always` takes the min: **`+2.50`**.

At **`t = 2 s`**, the window covers `[2, 7]` s. The sample at `t = 7 s`
has value 30.5 — over the threshold by 0.5. `Always` reports that worst
margin: **`-0.50`**.

From **`t = 4 s`** onward, the window includes `t = 9 s` (value 33.5),
which violates by 3.5. That's the new worst case, and `Always` reports it
flat at **`-3.50`** for the rest.

This is the STL robustness story in one plot: positive when the spec holds
with a safety margin, negative with magnitude equal to the depth of the
worst violation in the window.

### Rust notes — the `Trace { 0: HashMap::new() }` syntax

Because `Trace` is a tuple struct (`Trace(pub HashMap<...>)`), its single
field is named `0`. You can build one with either:

```rust
Trace(HashMap::new())          // positional
Trace { 0: HashMap::new() }    // named
```

Both work. We chose the named form here to keep the link between "struct
with one field named `0`" visible — but the positional form is more common
in real code.

(If you derive `Default` on `Trace` — `#[derive(Default)]` — you can also
write `Trace::default()`, since `HashMap::default()` returns an empty map.
A nice cleanup once you understand what's happening.)

---

## §7-§10  Beyond the basics (stretch goals)

By the end of §6 you've used: **structs, enums, traits, trait objects
(`Box<dyn Formula>`, `&dyn Formula`), pattern matching, closures, iterators,
`Option`, derive macros, lifetimes implicitly, `&` vs `&mut`, `Box`, `Vec`,
`HashMap`, the newtype pattern, and feature flags on a dependency**. That's
most of the working vocabulary of day-to-day Rust.

Natural next directions, each with its own Rust lesson:

- **§7  Streaming `Monitor`** — a struct that owns a sliding-window buffer
  and exposes `push(&mut self, channel, sample) -> Option<f64>`. Forces you
  to grapple with `&mut self`, `HashMap::entry().or_default()`,
  `Vec::drain(..n)`, and the `?` operator on `Option`.
- **§7.2  Monotonic deque** — replace the naive `Always`-over-sliding-window
  with a `VecDeque<usize>` algorithm that does O(1) amortised per push.
  Introduces `VecDeque`, `while let`, and `partial_cmp`.
- **§8  `CorpusMonitor<'a>`** — evaluate one spec against many traces, with
  the spec stored as a borrow. First time you need to *write* an explicit
  lifetime parameter on a struct.
- **§10  Refactors that teach Rust** — replace `Box<dyn Formula>` with an
  `enum`-based AST (and discover when `enum` beats `dyn`); wire up `serde`
  derives; add doctests; add `criterion` benchmarks.

Each of those would extend `main.rs` by 50–150 lines and is worth a
separate sitting. The pattern is the same: type the code, read the Rust
notes, run `cargo check`, fix one error at a time.

---

## Reference — TRPL chapter map

| Concept introduced | TRPL chapter |
|---|---|
| `use`, modules | [ch. 7](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html) |
| Structs, methods, `&self` | [ch. 5](https://doc.rust-lang.org/book/ch05-00-structs.html) |
| `move`/`copy` semantics | [ch. 4.1](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html) |
| `&` references, borrow checker | [ch. 4.2](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html) |
| Slices, `&[T]`, `&str` | [ch. 4.3](https://doc.rust-lang.org/book/ch04-03-slices.html) |
| Enums, `Option`, `match` | [ch. 6](https://doc.rust-lang.org/book/ch06-00-enums.html) |
| Generics, traits, lifetimes | [ch. 10](https://doc.rust-lang.org/book/ch10-00-generics.html) |
| Closures, iterators | [ch. 13](https://doc.rust-lang.org/book/ch13-00-functional-features.html) |
| `Box<T>`, smart pointers | [ch. 15](https://doc.rust-lang.org/book/ch15-00-smart-pointers.html) |
| Trait objects (`dyn`) | [ch. 17](https://doc.rust-lang.org/book/ch17-00-oop.html) |

---

## Mental model to carry away

*Types describe ownership as much as they describe data.*

- `Sample` is `Copy` because it's plain data.
- `Signal` is a newtype because we want methods on it.
- `Trace` owns its channel names (`String`) so its lifetime is independent.
- `Formula` is a trait because the AST is heterogeneous.
- `Box<dyn Formula>` is the sized handle to the unsized erased type.
- `&dyn Formula` is the borrow form for read-only traversal.
- `&self` vs `&mut self` is the difference between "many readers" and
  "exclusive writer."

Every choice in this tutorial is the same question repeated: *who owns
this, for how long, and what can they do with it?* Once that question
becomes second nature, Rust feels like Rust instead of like a fight.

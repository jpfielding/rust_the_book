# proj_iterkit

## What you'll build

A small library of hand-rolled iterators and iterator adapters. You'll build a
source iterator from scratch, an infinite one, then wrap iterators in generic
adapter structs that add behavior — stuttering, mapping, skipping — the same
way `std`'s `.map()` and `.filter()` do. By the end you'll know exactly what
`.map().filter().take(3)` desugars to, because you'll have built the pieces
yourself.

## Concepts you'll practice

- Iterator trait & associated types — ch. 13.2
- Generics & trait bounds — ch. 10.1
- Closures `Fn`/`FnMut`/`FnOnce` — ch. 13.1
- `impl Trait` — ch. 10

## Ground rules

- This is test-driven. `src/lib.rs` is a stub — you write everything: the
  types, the impls, and the `#[cfg(test)]` tests that prove they work. Your
  loop is `cargo test -p proj_iterkit`, run early and often.
- Each milestone's definition of done is "the unit tests you write pass." No
  test, no done — an impl with no test is unverified, not finished.
- Hints are collapsible and escalate. Give yourself 20+ minutes of staring at
  the compiler before you open Hint 1. If you open Hint 3 without trying to
  compile anything first, you've skipped the exercise, not finished it.
- Work the milestones in order. Later ones assume the earlier types exist.

## Milestones

### 1. `Countdown` — a source iterator

**Goal:** a struct with no wrapped iterator inside it — a pure source.
`Countdown::new(n)` yields `n, n-1, ..., 1`, then `None` forever after.

**Design questions**
- What single field does `Countdown` need to track its remaining state?
- What is `Self::Item` here, and does it need to be generic at all?
- What happens on `next()` when the internal count is already `0`? Why must
  that check happen before you decrement anything?
- Does `Countdown` need `Clone` or `Copy`? Why or why not?

**Definition of done** — tests you write should assert:
- `Countdown::new(3)` collected into a `Vec<u32>` equals `[3, 2, 1]`.
- Calling `.next()` on a `Countdown::new(0)` immediately returns `None`.
- Calling `.next()` repeatedly after exhaustion keeps returning `None` (it
  doesn't panic or wrap around).

<details><summary>Hint 1 — nudge</summary>

An iterator is just a struct plus a rule for "what's the next value, and how
do I know I'm done." The struct holds *state*, not the whole sequence. Think
about what one `u32` field needs to represent to answer both "what's next"
and "am I done."

</details>

<details><summary>Hint 2 — API pointers</summary>

Look up `std::iter::Iterator` in the std docs — specifically the
`type Item` associated type and the required method `fn next(&mut self) ->
Option<Self::Item>`. That's the entire contract; everything else on the
trait has default implementations built on top of `next`.

</details>

<details><summary>Hint 3 — shape</summary>

```rust
struct Countdown {
    remaining: u32,
}

impl Countdown {
    fn new(start: u32) -> Countdown;
}

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item>;
}
```

</details>

### 2. `Fibs` — an infinite iterator

**Goal:** an iterator yielding the Fibonacci sequence forever:
`1, 1, 2, 3, 5, 8, ...` (or `0, 1, 1, 2, ...` — pick a starting convention and
document it in a comment). Then prove that once `Iterator` is implemented,
every std adapter works on your type for free.

**Design questions**
- Since this iterator never ends, what does `next()` ever return besides
  `Some(...)`?
- What state do you need to compute "the next Fibonacci number" — how many
  previous values must you remember?
- `.take(5)` is not a method you're writing. Where does it come from, and
  what does that tell you about the relationship between `next()` and the
  other 70+ methods on `Iterator`?
- Could this overflow? At what point, and do you care for this exercise?

**Definition of done:**
- `Fibs::new().take(8).collect::<Vec<_>>()` matches your documented sequence
  for the first 8 terms.
- `Fibs::new().nth(20)` returns the expected 21st term (or whatever indexing
  convention you choose — just be consistent and assert it).

<details><summary>Hint 1 — nudge</summary>

An infinite iterator's `next()` has no exit condition at all — it always
returns `Some`. All the "when do I stop" logic (`.take(n)`, `.take_while`,
etc.) lives in *other* iterators that wrap yours, not in `Fibs` itself.

</details>

<details><summary>Hint 2 — API pointers</summary>

You only need to implement `next`. `Iterator::take`, `Iterator::nth`, and
`Iterator::collect` are all default methods on the trait — check the std
docs' "Provided Methods" section on `Iterator` to see the full list you just
got access to.

</details>

<details><summary>Hint 3 — shape</summary>

```rust
struct Fibs {
    curr: u64,
    next: u64,
}

impl Fibs {
    fn new() -> Fibs;
}

impl Iterator for Fibs {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item>;
}
```

</details>

### 3. `Stutter<I>` — your first generic adapter

**Goal:** `Stutter<I>` wraps any iterator `I` and yields each item twice in a
row: `[1, 2, 3]` becomes `[1, 1, 2, 2, 3, 3]`.

**Design questions**
- `Stutter` wraps an inner iterator — what type parameter does it need, and
  what trait bound on that parameter?
- Why does `I::Item` need a `Clone` bound here specifically? What would you
  have to do instead if it didn't (hint: you can't — think about *why*)?
- On a call to `next()`, you need to emit an item, then emit it again on the
  *next* call without pulling a new item from the inner iterator. What field
  holds that pending second copy between calls?
- What happens when the inner iterator is exhausted mid-stutter — does your
  state cleanly resolve to `None`, or do you need to special-case it?

**Definition of done:**
- `Stutter::new(vec![1, 2, 3].into_iter())` collected equals
  `[1, 1, 2, 2, 3, 3]`.
- Stuttering an empty iterator yields an empty sequence (no panics).
- Stuttering a `Countdown::new(2)` produces `[2, 2, 1, 1]` — proof it composes
  with your own iterator from Milestone 1, not just std ones.

<details><summary>Hint 1 — nudge</summary>

Genericity here means "works over *any* iterator," not "works over any type."
The bound goes on the iterator type parameter, constraining what its `Item`
must support. Also think about where the "second copy" needs to be stored so
it survives between two separate calls to `next()`.

</details>

<details><summary>Hint 2 — API pointers</summary>

Look up `std::clone::Clone` and think about why you need an owned duplicate
of a value you're about to yield — you can't hand out the same value twice by
value without either cloning it or restructuring around references. Also
revisit `Option::take` on `std::option::Option` — useful for "consume a
pending value if there is one."

</details>

<details><summary>Hint 3 — shape</summary>

```rust
struct Stutter<I: Iterator> {
    inner: I,
    pending: Option<I::Item>,
}

impl<I: Iterator> Stutter<I> {
    fn new(inner: I) -> Stutter<I>;
}

impl<I: Iterator> Iterator for Stutter<I>
where
    I::Item: Clone,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item>;
}
```

</details>

### 4. `MyMap<I, F>` — re-implementing `map`

**Goal:** reimplement `Iterator::map` yourself: `MyMap` wraps an inner
iterator and a closure, applying the closure to each item lazily.

**Design questions**
- Where does the closure live — who owns it, and for how long?
- Why `FnMut` and not `Fn` or `FnOnce`? What would break with each of the
  other two, given that `next()` is called repeatedly through `&mut self`?
- Why does the output type need its own type parameter (`B`) separate from
  `I::Item`? What would happen if you tried to make output type = input type?
- `MyMap` has two type parameters, `I` and `F`. What does each one need to be
  bounded by, and where do those bounds actually have to be written for the
  type to even compile (struct def vs. impl block)?

**Definition of done:**
- `MyMap::new(vec![1, 2, 3].into_iter(), |x| x * 10)` collected equals
  `[10, 20, 30]`.
- The closure can change the item's *type*, not just its value — e.g. mapping
  `u32` items to `String` items — and it still compiles and collects
  correctly.
- Composing `MyMap` over a `Stutter` over a `Countdown` (three of your own
  types stacked) produces the expected sequence.

<details><summary>Hint 1 — nudge</summary>

The closure is data. A struct that needs to call a closure later, more than
once, has to store it as a field — same as storing any other value. That
immediately tells you something about ownership and about which `Fn*` trait
fits "called repeatedly through a mutable reference."

</details>

<details><summary>Hint 2 — API pointers</summary>

Look up `std::ops::FnMut` vs `Fn` vs `FnOnce` — specifically which one is a
supertrait of which, and what `&mut self` in `next()` implies about what you
can call. Also check how the real `std::iter::Map` struct is declared (its
docs show the struct signature, not the body) for the type-parameter shape.

</details>

<details><summary>Hint 3 — shape</summary>

```rust
struct MyMap<I, F> {
    inner: I,
    f: F,
}

impl<I, F> MyMap<I, F> {
    fn new(inner: I, f: F) -> MyMap<I, F>;
}

impl<I, F, B> Iterator for MyMap<I, F>
where
    I: Iterator,
    F: FnMut(I::Item) -> B,
{
    type Item = B;
    fn next(&mut self) -> Option<Self::Item>;
}
```

</details>

### 5. `EveryNth<I>` — stateful skipping

**Goal:** `EveryNth<I>` wraps an iterator and yields only every Nth item —
`EveryNth::new(iter, 3)` over `1..=9` yields `[3, 6, 9]`.

**Design questions**
- Unlike `Stutter`, one call to `next()` may need to pull *multiple* items
  from the inner iterator before it has one to return (or none, if the inner
  iterator runs out early). What loop shape handles that?
- What state does `EveryNth` need to remember between calls — just the `n`,
  or also a running counter? Where does the counter reset?
- What should `EveryNth::new(iter, 0)` do? Is that a case you handle at
  construction time, at first `next()` call, or not at all (document your
  choice)?
- Does your loop terminate correctly when the inner iterator is exhausted
  partway through counting to N?

**Definition of done:**
- `EveryNth::new((1..=9), 3)` collected equals `[3, 6, 9]`.
- `EveryNth::new((1..=5), 10)` (n bigger than the sequence) collected equals
  `[]`.
- `EveryNth::new((1..=1), 1)` collected equals `[1]` (boundary: every 1st item
  is every item).

<details><summary>Hint 1 — nudge</summary>

`next()` doesn't have to return after exactly one pull from the inner
iterator. It's legal — and necessary here — for it to loop internally,
consuming several inner items, and only return once it has a real value or
the inner iterator is definitively exhausted.

</details>

<details><summary>Hint 2 — API pointers</summary>

You'll likely reach for a `loop` (or repeated `?`-free pattern matching) over
`self.inner.next()` inside your own `next()`, using a `while let` or manual
match with the `Some`/`None` arms. There's no single std method to look up
here — this one's about control flow you write yourself, not a trait you
call into.

</details>

<details><summary>Hint 3 — shape</summary>

```rust
struct EveryNth<I> {
    inner: I,
    n: usize,
}

impl<I> EveryNth<I> {
    fn new(inner: I, n: usize) -> EveryNth<I>;
}

impl<I: Iterator> Iterator for EveryNth<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item>;
}
```

</details>

### 6. `IterKitExt` — chainable adapter methods

**Goal:** an extension trait with a blanket impl so any `Iterator` gets
`.stutter()` and `.every_nth(n)` as methods, letting you write
`(1..10).stutter().every_nth(3)` the way std lets you write
`(1..10).map(f).filter(g)`.

**Design questions**
- What must the trait's methods return — the concrete `Stutter<Self>` /
  `EveryNth<Self>` types, or something else? What are the tradeoffs?
- What bound does the trait itself need (`Self: Sized`? `Self: Iterator`?)
  for the blanket impl to be legal?
- A "blanket impl" means implementing your trait for every type satisfying
  some bound, in one impl block, instead of one type at a time. What does
  that impl header look like for "every `Iterator`"?
- Why does this pattern — extension trait + blanket impl — let you add
  methods to types you don't own (like `std::ops::Range`)? What Rust rule
  makes that legal here but not for arbitrary inherent methods?

**Definition of done:**
- `(1..10).stutter().collect::<Vec<_>>()` matches calling `Stutter::new`
  directly on the same range.
- `(1..10).stutter().every_nth(2).collect::<Vec<_>>()` matches manually
  nesting `EveryNth::new(Stutter::new(1..10), 2)`.
- The extension methods work on at least one non-`Vec`, non-`Range` source —
  e.g. your own `Countdown` or `Fibs::new().take(20)`.

<details><summary>Hint 1 — nudge</summary>

You're not writing a new iterator type here — you're writing a trait whose
methods are thin wrappers that construct the adapter types you already built
in Milestones 3 and 5. The trait is the "front door"; the structs behind it
already exist.

</details>

<details><summary>Hint 2 — API pointers</summary>

Look up how `Iterator` itself is often extended in real crates via "extension
trait" — the pattern is: define a trait with default method bodies, then
`impl<T: Iterator> YourTrait for T {}` with an empty body, since the defaults
do all the work. Check `Sized` as a bound you may need on `Self`.

</details>

<details><summary>Hint 3 — shape</summary>

```rust
trait IterKitExt: Iterator {
    fn stutter(self) -> Stutter<Self>
    where
        Self: Sized,
        Self::Item: Clone;

    fn every_nth(self, n: usize) -> EveryNth<Self>
    where
        Self: Sized;
}

impl<T: Iterator> IterKitExt for T {}
```

</details>

## Compiler errors you'll probably meet

- **E0207 (unconstrained type parameter):** happens if you declare a generic
  parameter (commonly the output type `B` in `MyMap`) on an `impl` block but
  nothing about the trait or the type it's implemented for actually pins that
  parameter down. Rust needs every generic on an `impl` to be inferable from
  either the trait being implemented or the type — it won't guess.
- **E0277 (trait bound not satisfied):** you'll hit this constantly, usually
  meaning "you called `.collect()` or `.clone()` or `==` on something whose
  `Item` type doesn't implement the trait that operation needs." Read the
  bound Rust says is missing, then trace back to *why* your generic struct
  didn't require it in the first place.
- **E0507 (cannot move out of borrowed content):** shows up if you try to
  return an owned value from inside a `&mut self` method by moving it out of
  a field you only have a reference to (e.g. trying to move `self.pending`
  instead of taking it). The fix-shape involves methods that swap a `None`
  in as they hand you the value out — but figure out which method from first
  principles.
- **Closure-capture E0525 (expected closure that implements `Fn`, found one
  that implements `FnMut`):** happens when you write a closure that mutates
  something it captured (like a counter) but hand it to an API that demands
  `Fn`. The trait hierarchy is `Fn: FnMut: FnOnce` — a closure only qualifies
  for `Fn` if calling it never needs `&mut` access to its captures.
- **"expected `Option<Self::Item>`, found `Option<&Self::Item>`" (or the
  reverse):** a plain type mismatch, not a numbered E-code, but you'll meet
  it in Milestone 3 or 5 if you reach for a reference-returning method
  (`.peek()`-style thinking) where the trait contract wants an owned value,
  or vice versa. `Iterator::next` always returns owned `Option<Self::Item>`.

## Stretch goals

- Implement `DoubleEndedIterator` for `Countdown` (it's finite and its
  reverse order is well-defined) so `.rev()` works on it for free too.
- Write a `chunks_of<I>` adapter that groups an inner iterator's items into
  fixed-size `Vec<I::Item>` chunks, yielding a partial final chunk if the
  length doesn't divide evenly — this forces you to buffer inside `next()`
  rather than emit one item per pull.
- Extend `IterKitExt` with `.chunks_of(n)` alongside `.stutter()` and
  `.every_nth(n)` — the pattern is called an **extension trait with a
  blanket impl**, and it's exactly how crates like `itertools` bolt new
  chainable methods onto every iterator in the ecosystem without ever
  touching `std`.
- Add a size hint: implement `size_hint()` for `Countdown` and `EveryNth` (it
  has a default but yours can be tighter) and see how it changes `.collect()`
  performance characteristics for `Vec` — read what `size_hint` is used for
  before assuming it's cosmetic.

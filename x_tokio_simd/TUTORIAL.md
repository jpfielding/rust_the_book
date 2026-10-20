# SIMD in Rust — Building Mitchell Hashimoto's Example Yourself

This tutorial shows you how to port the example from Mitchell Hashimoto's
[Everyone should know SIMD](https://mitchellh.com/writing/everyone-should-know-simd)
from Zig to Rust. **You write the code.** The tutorial explains each step, names
the intrinsics to use, and tells you what to check. A finished reference
implementation lives in [examples/printable_end.rs](examples/printable_end.rs).
Try each step before you look at it.

The article uses Zig's generic `@Vector`. Rust's portable equivalent
(`std::simd`) is nightly-only, so here you use stable `std::arch` intrinsics:
**aarch64 NEON** and **x86_64 AVX2**, plus a scalar fallback.

You work in `src/main.rs`. Check your work with:

```
cargo test -p x_tokio_simd
cargo run --release -p x_tokio_simd     # always benchmark with --release
```

To run the reference instead: `cargo run --release -p x_tokio_simd --example printable_end`.

---

## 1. The problem

Given decoded codepoints (`&[u32]`), find the end of the next run of printable
characters. The run ends at the first value `<= 0xF` (a C0 control character).
Terminals mostly print plain text, so finding this boundary fast lets them handle
the whole run as one batch.

**Your first task:** write the scalar version.

```rust
pub fn printable_end_scalar(cps: &[u32], start: usize) -> usize
```

It returns the index of the first value `<= 0xF` at or after `start`, or
`cps.len()` if there is none. It is a three-line `while` loop. It is also your
**test oracle**: every SIMD version must return exactly what it returns.

## 2. What SIMD is

SIMD (Single Instruction, Multiple Data) means one instruction works on a
*vector* of values. A 128-bit register holds four `u32` lanes, so one compare
instruction tests four values.

| ISA | Register | `u32` lanes | Ideal speedup |
|---|---|---|---|
| ARM NEON (incl. Apple Silicon) | 128 bit | 4 | 4x |
| x86 AVX2 | 256 bit | 8 | 8x |
| x86 AVX-512 | 512 bit | 16 | 16x |

The ideal is a ceiling: loads, branches and the reduction cost something too.

## 3. The five-step pattern

Hashimoto's key point: most SIMD loops share one skeleton, and only step 4 varies
much between algorithms.

1. **Broadcast** constants into every lane (here, the threshold).
2. **Loop** one full vector at a time, advancing by the lane count.
3. **Compare/compute** all lanes with one instruction.
4. **Reduce**: either keep going (all lanes passed) or work out *which* lane
   stopped you.
5. **Scalar tail**: finish the leftover elements (fewer than one vector) with
   your scalar function. It is also the fallback when SIMD isn't available.

His trace: for lanes `0x41, 0x42, 0x43, 0x0A, 0x44, …`, lane 3 fails, so the run
ends at index 3.

## 4. Write the tests first

SIMD bugs live at boundaries, so write tests that are exhaustive there. Create a
`#[cfg(test)]` module that checks each implementation against the scalar oracle:

- **The article's trace**: `[0x41, 0x42, 0x43, 0x0A, 0x44, 0x45, 0x46, 0x47]`
  gives `3`.
- **Every length and position**: for each length `0..40` and a control character
  at each position (or none), assert `simd == scalar`. Lengths from 0 to 40 cross
  the 4- and 8-lane boundaries and exercise the tail. Fill printable slots with a
  big value like `0x1F600`, not just ASCII.
- **Threshold boundary**: `0x10` is printable, `0xF` is not.
- **High bit set**: `u32::MAX` and `0x8000_0000` are printable. (You will see why
  in section 6.)
- **Non-zero `start`.**

## 5. NEON (aarch64, 4 lanes)

Skeleton to fill in: `pub fn printable_end_neon(cps: &[u32], start: usize) -> usize`,
with `use std::arch::aarch64::*;` inside.

| Step | Intrinsic | Notes |
|---|---|---|
| 1. broadcast | `vdupq_n_u32` | Zig's `@splat` |
| 2. load | `vld1q_u32` | takes a `*const u32`; use `cps.as_ptr().add(end)` |
| 3. compare | `vcgtq_u32` | per lane: all ones if true, all zeros if false |
| 4a. all passed? | `vminvq_u32` | horizontal min; equals `u32::MAX` only if every lane is all ones. This is Zig's `@reduce(.And, …)` |
| 4b. which lane? | see below | |

**Safety.** Intrinsics are `unsafe`. Your safety argument is the loop guard:
`while end + LANES <= cps.len()` means every load stays in bounds. NEON is
mandatory on aarch64, so you need no feature check. Put the loop in an `unsafe`
block and write a `// SAFETY:` comment saying exactly this.

**Which lane failed?** Zig does `@bitCast` of the booleans into an integer, then
`@ctz(~mask)`. x86 has a movemask instruction for that; **NEON does not**. Two
options:

- *Simple*: when the all-passed check fails, just scan those 4 lanes with scalar
  code (or let the scalar tail do it by not advancing `end`). Fine for 4 lanes.
- *Closer to the article*: narrow the four 32-bit mask lanes to 16 bits each with
  `vmovn_u32` (64 bits total), move that into a `u64` with `vreinterpret_u64_u16`
  and `vget_lane_u64`, then `(!bits).trailing_zeros() / 16` is the failing lane.
  Work out on paper why dividing by 16 is right.

Structure: while a full vector fits, either advance by `LANES` or find the failing
lane and return. When the loop ends, fall through to the scalar tail starting at
`end`.

## 6. AVX2 (x86_64, 8 lanes)

Skeleton: `#[target_feature(enable = "avx2")] pub unsafe fn printable_end_avx2(...)`,
with `use std::arch::x86_64::*;` inside.

| Step | Intrinsic | Notes |
|---|---|---|
| 1. broadcast | `_mm256_set1_epi32` | takes an `i32` |
| 2. load | `_mm256_loadu_si256` | `u` = unaligned; cast the pointer to `*const __m256i` |
| 3. compare | see trap below | |
| 4. mask | `_mm256_movemask_ps` | needs `_mm256_castsi256_ps` first; returns one bit per lane |

**The unsigned compare trap.** AVX2 only provides a *signed* 32-bit greater-than
(`_mm256_cmpgt_epi32`). Codepoints are `u32`, so `0x8000_0000` would look negative
and be wrongly treated as a control character. Your high-bit test from section 4
should catch this if you use `cmpgt`. The fix is to test `v >= 0x10` (the same as
`v > 0xF`) using `_mm256_max_epu32(v, splat(0x10))` compared for equality with `v`
using `_mm256_cmpeq_epi32`. Convince yourself why the max equals `v` exactly when
`v >= 0x10`.

**Reduce.** `movemask` gives you the Zig `@bitCast` mask directly: it is `0xFF`
when all 8 lanes passed; otherwise `(!mask).trailing_zeros()` is the failing lane.

**Why `unsafe fn`.** `target_feature(avx2)` lets the compiler emit AVX2
instructions, but calling it on a CPU without AVX2 is undefined behaviour. The
caller must check, which is the next section.

## 7. Dispatch

Write one `pub fn printable_end(cps, start)` that picks an implementation:

- `#[cfg(target_arch = "aarch64")]` selects NEON at compile time.
- On `x86_64`, call `is_x86_feature_detected!("avx2")` at runtime and, if true,
  call your AVX2 function inside `unsafe` with a `// SAFETY:` comment citing that
  check.
- Otherwise use the scalar function.

This replaces Zig's `simd.lanes(u32)` returning `null` when there's no vector
width. Watch for `unreachable_code` warnings from the early `return`s and decide
how you want to structure it.

Run your section 4 tests against the dispatch function and the architecture
specific ones.

## 8. Benchmark

In `main`, build a 1 Mi-element `Vec<u32>` of `0x41` with a `0x0A` in the last slot,
and time the scalar and SIMD versions over a few thousand iterations in
`--release`. Wrap inputs and outputs in `std::hint::black_box` or the compiler may
delete the work.

For reference, the reference example measured on an aarch64 container (NEON):

```
scalar: 249.78µs per scan (16.8 GB/s)
simd  : 72.553µs per scan (57.8 GB/s)
```

About 3.4x against an ideal 4x. Expect different numbers on your machine; use
`criterion` for anything you'd act on. The article reports about 5x on AVX2 for the
end-to-end terminal workload.

**Caveat:** the AVX2 path in the reference was only type-checked
(`cargo check --target x86_64-unknown-linux-gnu --tests`), not run, because the
dev machine is aarch64. Run it on an AVX2 machine before trusting it.

## 9. Takeaways

- Map your loop onto the five steps. Steps 1, 2, 3 and 5 barely change between
  algorithms; step 4 is where the thinking is.
- Keep a scalar oracle and test every length and position around the vector width.
- Explicit SIMD keeps the optimisation visible; auto-vectorization can silently
  vanish after an unrelated change.
- Use `std::simd` once it stabilises for one implementation across targets.
- This example is deliberately simple. simdjson and simdutf use far more advanced
  techniques.

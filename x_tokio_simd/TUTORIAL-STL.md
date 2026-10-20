# Pragmatic SIMD for STL Robustness

How to apply SIMD to the Signal Temporal Logic evaluator in
[src/stl.rs](src/stl.rs) without turning the library into an intrinsics project.
This is a companion to [TUTORIAL.md](TUTORIAL.md) (which covers the five-step SIMD
pattern from Mitchell Hashimoto's article; read that first if the words
"broadcast / reduce / scalar tail" are new).

**You write the code in `src/`.** The working reference is
[examples/stl_simd.rs](examples/stl_simd.rs); try each step before opening it.

```
cargo test -p x_tokio_simd                       # the stl.rs unit tests
cargo test -p x_tokio_simd --example stl_simd    # reference kernels + differential test
cargo run  -p x_tokio_simd --example stl_simd --release
```

---

## 1. Measure before you vectorize

The reference example benchmarks the same formula
(`Always[0,500ms](x > 0 -> Eventually[0,200ms](y < 50))`) over 2000 samples at
10 ms spacing, on an aarch64 container (NEON), `--release`:

```
scalar Formula::robustness      ~430 ms
dense  eval                      ~27 µs
```

That is roughly **15,000x**, and **none of it is SIMD**. It comes from layout and
algorithm:

- `Trace::sample_times_in_window` scans *every sample of every channel*, collects
  into a fresh `Vec`, sorts and dedups, on **every** `Always`/`Eventually`
  evaluation at **every** time point. That is O(n) work (plus allocation) per
  point, so O(n²) overall, before any arithmetic.
- `Atom` does a `HashMap` string lookup and a `partition_point` binary search per
  point.
- Samples are `{DateTime, f64}` structs, so values are strided in memory, not
  contiguous.

Treat the numbers as one run on one machine with a hand-rolled timer; the order of
magnitude is the point. The lesson for "pragmatic SIMD": **fix the data layout
first**. SIMD is only possible *after* it, and gives a further constant factor
(section 6) on top of a change that is already worth four orders of magnitude.

## 2. Where SIMD fits in STL

Robustness semantics map onto four kinds of operation:

| Formula node | Operation on robustness arrays | SIMD shape |
|---|---|---|
| `Atom` (`>`/`<`) | `v - bound` or `bound - v` | elementwise map |
| `Not` | negate | elementwise map |
| `And` / `Or` | elementwise min / max of children | elementwise zip |
| `Always` / `Eventually` | min / max over a window of the child | **reduction** over a contiguous slice |

The first three are trivial for LLVM. The windowed reduction is the interesting
kernel, and it is the same shape as Hashimoto's example: scan a contiguous run,
keep going or stop, reduce to one value.

## 3. Step 1: dense, structure-of-arrays traces

SIMD needs contiguous `f64`s on a shared time grid. Define something like:

```rust
struct DenseTrace { dt: Duration, n: usize, chans: HashMap<String, Vec<f64>> }
```

and an evaluator `eval(&Formula) -> Vec<f64>` returning the robustness at **every
grid index**, instead of `robustness(trace, t) -> f64` for one `t`. Evaluate
children once, then combine arrays; one `HashMap` lookup per atom per
formula, not per sample.

**This is a semantic change; decide it deliberately.**

- It assumes all channels are sampled on **one uniform grid**. The existing
  evaluator handles irregular, per-channel timestamps with zero-order hold. The
  `cars` test in `stl.rs` is irregular, so it cannot be evaluated densely as-is.
  Resampling it onto a grid would *add* sample points, and `Always`/`Eventually`
  would then see more points in each window and can legitimately give different
  answers. So the dense path is for data that is already uniform (typical for
  sensor logs), and irregular data stays on the existing path.
- Window `[a, b]` becomes sample offsets `ceil(a/dt) ..= floor(b/dt)`. Work out
  why `ceil` for the lower bound and `floor` for the upper (only grid points
  inside the real-time interval count). Reject negative bounds explicitly.
- The window is clipped at the end of the trace. If it starts past the end, it is
  empty: `Always` gives `+inf`, `Eventually` gives `-inf`. This matches the
  existing evaluator's vacuous cases; keep it.
- A missing channel gives `-inf` at every index, as in the existing `Atom` arm.

## 4. Step 2: the oracle (do this before writing kernels)

The existing `Formula::robustness` is your **oracle**, exactly like the scalar
loop in the first tutorial. Write a differential test:

1. Build a random uniform `DenseTrace` (a tiny LCG is enough, no `rand` needed).
2. Convert it to a sparse `Trace` (one `Sample` per grid point).
3. For several formulas and for **every** index, assert dense == scalar.

Formulas worth covering: a bare atom; a missing channel; `Not`; `And`; `Or`;
`Eventually` with bounds that are *not* multiples of `dt` (e.g. 15 ms and 47 ms at
`dt = 10 ms`, giving offsets 2..=4); a zero-width window `[0, 0]`; a window
starting beyond a short trace; and a nested `Always(implies(.., Eventually(..)))`.
Run it over lengths that cross the vector width: `1, 2, 7, 8, 9, 33, 70`.

Compare with `==` (not an epsilon) plus an allowance for `-0.0 == 0.0`; the
dense and sparse paths do the same IEEE operations on the same values.

**Check that the test can fail.** Temporarily change `ceil` to `floor` in your
offset calculation and confirm the test goes red. The reference does this; an
oracle test you've never seen fail is not evidence.

## 5. Step 3: the safe, autovectorized kernel

The library has `#![forbid(unsafe_code)]` ([src/lib.rs](src/lib.rs)), which rules
out `std::arch`. That is a feature worth keeping, so the default kernel is plain
safe Rust arranged so LLVM vectorizes it. Map it onto the five steps:

1. **Broadcast**: an accumulator array `[f64::INFINITY; LANES]` (`LANES = 8`).
2. **Loop**: `xs.chunks_exact(LANES)`; the bounds check disappears because each
   chunk has a statically known length.
3. **Compare**: for each lane `i`, `acc[i] = if c[i] < acc[i] { c[i] } else { acc[i] }`.
4. **Reduce**: fold the `LANES` accumulators into one value.
5. **Tail**: `chunks.remainder()` goes through the same scalar fold.

Why independent accumulators? Floating-point min isn't reassociated for a plain
sequential loop, so one accumulator is one long dependency chain. Eight
independent accumulators are eight independent chains, and the loop body is a
natural vector operation.

Do the same for max, and zip loops for `min2` / `max2` / atoms.

**NaN contract.** `a < b` style selects are not `f64::min`: `f64::min` ignores NaN,
selects don't. Decide and document. The reference says "inputs must not contain
NaN" and the tests use finite values and `±inf`. If NaN can reach you (bad sensor
data), validate at the `DenseTrace` boundary rather than paying for it in the
kernel.

Test the kernel against `fold(f64::INFINITY, f64::min)` for **every length 0..70**,
and put the minimum at every lane position and in the tail (lengths 1, 7, 8, 9, 16,
23). These are the boundary bugs from the first tutorial again.

## 6. Step 4: intrinsics, and what they actually buy

The reference adds `reduce_min` with `std::arch` (NEON on aarch64; AVX2 on x86_64
with `is_x86_feature_detected!`) so you can measure the gap. Intrinsics need
`unsafe`, so they live in `examples/`, which is a separate crate and isn't covered
by the library's `forbid`. Intrinsics to look up:

| | NEON | AVX2 |
|---|---|---|
| broadcast | `vdupq_n_f64` | `_mm256_set1_pd` |
| load | `vld1q_f64` | `_mm256_loadu_pd` |
| min | `vminq_f64` | `_mm256_min_pd` |
| horizontal reduce | `vminvq_f64` | store to `[f64; 4]`, scalar fold |

Measured on the aarch64 container (`--release`, one run each):

```
reduce_min, 64 Ki f64:   naive 7.8 µs   safe chunked 7.4 µs   intrinsics 3.8 µs
window(min), n=20k, w=501:   1.22 ms    1.13 ms               0.67 ms
```

Read this carefully rather than as "SIMD wins":

- The safe chunked version **did vectorize**. The emitted assembly (`cargo rustc
  --release --example stl_simd -- --emit asm`) shows `fcmgt v.2d` + `bit` on four
  accumulators. It just isn't *faster than the naive fold*, which LLVM already
  handled well enough here.
- Intrinsics are about **2x faster** on NEON because `vminq_f64` is one `fmin`
  instruction, while the select form is a compare plus a bit-select (two
  instructions per vector). On NEON the two aren't equivalent for NaN, so LLVM
  can't swap them. **I have not measured x86.** There, `a < b ? a : b` is
  exactly the semantics of `minpd`, so the safe version may well match the
  intrinsics; check before assuming either way. The AVX2 code in the reference is
  only type-checked (`cargo check --target x86_64-unknown-linux-gnu`), not run.
- Inside `Always` with a small window (the benchmark spec uses 51 samples) the
  kernel is nowhere near the bottleneck. The 15,000x came from section 3.

## 7. Recommendation

1. Do section 3 and the oracle test (sections 3-4) first. Largest win by far,
   and no SIMD.
2. Ship the safe chunked kernel. It keeps `forbid(unsafe_code)`, needs no
   dispatch, and is portable.
3. Add intrinsics only if a profile of your real workload shows `reduce_min` /
   `reduce_max` dominating, and then behind a Cargo feature or a separate crate
   so the safe crate stays `forbid(unsafe_code)`, keeping the safe version as the
   fallback and as the test oracle for the fast one.
4. For large windows, change the algorithm before the instructions: a sliding
   window min/max (monotonic deque, or van Herk/Gil-Werman) is O(n) regardless of
   window length, versus O(n·w) for "reduce each window". The reference uses the
   O(n·w) form because it's what SIMD accelerates; at `w = 5000` the deque will
   beat any vector width. This is a hypothesis from the complexity, which I did
   not benchmark.
5. When `std::simd` stabilises, it replaces the hand-rolled accumulator arrays and
   the per-arch intrinsics with one portable implementation.

## 8. What changed in the crate to support this

- `Cargo.toml`: `chrono` dependency (what `stl.rs` needs).
- `src/lib.rs`: `mod stl` became `pub mod stl` so the example can use `Formula`.
- `src/stl.rs`: the `main`, `cars` and `temps` demos became the unit tests
  `cars_brake_response` and `temps_always_below_threshold`, asserting the
  robustness values that the demos used to print (checked by hand against the
  semantics, not just copied from output).

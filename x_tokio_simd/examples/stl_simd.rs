//! Pragmatic SIMD for STL robustness. Companion to TUTORIAL-STL.md.
//!
//! Dense (uniform-grid, structure-of-arrays) evaluation of `stl::Formula`, with the
//! hot kernel (`reduce_min`) implemented three ways: naive fold, safe chunked
//! (autovectorized), and hand-written intrinsics. The scalar `Formula::robustness`
//! from src/stl.rs is the test oracle.
//!
//!   cargo test  -p x_tokio_simd --example stl_simd
//!   cargo run   -p x_tokio_simd --example stl_simd --release

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::hint::black_box;
use std::time::Instant;
use x_tokio_simd::stl::{Formula, Op, Sample, Signal, Trace};

// ---------------------------------------------------------------------------
// Safe kernels. Inputs must not contain NaN: comparisons below are `<`/`>`
// based (like x86 min/max), not `f64::min` (which ignores NaN).
// ---------------------------------------------------------------------------

const LANES: usize = 8;

/// Naive reduction, the baseline. Empty input gives +inf (vacuous Always).
fn reduce_min_naive(xs: &[f64]) -> f64 {
    xs.iter().copied().fold(f64::INFINITY, f64::min)
}

/// Chunked reduction. Float min is not reassociated by LLVM for plain loops, so
/// we hand it LANES independent accumulators; each chunk step is then LANES
/// independent compare/selects, which LLVM turns into vector min.
fn reduce_min(xs: &[f64]) -> f64 {
    let mut acc = [f64::INFINITY; LANES];
    let chunks = xs.chunks_exact(LANES);
    let tail = chunks.remainder();
    for c in chunks {
        for i in 0..LANES {
            acc[i] = if c[i] < acc[i] { c[i] } else { acc[i] };
        }
    }
    let mut m = f64::INFINITY;
    for &x in acc.iter().chain(tail) {
        m = if x < m { x } else { m };
    }
    m
}

fn reduce_max(xs: &[f64]) -> f64 {
    let mut acc = [f64::NEG_INFINITY; LANES];
    let chunks = xs.chunks_exact(LANES);
    let tail = chunks.remainder();
    for c in chunks {
        for i in 0..LANES {
            acc[i] = if c[i] > acc[i] { c[i] } else { acc[i] };
        }
    }
    let mut m = f64::NEG_INFINITY;
    for &x in acc.iter().chain(tail) {
        m = if x > m { x } else { m };
    }
    m
}

fn min2(a: &[f64], b: &[f64]) -> Vec<f64> {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| if x < y { x } else { y })
        .collect()
}

fn max2(a: &[f64], b: &[f64]) -> Vec<f64> {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| if x > y { x } else { y })
        .collect()
}

fn atom(values: &[f64], op: Op, bound: f64) -> Vec<f64> {
    match op {
        Op::Gt | Op::Ge => values.iter().map(|v| v - bound).collect(),
        Op::Lt | Op::Le => values.iter().map(|v| bound - v).collect(),
    }
}

/// out[i] = reduce(child[i+lo ..= min(i+hi, n-1)]), or `empty` if i+lo >= n.
fn window(child: &[f64], lo: usize, hi: usize, empty: f64, reduce: fn(&[f64]) -> f64) -> Vec<f64> {
    let n = child.len();
    (0..n)
        .map(|i| {
            if i + lo >= n || hi < lo {
                empty
            } else {
                reduce(&child[i + lo..=(i + hi).min(n - 1)])
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Dense trace + evaluator
// ---------------------------------------------------------------------------

/// All channels sampled on one shared grid: t_i = t0 + i*dt. Structure of arrays.
struct DenseTrace {
    dt: Duration,
    n: usize,
    chans: HashMap<String, Vec<f64>>,
}

impl DenseTrace {
    /// Rebuild the equivalent sparse `Trace`, so the scalar evaluator can act as oracle.
    fn to_trace(&self, t0: DateTime<Utc>) -> Trace {
        Trace(
            self.chans
                .iter()
                .map(|(k, vs)| {
                    let s = vs
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| Sample {
                            t: t0 + self.dt * i as i32,
                            v,
                        })
                        .collect();
                    (k.clone(), Signal(s))
                })
                .collect(),
        )
    }

    /// Window offsets in samples: ceil(a/dt) ..= floor(b/dt). Panics on negative bounds.
    fn offsets(&self, a: Duration, b: Duration) -> (usize, usize) {
        let dt = self.dt.num_nanoseconds().unwrap();
        let (a, b) = (a.num_nanoseconds().unwrap(), b.num_nanoseconds().unwrap());
        assert!(
            a >= 0 && b >= 0,
            "dense evaluation needs non-negative windows"
        );
        (((a + dt - 1) / dt) as usize, (b / dt) as usize)
    }

    /// Robustness of `f` at every grid point.
    fn eval(&self, f: &Formula) -> Vec<f64> {
        match f {
            Formula::Atom { channel, op, bound } => match self.chans.get(channel) {
                Some(vs) => atom(vs, *op, *bound),
                None => vec![f64::NEG_INFINITY; self.n],
            },
            Formula::Not(c) => self.eval(c).iter().map(|x| -x).collect(),
            Formula::And(cs) => cs
                .iter()
                .map(|c| self.eval(c))
                .reduce(|a, b| min2(&a, &b))
                .unwrap_or_else(|| vec![f64::INFINITY; self.n]),
            Formula::Or(cs) => cs
                .iter()
                .map(|c| self.eval(c))
                .reduce(|a, b| max2(&a, &b))
                .unwrap_or_else(|| vec![f64::NEG_INFINITY; self.n]),
            Formula::Always { a, b, f } => {
                let (lo, hi) = self.offsets(*a, *b);
                window(&self.eval(f), lo, hi, f64::INFINITY, reduce_min)
            }
            Formula::Eventually { a, b, f } => {
                let (lo, hi) = self.offsets(*a, *b);
                window(&self.eval(f), lo, hi, f64::NEG_INFINITY, reduce_max)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Intrinsics: the same reduce_min, by hand. Needs `unsafe`, which is why this
// lives in an example and not in the `forbid(unsafe_code)` library.
// ---------------------------------------------------------------------------

#[cfg(target_arch = "aarch64")]
fn reduce_min_neon(xs: &[f64]) -> f64 {
    use std::arch::aarch64::*;
    // 4 independent 2-lane accumulators = 8 elements per iteration.
    let chunks = xs.chunks_exact(8);
    let tail = chunks.remainder();
    // SAFETY: NEON is mandatory on aarch64; each chunk is exactly 8 f64s,
    // so the four 2-lane loads at offsets 0,2,4,6 are in bounds.
    let mut m = unsafe {
        let inf = vdupq_n_f64(f64::INFINITY);
        let (mut a0, mut a1, mut a2, mut a3) = (inf, inf, inf, inf);
        for c in chunks {
            let p = c.as_ptr();
            a0 = vminq_f64(a0, vld1q_f64(p));
            a1 = vminq_f64(a1, vld1q_f64(p.add(2)));
            a2 = vminq_f64(a2, vld1q_f64(p.add(4)));
            a3 = vminq_f64(a3, vld1q_f64(p.add(6)));
        }
        vminvq_f64(vminq_f64(vminq_f64(a0, a1), vminq_f64(a2, a3)))
    };
    for &x in tail {
        m = if x < m { x } else { m };
    }
    m
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn reduce_min_avx2(xs: &[f64]) -> f64 {
    use std::arch::x86_64::*;
    // 2 independent 4-lane accumulators = 8 elements per iteration.
    let chunks = xs.chunks_exact(8);
    let tail = chunks.remainder();
    let mut lanes = [f64::INFINITY; 4];
    // SAFETY: each chunk is exactly 8 f64s; loads are unaligned (`loadu`).
    unsafe {
        let inf = _mm256_set1_pd(f64::INFINITY);
        let (mut a0, mut a1) = (inf, inf);
        for c in chunks {
            let p = c.as_ptr();
            a0 = _mm256_min_pd(a0, _mm256_loadu_pd(p));
            a1 = _mm256_min_pd(a1, _mm256_loadu_pd(p.add(4)));
        }
        _mm256_storeu_pd(lanes.as_mut_ptr(), _mm256_min_pd(a0, a1));
    }
    let mut m = f64::INFINITY;
    for &x in lanes.iter().chain(tail) {
        m = if x < m { x } else { m };
    }
    m
}

/// Best available implementation for this CPU.
fn reduce_min_best(xs: &[f64]) -> f64 {
    #[cfg(target_arch = "aarch64")]
    {
        return reduce_min_neon(xs);
    }
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 support was just checked.
            return unsafe { reduce_min_avx2(xs) };
        }
    }
    #[allow(unreachable_code)]
    reduce_min(xs)
}

// ---------------------------------------------------------------------------
// Demo / benchmark
// ---------------------------------------------------------------------------

/// Tiny deterministic PRNG so the example has no dependencies.
struct Lcg(u64);
impl Lcg {
    fn next_f64(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64) * 200.0 - 100.0
    }
}

fn dense(n: usize, seed: u64) -> DenseTrace {
    let mut r = Lcg(seed);
    let mut chans = HashMap::new();
    for name in ["x", "y"] {
        chans.insert(name.to_string(), (0..n).map(|_| r.next_f64()).collect());
    }
    DenseTrace {
        dt: Duration::milliseconds(10),
        n,
        chans,
    }
}

fn spec() -> Formula {
    let at = |c: &str, op, bound| Formula::Atom {
        channel: c.into(),
        op,
        bound,
    };
    // Always[0,500ms]( x > 0  ->  Eventually[0,200ms]( y < 50 ) )
    Formula::Always {
        a: Duration::zero(),
        b: Duration::milliseconds(500),
        f: Box::new(x_tokio_simd::stl::implies(
            at("x", Op::Gt, 0.0),
            Formula::Eventually {
                a: Duration::zero(),
                b: Duration::milliseconds(200),
                f: Box::new(at("y", Op::Lt, 50.0)),
            },
        )),
    }
}

fn time<T>(name: &str, iters: u32, mut f: impl FnMut() -> T) {
    let t = Instant::now();
    for _ in 0..iters {
        black_box(f());
    }
    println!("  {name:<28} {:>12?} / iter", t.elapsed() / iters);
}

fn main() {
    // 1. Kernel: reduce_min over 64 Ki f64.
    let xs: Vec<f64> = {
        let mut r = Lcg(1);
        (0..1 << 16).map(|_| r.next_f64()).collect()
    };
    println!("reduce_min over {} f64:", xs.len());
    time("naive fold(f64::min)", 2000, || {
        reduce_min_naive(black_box(&xs))
    });
    time("safe chunked (autovec)", 2000, || {
        reduce_min(black_box(&xs))
    });
    time("intrinsics (best for CPU)", 2000, || {
        reduce_min_best(black_box(&xs))
    });

    // 1b. The kernel inside a sliding window: Always over 500 samples of 20k.
    let child = &xs[..20_000];
    println!("\nwindow(min), n = 20000, w = 501:");
    time("naive fold", 3, || {
        window(black_box(child), 0, 500, f64::INFINITY, reduce_min_naive)
    });
    time("safe chunked", 3, || {
        window(black_box(child), 0, 500, f64::INFINITY, reduce_min)
    });
    time("intrinsics", 3, || {
        window(black_box(child), 0, 500, f64::INFINITY, reduce_min_best)
    });

    // 2. End to end: the same formula, sparse scalar evaluator vs dense.
    let n = 2000;
    let d = dense(n, 7);
    let t0 = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
    let tr = d.to_trace(t0);
    let times: Vec<_> = (0..n as i32).map(|i| t0 + d.dt * i).collect();
    let f = spec();
    println!("\nspec over {n} samples (dt = 10 ms, Always window = 50 samples):");
    time("scalar Formula::robustness", 3, || {
        times
            .iter()
            .map(|&t| f.robustness(black_box(&tr), t))
            .sum::<f64>()
    });
    time("dense eval", 200, || d.eval(black_box(&f)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: f64, b: f64) -> bool {
        a == b || (a - b).abs() < 1e-12 // == also covers +-inf and -0.0 vs 0.0
    }

    #[test]
    fn reduce_kernels_match_naive_for_every_length() {
        let mut r = Lcg(42);
        for len in 0..70 {
            let xs: Vec<f64> = (0..len).map(|_| r.next_f64()).collect();
            let want_min = xs.iter().copied().fold(f64::INFINITY, f64::min);
            let want_max = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            assert!(same(reduce_min(&xs), want_min), "safe min len={len}");
            assert!(same(reduce_max(&xs), want_max), "safe max len={len}");
            assert!(same(reduce_min_best(&xs), want_min), "best min len={len}");
            #[cfg(target_arch = "x86_64")]
            if is_x86_feature_detected!("avx2") {
                assert!(
                    same(unsafe { reduce_min_avx2(&xs) }, want_min),
                    "avx2 len={len}"
                );
            }
        }
    }

    #[test]
    fn min_position_does_not_matter() {
        // The minimum in every lane position and in the tail.
        for len in [1, 7, 8, 9, 16, 23] {
            for pos in 0..len {
                let mut xs = vec![5.0; len];
                xs[pos] = -3.0;
                assert_eq!(reduce_min(&xs), -3.0, "len={len} pos={pos}");
                assert_eq!(reduce_min_best(&xs), -3.0, "len={len} pos={pos}");
            }
        }
    }

    #[test]
    fn dense_matches_scalar_oracle() {
        let t0 = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let ms = Duration::milliseconds;
        let at = |c: &str, op, bound| Formula::Atom {
            channel: c.into(),
            op,
            bound,
        };
        let formulas = vec![
            spec(),
            at("x", Op::Gt, 10.0),
            at("nope", Op::Gt, 0.0), // missing channel: -inf everywhere
            Formula::Not(Box::new(at("y", Op::Le, 0.0))),
            Formula::And(vec![at("x", Op::Gt, 0.0), at("y", Op::Lt, 0.0)]),
            Formula::Or(vec![at("x", Op::Gt, 50.0), at("y", Op::Gt, 50.0)]),
            // bounds that are not multiples of dt: window is ceil(15/10)=2 ..= floor(47/10)=4
            Formula::Eventually {
                a: ms(15),
                b: ms(47),
                f: Box::new(at("x", Op::Gt, 0.0)),
            },
            Formula::Always {
                a: ms(0),
                b: ms(0),
                f: Box::new(at("y", Op::Gt, 0.0)),
            },
            // window starting beyond the end of short traces: vacuous
            Formula::Always {
                a: ms(400),
                b: ms(600),
                f: Box::new(at("x", Op::Gt, 0.0)),
            },
        ];
        for n in [1, 2, 7, 8, 9, 33, 70] {
            let d = dense(n, n as u64);
            let tr = d.to_trace(t0);
            for (k, f) in formulas.iter().enumerate() {
                let got = d.eval(f);
                for i in 0..n {
                    let want = f.robustness(&tr, t0 + d.dt * i as i32);
                    assert!(
                        same(got[i], want),
                        "formula {k} n={n} i={i}: dense {} vs scalar {want}",
                        got[i]
                    );
                }
            }
        }
    }
}

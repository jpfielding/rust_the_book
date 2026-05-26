//! STL (Signal Temporal Logic) demo — AST-as-enum variant.
//!
//! This crate is a sibling of `stl_demo` that ports the same semantics
//! to a different Rust idiom. STL background and the meaning of
//! "robustness" live in the sibling crate's docs; the value of this
//! crate is the *delta* in API shape:
//!
//! - `stl_demo` models the formula AST as a `trait Formula` with
//!   `Box<dyn Formula>` children (open hierarchy, dynamic dispatch).
//! - `stl_ast_demo` models it as a single `enum Formula` with a
//!   `match`-based `robustness` method (closed AST, static dispatch).
//!
//! Each `// WHY vs stl_demo:` comment marks a deliberate choice so a
//! reader can diff the two designs side-by-side.

use std::collections::HashMap;
use std::ops::Add;
use std::time::Duration;

// --- signals & traces -------------------------------------------------------

/// A point in time, expressed as an offset from an implicit trace epoch.
///
/// WHY vs stl_demo: `stl_demo` uses `chrono::DateTime<Utc>`, a
/// wall-clock type pulled in as a third-party dependency. STL only
/// cares about offsets from the trace start, so wall-clock semantics
/// are noise. Wrapping a `Duration` in a distinct `Timestamp` newtype
/// also lets the type system separate "when a sample was taken" from
/// "how long a formula window is": `t + window_a` typechecks,
/// `t1 + t2` does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(Duration);

impl Timestamp {
    pub const ZERO: Self = Self(Duration::ZERO);
    pub const fn from_millis(ms: u64) -> Self {
        Self(Duration::from_millis(ms))
    }
    pub const fn from_secs(s: u64) -> Self {
        Self(Duration::from_secs(s))
    }
    pub const fn as_secs(self) -> u64 {
        self.0.as_secs()
    }
}

impl Add<Duration> for Timestamp {
    type Output = Self;
    fn add(self, d: Duration) -> Self {
        Self(self.0 + d)
    }
}

/// A sample is a timestamped value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub t: Timestamp,
    pub v: f64,
}

/// A signal is a sequence of samples, ordered by time. We assume
/// zero-order hold semantics, so the value of the signal at any time t
/// is the value of the most recent sample at or before t.
///
/// WHY vs stl_demo: `stl_demo` wraps the Vec in a
/// `pub struct Signal(pub Vec<Sample>)` newtype. The newtype enforced
/// no invariant, so every call site paid for the wrapper with `.0`.
/// A type alias keeps the name and the docs without the friction.
pub type Signal = Vec<Sample>;

/// A trace is a mapping from channel names to signals.
///
/// WHY vs stl_demo: same reasoning as `Signal` — `stl_demo`'s
/// `pub struct Trace(pub HashMap<String, Signal>)` adds noise (`tr.0`)
/// without invariants.
pub type Trace = HashMap<String, Signal>;

/// Returns the value of the signal at time t, using zero-order hold
/// semantics. If t is before the first sample, returns the value of the
/// first sample. If t is after the last sample, returns the value of
/// the last sample. If there are no samples, returns None.
///
/// WHY vs stl_demo: free function over `&[Sample]` instead of a method
/// on the Signal newtype. With `Signal` now an alias, there is no
/// distinct type to hang a method on, and a slice argument accepts
/// either a `&Signal` or any other `&[Sample]` the caller has.
pub fn signal_at(signal: &[Sample], t: Timestamp) -> Option<f64> {
    if signal.is_empty() {
        return None;
    }
    if t < signal[0].t {
        return Some(signal[0].v);
    }
    // First index strictly after t; ZOH value is the one before it.
    let i = signal.partition_point(|s| s.t <= t);
    if i == 0 {
        return Some(signal[0].v);
    }
    Some(signal[i - 1].v)
}

// --- AST --------------------------------------------------------------------

/// The comparison operator for an atomic formula.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Gt,
    Lt,
    Ge,
    Le,
}

/// An STL formula. Atomic comparisons, boolean connectives (Not, And,
/// Or), and the two bounded temporal operators (Always, Eventually).
/// Implication is a derived operator — see [`implies`].
///
/// The robustness of a formula at time t is a real number that
/// indicates how strongly the formula is satisfied or violated at t.
/// A positive value means satisfied, negative means violated, and the
/// magnitude is the signed slack to the threshold.
///
/// WHY vs stl_demo: `stl_demo` models the AST as `trait Formula` with
/// each variant in its own struct and `Box<dyn Formula>` children. A
/// single enum is a better fit here:
///   - the AST is *closed* — we own every variant and never expect a
///     downstream crate to add one
///   - `match` dispatches statically; no vtable, no per-node heap
///     overhead beyond the explicit boxes we keep for recursion
///   - `#[derive(Clone, Debug, PartialEq)]` works on the entire tree;
///     `dyn Formula` cannot derive these
///   - every variant is visible in one place, so adding `Until` later
///     is a one-file edit, not a new module
#[derive(Clone, Debug, PartialEq)]
pub enum Formula {
    /// "channel op bound", e.g. `speed > 60`.
    ///
    /// WHY vs stl_demo: `channel: &'static str` instead of `String`.
    /// Specs are built from string literals, so there is no need to
    /// allocate a fresh `String` per Atom.
    Atom {
        channel: &'static str,
        op: Op,
        bound: f64,
    },
    /// Negation: satisfied iff the child is not. Robustness flips sign.
    Not(Box<Formula>),
    /// Conjunction: robustness is the minimum across children.
    And(Vec<Formula>),
    /// Disjunction: robustness is the maximum across children.
    Or(Vec<Formula>),
    /// `G_[a,b] f` — `f` holds at every sample in `[t+a, t+b]`.
    Always {
        a: Duration,
        b: Duration,
        f: Box<Formula>,
    },
    /// `F_[a,b] f` — `f` holds at some sample in `[t+a, t+b]`.
    Eventually {
        a: Duration,
        b: Duration,
        f: Box<Formula>,
    },
}

impl Formula {
    /// Robustness of this formula at time `t` against `tr`.
    ///
    /// WHY vs stl_demo: one `match` arm per variant instead of one
    /// `impl Formula for ...` block per variant. Same semantics, far
    /// less boilerplate, and the dispatch is direct.
    pub fn robustness(&self, tr: &Trace, t: Timestamp) -> f64 {
        match self {
            Formula::Atom { channel, op, bound } => {
                let Some(sig) = tr.get(*channel) else {
                    return f64::NEG_INFINITY;
                };
                let Some(v) = signal_at(sig, t) else {
                    return f64::NEG_INFINITY;
                };
                // Ge/Le share the math with Gt/Lt: robustness is a
                // signed slack to the threshold, so strict vs.
                // non-strict collapses at the real-valued level.
                match *op {
                    Op::Gt | Op::Ge => v - *bound,
                    Op::Lt | Op::Le => *bound - v,
                }
            }
            Formula::Not(f) => -f.robustness(tr, t),
            Formula::And(children) => children
                .iter()
                .map(|c| c.robustness(tr, t))
                .fold(f64::INFINITY, f64::min),
            Formula::Or(children) => children
                .iter()
                .map(|c| c.robustness(tr, t))
                .fold(f64::NEG_INFINITY, f64::max),
            Formula::Always { a, b, f } => {
                // Vacuously satisfied if no samples fall in the window.
                let win = sample_times_in_window(tr, t + *a, t + *b);
                if win.is_empty() {
                    return f64::INFINITY;
                }
                win.into_iter()
                    .map(|tau| f.robustness(tr, tau))
                    .fold(f64::INFINITY, f64::min)
            }
            Formula::Eventually { a, b, f } => {
                // Vacuously violated if no samples fall in the window.
                let win = sample_times_in_window(tr, t + *a, t + *b);
                if win.is_empty() {
                    return f64::NEG_INFINITY;
                }
                win.into_iter()
                    .map(|tau| f.robustness(tr, tau))
                    .fold(f64::NEG_INFINITY, f64::max)
            }
        }
    }
}

/// `p -> q` is equivalent to `!p \/ q`.
pub fn implies(p: Formula, q: Formula) -> Formula {
    Formula::Or(vec![Formula::Not(Box::new(p)), q])
}

/// Returns a sorted, deduped list of all sample times across every
/// channel in the trace that fall within `[lo, hi]`. Tutorial-grade —
/// production code would cache a merged timeline instead of rebuilding
/// it per call.
fn sample_times_in_window(tr: &Trace, lo: Timestamp, hi: Timestamp) -> Vec<Timestamp> {
    let mut out: Vec<Timestamp> = tr
        .values()
        .flat_map(|sig| sig.iter().map(|s| s.t))
        .filter(|&t| t >= lo && t <= hi)
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Evaluates the robustness of `spec` at each given time and yields
/// the resulting samples.
///
/// WHY vs stl_demo: `stl_demo::evaluate_along` returns a `Signal`
/// (eagerly-built `Vec`). Returning `impl Iterator` lets the caller
/// decide — collect into a Vec, fold into a min/max, or stream to
/// stdout — without paying for an intermediate allocation.
pub fn evaluate_along<'a, I>(
    spec: &'a Formula,
    tr: &'a Trace,
    times: I,
) -> impl Iterator<Item = Sample> + 'a
where
    I: IntoIterator<Item = Timestamp> + 'a,
{
    times.into_iter().map(move |t| Sample {
        t,
        v: spec.robustness(tr, t),
    })
}

// --- main -------------------------------------------------------------------

fn main() {
    temps();
    cars();
}

fn cars() {
    println!("Cars example:");
    let ms = Timestamp::from_millis;
    let mut tr = Trace::new();
    #[rustfmt::skip]
    let speed = vec![
        // phase 1 — speeding, no brake
        Sample { t: ms(0),    v: 70.0 },
        Sample { t: ms(300),  v: 70.0 },
        Sample { t: ms(600),  v: 70.0 },
        // phase 2 — compliant
        Sample { t: ms(1000), v: 70.0 }, // late brake response arrives
        Sample { t: ms(2000), v: 50.0 }, // back under the limit
        Sample { t: ms(3000), v: 60.0 }, // speeding up
        Sample { t: ms(3200), v: 70.0 },
        Sample { t: ms(4000), v: 80.0 }, // speeding again
        // phase 3 — speeding, brake stays under
        Sample { t: ms(5000), v: 70.0 },
        Sample { t: ms(5300), v: 70.0 },
        Sample { t: ms(5500), v: 70.0 },
        Sample { t: ms(6000), v: 40.0 },
    ];
    #[rustfmt::skip]
    let brake = vec![
        // phase 1
        Sample { t: ms(0),    v: 0.0  },
        Sample { t: ms(300),  v: 0.0  },
        Sample { t: ms(600),  v: 0.0  },
        // phase 2
        Sample { t: ms(1000), v: 0.9  }, // arrives just outside [0,500ms]
        Sample { t: ms(2000), v: 0.0  },
        Sample { t: ms(3000), v: 0.0  },
        Sample { t: ms(3200), v: 0.9  }, // responds within 500ms of t=3s
        Sample { t: ms(4000), v: 0.9  },
        // phase 3 — brake reaches for 0.5 but never crosses
        Sample { t: ms(5000), v: 0.30 },
        Sample { t: ms(5300), v: 0.40 },
        Sample { t: ms(5500), v: 0.45 },
        Sample { t: ms(6000), v: 0.0  },
    ];
    tr.insert("speed".to_string(), speed);
    tr.insert("brake".to_string(), brake);

    // "Always between 0 and 1 s, if speed > 60 then eventually within 500 ms brake > 0.5"
    let spec = Formula::Always {
        a: Duration::ZERO,
        b: Duration::from_millis(1000),
        f: Box::new(implies(
            Formula::Atom {
                channel: "speed",
                op: Op::Gt,
                bound: 60.0,
            },
            Formula::Eventually {
                a: Duration::ZERO,
                b: Duration::from_millis(500),
                f: Box::new(Formula::Atom {
                    channel: "brake",
                    op: Op::Gt,
                    bound: 0.5,
                }),
            },
        )),
    };

    for sm in evaluate_along(&spec, &tr, (0..10).map(Timestamp::from_secs)) {
        println!("t={:>2}s  robustness={:+.2}", sm.t.as_secs(), sm.v);
    }
}

fn temps() {
    println!("Temperature example:");

    // Temperature ramps 20.0 → 33.5 °C over 10 s.
    // Crosses the 30 °C threshold at t = 7 s.
    let temp: Signal = (0..10u64)
        .map(|i| Sample {
            t: Timestamp::from_secs(i),
            v: 20.0 + (i as f64) * 1.5,
        })
        .collect();

    let mut tr = Trace::new();
    tr.insert("temp".to_string(), temp);

    let safe = Formula::Atom {
        channel: "temp",
        op: Op::Lt,
        bound: 30.0,
    };

    // "For the next 5 s, temp stays below 30 °C."
    let always_safe = Formula::Always {
        a: Duration::ZERO,
        b: Duration::from_secs(5),
        f: Box::new(safe),
    };

    for sm in evaluate_along(&always_safe, &tr, (0..10).map(Timestamp::from_secs)) {
        println!("t={:>2}s  robustness={:+.2}", sm.t.as_secs(), sm.v);
    }
}

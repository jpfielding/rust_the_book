use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// A sample is a timestamped value.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub t: DateTime<Utc>,
    pub v: f64,
}

/// A signal is a sequence of samples, ordered by time. We assume zero-order hold semantics, so the value of the signal at any time t is the value of the most recent sample at or before t.
pub struct Signal(pub Vec<Sample>);

/// A trace is a mapping from channel names to signals.
pub struct Trace(pub HashMap<String, Signal>);

/// Returns the value of the signal at time t, using zero-order hold semantics. If t is before the first sample, returns the value of the first sample. If t is after the last sample, returns the value of the last sample. If there are no samples, returns None.
impl Signal {
    pub fn at(&self, t: DateTime<Utc>) -> Option<f64> {
        let s = &self.0;
        if s.is_empty() {
            return None;
        }
        if t < s[0].t {
            return Some(s[0].v);
        }
        // First index strictly after t; ZOH value is the one before it.
        let i = s.partition_point(|s| s.t <= t);
        if i == 0 {
            return Some(s[0].v);
        }
        Some(s[i - 1].v)
    }
}

/// The comparison operator for an atomic formula.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Gt,
    Lt,
    Ge,
    Le,
}

/// An STL formula is a closed, recursive syntax tree. The variants cover atomic
/// comparisons, the boolean connectives (negation, conjunction, disjunction), and
/// the bounded temporal operators (always, eventually). Implication is a derived
/// operator built from negation and disjunction (see [`implies`]).
///
/// Recursive variants box their children so the enum has a finite size.
#[derive(Clone, Debug)]
pub enum Formula {
    /// `channel op bound`, e.g. `speed > 60`.
    Atom { channel: String, op: Op, bound: f64 },
    /// Logical negation of a subformula.
    Not(Box<Formula>),
    /// Conjunction: satisfied iff every child is satisfied.
    And(Vec<Formula>),
    /// Disjunction: satisfied iff at least one child is satisfied.
    Or(Vec<Formula>),
    /// `always` over the relative window `[a, b]`: the subformula must hold at
    /// every sample time in the window.
    Always {
        a: Duration,
        b: Duration,
        f: Box<Formula>,
    },
    /// `eventually` over the relative window `[a, b]`: the subformula must hold at
    /// some sample time in the window.
    Eventually {
        a: Duration,
        b: Duration,
        f: Box<Formula>,
    },
}

impl Formula {
    /// The robustness of a formula at time `t` is a real number indicating how
    /// strongly the formula is satisfied or violated there. Positive means
    /// satisfied, negative means violated, and the magnitude is the margin.
    ///
    /// - **Atom**: signed distance of the signal value to the threshold. A missing
    ///   channel or sample yields `-inf` (definitively violated).
    /// - **Not**: negation of the child's robustness.
    /// - **And**: minimum over the children (weakest link).
    /// - **Or**: maximum over the children (strongest witness).
    /// - **Always**: minimum of the subformula over the sample times in the window;
    ///   `+inf` if the window is empty (vacuously satisfied).
    /// - **Eventually**: maximum of the subformula over the sample times in the
    ///   window; `-inf` if the window is empty.
    pub fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        match self {
            Formula::Atom { channel, op, bound } => {
                let Some(sig) = tr.0.get(channel) else {
                    return f64::NEG_INFINITY;
                };
                let Some(v) = sig.at(t) else {
                    return f64::NEG_INFINITY;
                };
                match op {
                    Op::Gt | Op::Ge => v - bound,
                    Op::Lt | Op::Le => bound - v,
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
                let times = tr.sample_times_in_window(t + *a, t + *b);
                if times.is_empty() {
                    return f64::INFINITY;
                }
                times
                    .into_iter()
                    .map(|tau| f.robustness(tr, tau))
                    .fold(f64::INFINITY, f64::min)
            }
            Formula::Eventually { a, b, f } => {
                let times = tr.sample_times_in_window(t + *a, t + *b);
                if times.is_empty() {
                    return f64::NEG_INFINITY;
                }
                times
                    .into_iter()
                    .map(|tau| f.robustness(tr, tau))
                    .fold(f64::NEG_INFINITY, f64::max)
            }
        }
    }
}

/// `p -> q` is equivalent to `!p || q`.
pub fn implies(p: Formula, q: Formula) -> Formula {
    Formula::Or(vec![Formula::Not(Box::new(p)), q])
}

/// Returns a sorted list of all sample times in the trace that are within the given time window.
impl Trace {
    pub fn sample_times_in_window(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<DateTime<Utc>> {
        let mut out: Vec<DateTime<Utc>> = self
            .0
            .values()
            .flat_map(|sig| sig.0.iter().map(|s| s.t))
            .filter(|&t| t >= from && t <= to)
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

/// Evaluates the robustness of the formula at all sample times in the trace, and returns a signal of the robustness
/// values at those times.
pub fn evaluate_along(spec: &Formula, tr: &Trace, times: &[DateTime<Utc>]) -> Signal {
    let samples = times
        .iter()
        .map(|&t| Sample {
            t,
            v: spec.robustness(tr, t),
        })
        .collect();
    Signal(samples)
}

fn main() {
    temps();
    cars();
}

fn cars() {
    println!("Cars example:");
    let t0 = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
    let ms = |n: i64| t0 + Duration::milliseconds(n);
    let mut tr = Trace { 0: HashMap::new() };
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
    tr.0.insert("speed".to_string(), Signal(speed));
    tr.0.insert("brake".to_string(), Signal(brake));

    // "Always between 0 and 1 s, if speed > 60 then eventually within 500 ms brake > 0.5"
    let spec = Formula::Always {
        a: Duration::zero(),
        b: Duration::milliseconds(1000),
        f: Box::new(implies(
            Formula::Atom {
                channel: "speed".into(),
                op: Op::Gt,
                bound: 60.0,
            },
            Formula::Eventually {
                a: Duration::zero(),
                b: Duration::milliseconds(500),
                f: Box::new(Formula::Atom {
                    channel: "brake".into(),
                    op: Op::Gt,
                    bound: 0.5,
                }),
            },
        )),
    };
    let sec = |n: i64| t0 + Duration::seconds(n);
    let times: Vec<_> = (0..10).map(sec).collect();
    let r = evaluate_along(&spec, &tr, &times);

    for sm in &r.0 {
        println!("t={:>2}s  robustness={:+.2}", sm.t.timestamp(), sm.v);
    }
}

fn temps() {
    println!("Temperature example:");
    let t0 = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
    let sec = |n: i64| t0 + Duration::seconds(n);

    // Temperature ramps 20.0 → 33.5 °C over 10 s.
    // Crosses the 30 °C threshold at t = 7 s.
    let temp = Signal(
        (0..10)
            .map(|i| Sample {
                t: sec(i),
                v: 20.0 + (i as f64) * 1.5,
            })
            .collect(),
    );

    let mut tr = Trace(HashMap::new());
    tr.0.insert("temp".into(), temp);

    let safe = Formula::Atom {
        channel: "temp".into(),
        op: Op::Lt,
        bound: 30.0,
    };

    // "For the next 5 s, temp stays below 30 °C."
    let always_safe = Formula::Always {
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

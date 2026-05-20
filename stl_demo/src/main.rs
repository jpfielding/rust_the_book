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

/// An STL formula is either an atomic formula, a negation, a conjunction, or a disjunction. We can also define implication as a derived operator.
#[derive(Clone, Copy, Debug, PartialEq)]

/// The comparison operator for an atomic formula.
pub enum Op {
    Gt,
    Lt,
    Ge,
    Le,
}

/// The robustness of a formula at time t is a real number that indicates how strongly the formula is satisfied or violated at time t. A positive robustness means the formula is satisfied, a negative robustness means it is violated, and the magnitude indicates how strongly it is satisfied or violated.
pub trait Formula {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64;
}

/// An atomic formula is of the form "channel op bound", where op is a comparison operator (>, <, >=, <=) and bound is a real number. The robustness of an atomic formula is the distance of the signal value to the threshold, with the sign determined by whether the formula is satisfied or not.
pub struct Atom {
    pub channel: String,
    pub op: Op,
    pub bound: f64,
}

/// Robustness of an atomic formula is the distance of the signal value to the threshold, with the sign determined by whether the formula is satisfied or not.
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

/// Negation of a formula is satisfied if and only if the original formula is not satisfied, so the robustness of a negation is the negative of the robustness of the negated formula.
pub struct Not {
    pub f: Box<dyn Formula>,
}

/// Robustness of negation is the negative of the robustness of the negated formula.
impl Formula for Not {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        -self.f.robustness(tr, t)
    }
}

/// A conjunction is satisfied if and only if all of its conjuncts are satisfied, so the robustness of a conjunction is the minimum of the robustness of its conjuncts.
pub struct And {
    pub children: Vec<Box<dyn Formula>>,
}

/// Robustness of conjunction is the minimum of the robustness of the conjuncts.
impl Formula for And {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        self.children
            .iter()
            .map(|c| c.robustness(tr, t))
            .fold(f64::INFINITY, f64::min)
    }
}

/// A disjunction is satisfied if and only if at least one of its disjuncts is satisfied, so the robustness of a disjunction is the maximum of the robustness of its disjuncts.
pub struct Or {
    pub children: Vec<Box<dyn Formula>>,
}

/// Robustness of an OR is the max of the robustness of its children.
impl Formula for Or {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        self.children
            .iter()
            .map(|c| c.robustness(tr, t))
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

/// p -> q is equivalent to !p or q
pub fn implies(p: Box<dyn Formula>, q: Box<dyn Formula>) -> Or {
    Or {
        children: vec![Box::new(Not { f: p }), q],
    }
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

/// An always formula is satisfied if and only if the subformula is satisfied at all times in the given time window,
/// so the robustness of an always formula is the minimum of the robustness of the subformula at all times in the time window.
pub struct Always {
    pub a: Duration,
    pub b: Duration,
    pub f: Box<dyn Formula>,
}

/// Robustness of an always formula is the minimum of the robustness of the subformula at all times in the time window.
/// We can compute this by first finding all sample times in the trace that are within the time window, and then taking
/// the minimum robustness of the subformula at those times. If there are no sample times in the time window, we can
/// return positive infinity, since the formula is vacuously satisfied.
impl Formula for Always {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        let times = tr.sample_times_in_window(t + self.a, t + self.b);
        if times.is_empty() {
            return f64::INFINITY;
        }
        times
            .into_iter()
            .map(|tau| self.f.robustness(tr, tau))
            .fold(f64::INFINITY, f64::min)
    }
}

/// An eventually formula is satisfied if and only if the subformula is satisfied at some time in the given time window,
/// so the robustness of an eventually formula is the maximum of the robustness of the subformula at all times in the time window.
pub struct Eventually {
    pub a: Duration,
    pub b: Duration,
    pub f: Box<dyn Formula>,
}

/// Robustness of an eventually formula is the maximum of the robustness of the subformula at all times in the time window.
impl Formula for Eventually {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        let times = tr.sample_times_in_window(t + self.a, t + self.b);
        if times.is_empty() {
            return f64::NEG_INFINITY;
        }
        times
            .into_iter()
            .map(|tau| self.f.robustness(tr, tau))
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

/// Evaluates the robustness of the formula at all sample times in the trace, and returns a signal of the robustness
/// values at those times.
pub fn evaluate_along(spec: &dyn Formula, tr: &Trace, times: &[DateTime<Utc>]) -> Signal {
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
    let spec = Always {
        a: Duration::zero(),
        b: Duration::milliseconds(1000),
        f: Box::new(implies(
            Box::new(Atom {
                channel: "speed".into(),
                op: Op::Gt,
                bound: 60.0,
            }),
            Box::new(Eventually {
                a: Duration::zero(),
                b: Duration::milliseconds(500),
                f: Box::new(Atom {
                    channel: "brake".into(),
                    op: Op::Gt,
                    bound: 0.5,
                }),
            }),
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

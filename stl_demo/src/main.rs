use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub t: DateTime<Utc>,
    pub v: f64,
}

#[derive(Default)]
pub struct Signal(pub Vec<Sample>);

#[derive(Default)]
pub struct Trace(pub HashMap<String, Signal>);

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

impl Signal {
    pub fn at(&self, t: DateTime<Utc>) -> Option<f64>{
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

pub enum Op { Gt, Lt, Ge, Le }

pub trait Formula {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64;
}

// ─── §3  Atomic Predicates ─────────────────────────────────────────────────

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

// ─── §4  Boolean Operators ─────────────────────────────────────────────────

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

// ─── §5  Temporal Operators ────────────────────────────────────────────────

pub struct Always {
    pub a: Duration,
    pub b: Duration,
    pub f: Box<dyn Formula>,
}

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

pub struct Eventually {
    pub a: Duration,
    pub b: Duration,
    pub f: Box<dyn Formula>,
}

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

// ─── §6  Robustness as a Signal ────────────────────────────────────────────

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

// ─── Demo ──────────────────────────────────────────────────────────────────

fn main() {
    let t0 = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
    let sec = |n: i64| t0 + Duration::seconds(n);

    // Temperature ramps 20.0 → 33.5 over 10s, crossing the 30°C threshold at t=7.
    let temp = Signal(
        (0..10)
            .map(|i| Sample { t: sec(i), v: 20.0 + (i as f64) * 1.5 })
            .collect(),
    );

    let mut tr = Trace::default();
    tr.0.insert("temp".into(), temp);

    let safe = Atom { channel: "temp".into(), op: Op::Lt, bound: 30.0 };

    // "For the next 5s, temp stays below 30°C."
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

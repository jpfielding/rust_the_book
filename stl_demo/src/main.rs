use std::collections::HashMap;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub t: DateTime<Utc>,
    pub v: f64,
}

pub struct Signal(pub Vec<Sample>);

pub struct Trace(pub HashMap<String, Signal>);

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

pub enum Op { Gt, Lt, Ge, Le }

pub trait Formula {
    fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64;
}

fn main() {
    println!("Hello, world!");
}

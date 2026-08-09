use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "stl_blind", about = "writing this without any tutorial")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    SpeedBrakes {
        #[arg(short, long, default_value = "70", env = "SPEED")]
        speed: f64,
        #[arg(short, long, default_value = "0.5", env = "BRAKES")]
        brakes: f64,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::SpeedBrakes { speed, brakes } => {
            speed_brakes(speed, brakes).map_err(|e| format!("speed and brakes: {e}"))
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn speed_brakes(_sb: f64, _bb: f64) -> Result<(), String> {
    let t0 = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
    let ms = |n: i64| t0 + chrono::Duration::milliseconds(n);
    #[rustfmt::skip]
    let _speed = vec![
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
    let _brake = vec![
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

    let spec = Formula::Historically {
        a: Duration::milliseconds(0),
        b: Duration::milliseconds(500),
        f: Box::new(Formula::Implies(
            Box::new(Formula::Atom {
                channel: "speed".to_string(),
                op: Op::GT,
                value: 65.0,
            }),
            Box::new(Formula::Atom {
                channel: "brake".to_string(),
                op: Op::GE,
                value: 0.5,
            }),
        )),
    };
    let sec = |n: i64| t0 + chrono::Duration::seconds(n);
    let times: Vec<DateTime<Utc>> = (0..=6).map(sec).collect();
    let trace = Trace(HashMap::from([
        ("speed".to_string(), Signal(_speed)),
        ("brake".to_string(), Signal(_brake)),
    ]));
    for t in times {
        let r = spec.robustness(&trace, t);
        println!("robustness at {t:?} = {r}");
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub t: DateTime<Utc>,
    pub v: f64,
}

/// A signal is a series of samples, each with a timestamp and a value.
#[derive(Debug, Clone)]
pub struct Signal(pub Vec<Sample>);

/// zoh (zero-order hold) interpolation: the value of the signal at time t is the value of the most recent sample before t.
pub fn at(s: &Signal, t: DateTime<Utc>) -> Option<f64> {
    let i = s.0.partition_point(|s| s.t <= t);
    if i == 0 {
        return None;
    }
    Some(s.0[i - 1].v)
}

#[derive(Debug, Clone)]
pub struct Trace(pub HashMap<String, Signal>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    GT,
    LT,
    GE,
    LE,
}

pub enum Formula {
    /// the most basic leaf of this recursion
    Atom {
        channel: String,
        op: Op,
        value: f64,
    },
    Not(Box<Formula>),
    /// ops across forumlas
    And(Vec<Formula>),
    Or(Vec<Formula>),
    Implies(Box<Formula>, Box<Formula>), // A implies B is equivalent to !A or B
    /// temporal operators: past-time
    Historically {
        a: Duration,
        b: Duration,
        f: Box<Formula>,
    }, // historically, always in the past
    Once {
        a: Duration,
        b: Duration,
        f: Box<Formula>,
    }, // once, at least once in the past
    Since {
        a: Duration,
        b: Duration,
        f: Box<Formula>,
        g: Box<Formula>,
    }, // since, always in the past from some point
}

impl Formula {
    /// assuming that Trace only contains the channels that are used in the formula, compute the robustness of the formula at time t.
    pub fn robustness(&self, tr: &Trace, t: DateTime<Utc>) -> f64 {
        match self {
            Formula::Atom { channel, op, value } => {
                let Some(sig) = tr.0.get(channel) else {
                    return f64::NEG_INFINITY;
                };
                let Some(v) = at(sig, t) else {
                    return f64::NEG_INFINITY;
                };
                match op {
                    Op::GT | Op::GE => v - value,
                    Op::LT | Op::LE => value - v,
                }
            }
            Formula::Not(f) => -f.robustness(tr, t),
            Formula::And(children) => children
                .iter()
                .map(|f| f.robustness(tr, t))
                .fold(f64::INFINITY, f64::min),
            Formula::Or(children) => children
                .iter()
                .map(|f| f.robustness(tr, t))
                .fold(f64::NEG_INFINITY, f64::max),
            Formula::Implies(a, b) => {
                let ra = a.robustness(tr, t);
                let rb = b.robustness(tr, t);
                f64::max(-ra, rb)
            }
            // min robustness over all times in the window
            Formula::Historically { a, b, f } => {
                inf_over(tr, t + *a, t + *b, |t| f.robustness(tr, t))
            }
            // max robustness over all times in the window
            Formula::Once { a, b, f } => sup_over(tr, t + *a, t + *b, |t| f.robustness(tr, t)),
            // f->g robustness over all times in the window
            Formula::Since { a, b, f, g } => inf_over(tr, t + *a, t + *b, |t| {
                let rf = f.robustness(tr, t);
                let rg = g.robustness(tr, t);
                f64::min(rf, rg)
            }),
        }
    }
}

fn inf_over<F>(tr: &Trace, lo: DateTime<Utc>, hi: DateTime<Utc>, mut f: F) -> f64
where
    F: Fn(DateTime<Utc>) -> f64,
{
    let mut times = samples_in_trace_window(tr.clone(), lo, hi);
    if times.first() != Some(&lo) {
        times.insert(0, lo);
    }
    times.into_iter().map(&mut f).fold(f64::INFINITY, f64::min)
}

fn sup_over<F>(tr: &Trace, lo: DateTime<Utc>, hi: DateTime<Utc>, mut f: F) -> f64
where
    F: Fn(DateTime<Utc>) -> f64,
{
    let mut times = samples_in_trace_window(tr.clone(), lo, hi);
    if times.first() != Some(&lo) {
        times.insert(0, lo);
    }
    times
        .into_iter()
        .map(&mut f)
        .fold(f64::NEG_INFINITY, f64::max)
}

/// sample times within the window
pub fn samples_in_trace_window(
    tr: Trace,
    lo: DateTime<Utc>,
    hi: DateTime<Utc>,
) -> Vec<DateTime<Utc>> {
    let mut times = Vec::new();
    for sig in tr.0.values() {
        let start = sig.0.partition_point(|sample| sample.t < lo);
        let end = sig.0.partition_point(|sample| sample.t <= hi);
        times.extend(sig.0[start..end].iter().map(|sample| sample.t));
    }
    times.sort_unstable();
    times.dedup();
    times
}

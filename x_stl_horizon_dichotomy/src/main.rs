//! Stripped-down STL robustness model: one signal, one window width, and the
//! SAME timesteps evaluated through a past window and a future window.
//!
//! The point this file makes concrete:
//!   A temporal operator is nothing but a fold (here, `min`) over a *signed*
//!   offset window `[t+a, t+b]` hung off an evaluation cursor `t`.
//!       historically[0,W] phi  ==  MinWindow{ a:-W, b: 0 }   (window behind t)
//!       always[0,W]       phi  ==  MinWindow{ a: 0, b:+W }   (window ahead of t)
//!   The ONLY difference between "past" and "future" is the sign of the offsets.
//!
//! Online evaluation has a second cursor: the data wavefront `now` (the latest
//! sample that has arrived). A verdict for cursor `t` is DECIDABLE only once
//! `now` has reached the furthest-future sample the window needs, i.e.
//! `now >= t + forward_horizon`. A past window (b <= 0) is always decidable at
//! now = t. A future window (b > 0) must wait `b` steps.
//!
//! Run:  rustc -O stl_horizons_demo.rs && ./stl_horizons_demo
//! (no external crates; pastes straight into the Rust Playground)
// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// Quantitative verdict. `Some(r)`: decided, sign = satisfied/violated, |r| =
/// robustness margin. `None`: UNKNOWN — a sample the window needs has not yet
/// arrived at the wavefront. (This is "wait", never "guess".)
type Rho = Option<f64>;

enum Formula {
    /// Atom `signal >= threshold`. Robustness = value - threshold.
    Atom { threshold: f64 },
    /// Boolean AND = min of children's robustness (STL quantitative semantics).
    And(Box<Formula>, Box<Formula>),
    /// THE temporal primitive: min of `inner` over the offset window
    /// [t+a, t+b]. a<=0,b<=0 is a past op; a>=0,b>=0 is a future op.
    MinWindow { a: i64, b: i64, inner: Box<Formula> },
}

/// Forward horizon H: how far past `t` any future sample is needed. Pure
/// function of the AST (max over nodes of `max(0, b)`).
fn forward_horizon(f: &Formula) -> i64 {
    match f {
        Formula::Atom { .. } => 0,
        Formula::And(l, r) => forward_horizon(l).max(forward_horizon(r)),
        Formula::MinWindow { b, inner, .. } => (*b).max(0).max(forward_horizon(inner)),
    }
}

/// Past horizon: how far behind `t` any sample is needed. `max(0, -a)`.
fn past_horizon(f: &Formula) -> i64 {
    match f {
        Formula::Atom { .. } => 0,
        Formula::And(l, r) => past_horizon(l).max(past_horizon(r)),
        Formula::MinWindow { a, inner, .. } => (-*a).max(0).max(past_horizon(inner)),
    }
}

/// Robustness of `f` at verdict cursor `t`, given the data wavefront `now`.
///
/// The decidability gate lives in the atom: any required sample at an index
/// `> now` (the future hasn't happened yet) returns UNKNOWN, and `?` propagates
/// that up through every fold. So a window blocks on its furthest-future cell.
fn rho(f: &Formula, x: &[f64], t: i64, now: i64) -> Rho {
    match f {
        Formula::Atom { threshold } => {
            if t < 0 || t >= x.len() as i64 {
                return None; // outside the (finite) trace
            }
            if t > now {
                return None; // beyond the wavefront: not yet observed
            }
            Some(x[t as usize] - threshold)
        }
        Formula::And(l, r) => {
            let a = rho(l, x, t, now)?;
            let b = rho(r, x, t, now)?;
            Some(a.min(b))
        }
        Formula::MinWindow { a, b, inner } => {
            let mut acc: Rho = None;
            for s in (t + a)..=(t + b) {
                let r = rho(inner, x, s, now)?; // any pending cell -> whole window pending
                acc = Some(acc.map_or(r, |m: f64| m.min(r)));
            }
            acc
        }
    }
}

// ---------------------------------------------------------------------------
// Pretty-printing
// ---------------------------------------------------------------------------

fn verdict(r: Rho) -> String {
    match r {
        None => "  ……  PENDING (future not yet observed)".to_string(),
        Some(v) if v >= 0.0 => format!("rho={:+.0}  SAT", v),
        Some(v) => format!("rho={:+.0}  VIOL", v),
    }
}

fn atom(threshold: f64) -> Box<Formula> {
    Box::new(Formula::Atom { threshold })
}

fn main() {
    // One signal, sampled at integer timesteps t = 0..=10. Atom: x >= 0
    // ("inside the safety envelope"). The signal dips out of the envelope
    // around t=5..6 and recovers.
    //  t:   0  1  2  3  4   5   6  7  8  9 10
    let x: Vec<f64> = vec![2., 2., 2., 2., 1., -2., -1., 1., 2., 2., 2.];
    let w: i64 = 3;

    // SAME window width, SAME source data, opposite sign on the offsets.
    let hist = Formula::MinWindow {
        a: -w,
        b: 0,
        inner: atom(0.0),
    }; // historically[0,3]
    let alw = Formula::MinWindow {
        a: 0,
        b: w,
        inner: atom(0.0),
    }; //  always[0,3]
    let both = Formula::And(
        Box::new(Formula::MinWindow {
            a: -w,
            b: 0,
            inner: atom(0.0),
        }),
        Box::new(Formula::MinWindow {
            a: 0,
            b: w,
            inner: atom(0.0),
        }),
    );

    println!("SIGNAL  (atom: x >= 0,  window width W = {w})");
    print!("  t   : ");
    for t in 0..x.len() {
        print!("{:>4}", t);
    }
    println!();
    print!("  x[t]: ");
    for v in &x {
        print!("{:>4}", *v as i64);
    }
    println!("\n");

    println!("FORMULAS  (one primitive, the sign of the offsets is the only difference)");
    println!(
        "  HIST = MinWindow{{a:-{w}, b:0 }}   ==  historically[0,{w}](x>=0)   past_horizon={}  fwd_horizon={}",
        past_horizon(&hist),
        forward_horizon(&hist)
    );
    println!(
        "  ALW  = MinWindow{{a:0,  b:+{w}}}   ==  always[0,{w}](x>=0)         past_horizon={}  fwd_horizon={}",
        past_horizon(&alw),
        forward_horizon(&alw)
    );
    println!(
        "  BOTH = HIST AND ALW                                          past_horizon={}  fwd_horizon={}",
        past_horizon(&both),
        forward_horizon(&both)
    );
    println!("\n  A verdict for cursor t is decidable when  now >= t + fwd_horizon.");
    println!(
        "  HIST: fwd=0  -> decidable at now=t.   ALW/BOTH: fwd={w} -> decidable at now=t+{w}.\n"
    );

    // -------------------------------------------------------------------
    // Part 1: the wavefront walk. One line per arriving sample. For each
    // monitor, the FRONTIER it can newly speak to.
    // -------------------------------------------------------------------
    println!("=== PART 1: wavefront walk — what each monitor can commit as samples arrive ===\n");
    for now in 0..x.len() as i64 {
        println!(
            "now = {now}   (sample x[{now}] = {:+} just arrived)",
            x[now as usize] as i64
        );
        // HIST frontier: latest t with now >= t  -> t = now.
        let t_h = now;
        println!(
            "   HIST commits t={t_h:>2}: {}",
            verdict(rho(&hist, &x, t_h, now))
        );
        // BOTH/ALW frontier: latest t with now >= t + W -> t = now - W.
        let t_b = now - w;
        if t_b >= 0 {
            println!(
                "   BOTH commits t={t_b:>2}: {}",
                verdict(rho(&both, &x, t_b, now))
            );
        } else {
            println!("   BOTH commits t=--: ……  (nothing decidable yet; needs {w} more samples)");
        }
        // The shared-window gap: timesteps whose PAST half is known but whose
        // FUTURE half is still pending. These are the cells that "break".
        let lo = (now - w + 1).max(0);
        if lo <= now {
            let gap: Vec<String> = (lo..=now).map(|t| t.to_string()).collect();
            println!(
                "   gap (past-known, future-pending): t in {{{}}}",
                gap.join(",")
            );
        }
        println!();
    }

    // -------------------------------------------------------------------
    // Part 2: the break, focused. Take one cursor and show that the
    // pure-past monitor commits an answer that the shared-window monitor
    // later CONTRADICTS — same t, same data, different time horizon.
    // -------------------------------------------------------------------
    println!("=== PART 2: the break — same timestep, answer flips by horizon ===\n");
    for &t in &[3i64, 4, 7] {
        println!("cursor t = {t}:");
        // What a pure-past monitor says, the instant it can (now = t):
        println!(
            "   pure-past  HIST @ now={t:>2}: {}",
            verdict(rho(&hist, &x, t, t))
        );
        // What the future window says, the instant it can (now = t+W):
        println!(
            "   pure-future ALW @ now={:>2}: {}",
            t + w,
            verdict(rho(&alw, &x, t, t + w))
        );
        // What the shared-window rule says, the instant it can (now = t+W):
        println!(
            "   shared     BOTH @ now={:>2}: {}",
            t + w,
            verdict(rho(&both, &x, t, t + w))
        );
        // And prove BOTH is genuinely UNKNOWN at now=t (can't be emitted early):
        println!(
            "   shared     BOTH @ now={t:>2}: {}   <- cannot commit when the past monitor did",
            verdict(rho(&both, &x, t, t))
        );
        println!();
    }

    // -------------------------------------------------------------------
    // Part 3: final ledger. For every shared-window cursor, the past verdict
    // (and when it committed), the future verdict (and when), and whether a
    // single scalar at one wall-clock moment can represent the cell.
    // -------------------------------------------------------------------
    println!("=== PART 3: ledger over the shared-window cursors ===\n");
    println!(
        "  {:>3} | {:>14} @now | {:>14} @now | shared (now=t+{w}) | note",
        "t", "HIST (past)", "ALW (future)"
    );
    println!("  {}", "-".repeat(78));
    for t in w..=(x.len() as i64 - 1 - w) {
        let rh = rho(&hist, &x, t, t); // committed at now=t
        let ra = rho(&alw, &x, t, t + w); // committed at now=t+W
        let rb = rho(&both, &x, t, t + w);
        let sign = |r: Rho| r.map(|v| v >= 0.0);
        let note = match (sign(rh), sign(ra)) {
            (Some(true), Some(false)) => "past SAT but future VIOL — early SAT is a lie",
            (Some(false), Some(true)) => "past VIOL but future SAT — recovering",
            (Some(a), Some(b)) if a == b => "agree",
            _ => "",
        };
        println!(
            "  {:>3} | {:>14} @{:>2} | {:>14} @{:>2} | {:>13} | {note}",
            t,
            short(rh),
            t,
            short(ra),
            t + w,
            short(rb),
        );
    }
    println!(
        "\n  Takeaway: HIST(t) and the shared rule's verdict for the SAME t are\n  committed at different wall-clock moments (now=t vs now=t+{w}) and can carry\n  OPPOSITE signs. A monitor that emits one scalar per cursor must therefore\n  pick a single frontier — it cannot honor a past horizon and a future horizon\n  on the same emission without either lagging to the future horizon (BOTH) or\n  splitting into separate streams. That is the whole 'mode' question, in 130 lines."
    );
}

fn short(r: Rho) -> String {
    match r {
        None => "PEND".to_string(),
        Some(v) if v >= 0.0 => format!("{:+.0} SAT", v),
        Some(v) => format!("{:+.0} VIOL", v),
    }
}

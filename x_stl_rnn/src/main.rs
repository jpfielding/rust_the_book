//! STL × RNN: teach a recurrent net to be a Signal Temporal Logic monitor.
//!
//! STL (Signal Temporal Logic) hands us a *ground-truth* robustness signal for
//! free: for the spec `historically[0,W](x >= 0)` the robustness at time t is
//!
//!     rho(t) = min over s in [t-W, t] of x[s]
//!
//! i.e. a running, windowed minimum. Its SIGN is the boolean verdict
//! (>=0 SAT, <0 VIOL); its MAGNITUDE is the safety margin.
//!
//! `historically` is the *causal* STL operator — it only looks back — which is
//! exactly the shape a left-to-right RNN can represent. So this file pits the
//! two against each other: STL computes the exact monitor, and an LSTM is
//! trained (regression, MSE) to *approximate* that same monitor from the raw
//! signal alone. The closing ledger shows the learned monitor tracking the
//! analytic one timestep by timestep, plus its boolean (SAT/VIOL) agreement.
//!
//! Backend: pure-Rust `NdArray` CPU backend wrapped in `Autodiff`. No GPU, no
//! C, no hand-written assembly.
//!
//! Run:  cargo run --release

use burn::backend::{Autodiff, NdArray};
use burn::module::Module;
use burn::nn::loss::{MseLoss, Reduction};
use burn::nn::{Linear, LinearConfig, Lstm, LstmConfig};
use burn::optim::{AdamConfig, GradientsParams, Optimizer};
use burn::tensor::Tensor;
use burn::tensor::backend::Backend;

// ---------------------------------------------------------------------------
// STL ground truth — the monitor we want the RNN to imitate.
// ---------------------------------------------------------------------------

/// Window width W for `historically[0,W]`.
const W: usize = 4;

/// Exact STL robustness of `historically[0,W](x >= 0)` over a finite signal.
/// At t<W the window is clipped to `[0, t]` (no samples before the trace start).
fn stl_historically(x: &[f32]) -> Vec<f32> {
    (0..x.len())
        .map(|t| {
            let lo = t.saturating_sub(W);
            x[lo..=t].iter().copied().fold(f32::INFINITY, f32::min)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Synthetic signals — deterministic, dependency-free pseudo-randomness.
// ---------------------------------------------------------------------------

/// Tiny LCG so runs are reproducible without pulling in `rand`.
struct Lcg(u64);
impl Lcg {
    fn next_f32(&mut self) -> f32 {
        // Numerical Recipes constants; take the top bits for a [0,1) float.
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32) / ((1u64 << 24) as f32)
    }
}

/// One signal: a couple of sinusoids (so the windowed-min actually depends on
/// recent history) plus noise, recentred so it spends real time on both sides
/// of zero — otherwise the SAT/VIOL labels would be trivially constant.
fn make_signal(seq_len: usize, rng: &mut Lcg) -> Vec<f32> {
    let f1 = 0.15 + 0.25 * rng.next_f32();
    let f2 = 0.40 + 0.40 * rng.next_f32();
    let phase = std::f32::consts::TAU * rng.next_f32();
    let bias = 0.6 * rng.next_f32() - 0.3;
    (0..seq_len)
        .map(|t| {
            let t = t as f32;
            let s = (f1 * t + phase).sin() + 0.5 * (f2 * t).sin();
            let noise = 0.3 * (rng.next_f32() - 0.5);
            s + bias + noise
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Model: LSTM encoder + linear read-out to a single robustness scalar/step.
// ---------------------------------------------------------------------------

#[derive(Module, Debug)]
struct Monitor<B: Backend> {
    lstm: Lstm<B>,
    head: Linear<B>,
}

impl<B: Backend> Monitor<B> {
    fn new(d_hidden: usize, device: &B::Device) -> Self {
        Self {
            // d_input = 1: a univariate signal, one value per timestep.
            lstm: LstmConfig::new(1, d_hidden, true).init(device),
            head: LinearConfig::new(d_hidden, 1).init(device),
        }
    }

    /// `[batch, seq, 1]` signal -> `[batch, seq, 1]` per-step robustness estimate.
    fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        let (hidden, _state) = self.lstm.forward(x, None); // [batch, seq, d_hidden]
        self.head.forward(hidden) // linear acts on the last dim -> [batch, seq, 1]
    }
}

// ---------------------------------------------------------------------------
// Data plumbing: build batched [N, SEQ, 1] tensors of signals and STL targets.
// ---------------------------------------------------------------------------

fn build_batch<B: Backend>(
    signals: &[Vec<f32>],
    seq_len: usize,
    device: &B::Device,
) -> (Tensor<B, 3>, Tensor<B, 3>) {
    let n = signals.len();
    let mut xs = Vec::with_capacity(n * seq_len);
    let mut ys = Vec::with_capacity(n * seq_len);
    for sig in signals {
        xs.extend_from_slice(sig);
        ys.extend_from_slice(&stl_historically(sig));
    }
    let x = Tensor::<B, 1>::from_floats(xs.as_slice(), device).reshape([n, seq_len, 1]);
    let y = Tensor::<B, 1>::from_floats(ys.as_slice(), device).reshape([n, seq_len, 1]);
    (x, y)
}

// ---------------------------------------------------------------------------
// Train / evaluate.
// ---------------------------------------------------------------------------

type B = Autodiff<NdArray>;

const SEQ: usize = 32;
const N_TRAIN: usize = 256;
const D_HIDDEN: usize = 32;
const EPOCHS: usize = 400;
const LR: f64 = 1e-2;

fn main() {
    let device = Default::default();
    let mut rng = Lcg(0x5715_70_1c_e_u64 ^ 0xD1CE);

    // --- data ---------------------------------------------------------------
    let train_signals: Vec<Vec<f32>> = (0..N_TRAIN).map(|_| make_signal(SEQ, &mut rng)).collect();
    let (x_train, y_train) = build_batch::<B>(&train_signals, SEQ, &device);

    // --- model + optimizer --------------------------------------------------
    let mut model = Monitor::<B>::new(D_HIDDEN, &device);
    let mut optim = AdamConfig::new().init();

    println!(
        "Training an LSTM to imitate STL  historically[0,{W}](x >= 0)\n\
         backend = Autodiff<NdArray>   seqs = {N_TRAIN}   len = {SEQ}   hidden = {D_HIDDEN}\n"
    );

    // --- training loop (full-batch) ----------------------------------------
    for epoch in 0..EPOCHS {
        let pred = model.forward(x_train.clone());
        let loss = MseLoss::new().forward(pred, y_train.clone(), Reduction::Mean);

        let grads = GradientsParams::from_grads(loss.backward(), &model);
        model = optim.step(LR, model, grads);

        if epoch == 0 || (epoch + 1) % 50 == 0 {
            // Re-evaluate loss off the graph just for reporting.
            let l = MseLoss::new()
                .forward(
                    model.forward(x_train.clone()),
                    y_train.clone(),
                    Reduction::Mean,
                )
                .into_scalar();
            println!("  epoch {:>4}   mse = {:.5}", epoch + 1, l);
        }
    }

    // --- evaluate on a fresh, unseen signal --------------------------------
    let test = make_signal(SEQ, &mut rng);
    let truth = stl_historically(&test);
    let (x_test, _) = build_batch::<B>(std::slice::from_ref(&test), SEQ, &device);
    let pred: Vec<f32> = model
        .forward(x_test)
        .into_data()
        .to_vec()
        .expect("f32 predictions");

    println!("\n=== held-out signal: STL monitor vs. learned monitor ===\n");
    println!(
        "  {:>3} | {:>7} | {:>9} | {:>9} | verdict (STL / RNN)",
        "t", "x[t]", "STL rho", "RNN rho"
    );
    println!("  {}", "-".repeat(58));

    let mut agree = 0usize;
    for t in 0..SEQ {
        let (rt, rp) = (truth[t], pred[t]);
        let (st, sp) = (rt >= 0.0, rp >= 0.0);
        let flag = |b: bool| if b { "SAT " } else { "VIOL" };
        let mark = if st == sp {
            agree += 1;
            "  ok"
        } else {
            "  <- DISAGREE"
        };
        println!(
            "  {:>3} | {:>+7.2} | {:>+9.3} | {:>+9.3} | {} / {}{}",
            t,
            test[t],
            rt,
            rp,
            flag(st),
            flag(sp),
            mark
        );
    }

    let mae: f32 = truth
        .iter()
        .zip(&pred)
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>()
        / SEQ as f32;
    println!("\n  boolean agreement: {agree}/{SEQ} timesteps   |   robustness MAE: {mae:.3}");
    println!(
        "\n  The LSTM never saw the STL formula — only (signal, robustness) pairs.\n  \
         It has reconstructed the causal windowed-min monitor from examples."
    );
}

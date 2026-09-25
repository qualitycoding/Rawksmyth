//! Chart-informed constrained-NSDF note verifier (plan step S-005, §2B.1b).
//!
//! Given the *charted* pitch, decide whether the player is sounding it —
//! rather than estimating pitch blindly (A-003). For each expected note:
//!
//! 1. an onset from the detector is attributed to it (greedy, smallest
//!    |onset − expected|; one onset serves at most one note, §2B.1e);
//! 2. from `G_attack` after that onset, every 128-frame hop the normalized
//!    difference a(τ) = d(τ)/m(τ) is evaluated only on lags within ±100 cents
//!    of τ0 = fs/target, the minimum is refined by parabolic interpolation,
//!    and the hop *passes* iff a(τ*) ≤ θ, |cents| ≤ 50 and the signal is not
//!    periodic at a sub-lag τ0/k (which would mean the k-th harmonic / a note
//!    k× higher is being played → `HarmonicUp(k)`);
//! 3. `confirm` consecutive passing hops → `Present` (Decision evidence);
//! 4. after `Present`, W_r = 2048 windows starting in [onset+60 ms,
//!    onset+160 ms] give the median cents error (Refinement evidence, SC-1a).
//!
//! Streaming and block-size independent: hops are aligned to the absolute
//! frame grid, so the evidence (including `decided_at`) does not depend on
//! how the input is chopped into blocks.
//!
//! Limitations (also in HANDOFF.md): monophonic; legato notes are not
//! supported (they need a pick onset, A-014); a note lower than the expected
//! one whose odd harmonics carry no energy is indistinguishable from the
//! expected note (§2B.1b step 4).

use super::{EvidenceKind, ExpectedNote, Frame, NoteEvidence, NoteVerifier, Onset, Presence};
use std::collections::HashSet;

const SEMITONE: f64 = 1.059_463_094_359_295_3; // 2^(1/12)

#[derive(Debug, Clone)]
pub struct VerifierConfig {
    pub fs: f64,
    pub hop: u64,
    /// a(τ*) ≤ theta for a hop to pass.
    pub theta: f64,
    /// a_k ≤ theta_k at a sub-lag ⇒ HarmonicUp(k).
    pub theta_k: f64,
    /// Skip this many frames after the onset before analysing (attack
    /// transient / pitch glide guard).
    pub g_attack: u64,
    /// Consecutive passing hops required for Present.
    pub confirm: usize,
    pub w_min: usize,
    pub w_max: usize,
    pub w_refine: usize,
    /// Pitch identity band in cents (A-012).
    pub gate_cents: f64,
    /// Highest frequency the sub-lag check considers.
    pub f_max_hz: f64,
    /// Give up (Absent) if no decision after this many frames of hops
    /// following `g_attack`.
    pub observe_frames: u64,
    /// Refinement windows start in [onset + refine_from, onset + refine_to].
    pub refine_from: u64,
    pub refine_to: u64,
    /// Refinement hops with a(τ*) above this are ignored.
    pub theta_refine: f64,
    /// A note with no attributed onset is declared Absent this long after its
    /// window closes (must exceed the onset detector's reporting latency
    /// plus block granularity).
    pub no_onset_grace: u64,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        let fs = 44100.0;
        VerifierConfig {
            fs,
            hop: 128,
            theta: 0.2,
            theta_k: 0.2,
            g_attack: (0.005 * fs).round() as u64, // 5 ms
            confirm: 3,
            w_min: 512,
            w_max: 1024,
            w_refine: 2048,
            gate_cents: 50.0,
            // Plan §2B.1b says E5·2^(1/12), but that leaves octave-up errors above
            // E5 undetectable (no sub-lag to test). Widened to the top of a
            // 24-fret guitar (E6) · 2^(1/12); see DEVIATIONS in HANDOFF.md.
            f_max_hz: 1318.510_227_651_479_8 * SEMITONE,
            observe_frames: (0.150 * fs).round() as u64,
            refine_from: (0.060 * fs).round() as u64,
            refine_to: (0.160 * fs).round() as u64,
            theta_refine: 0.3,
            no_onset_grace: 2048,
        }
    }
}

/// Lag bands and window sizes for one target pitch.
#[derive(Debug, Clone)]
struct Geometry {
    target_hz: f64,
    /// Integration window W.
    w: usize,
    lo: usize,
    hi: usize,
    /// Samples needed per analysis window: W + hi.
    n: usize,
    /// Sub-lag bands for k = 2..=K: (k, lo_k, hi_k).
    sub: Vec<(u8, usize, usize)>,
}

impl Geometry {
    fn new(cfg: &VerifierConfig, target_hz: f64, w_override: Option<usize>, with_sub: bool) -> Self {
        let tau0 = cfg.fs / target_hz;
        let w = w_override.unwrap_or_else(|| (2.0 * tau0).round().clamp(cfg.w_min as f64, cfg.w_max as f64) as usize);
        let band = |tau: f64| -> (usize, usize) {
            let lo = ((tau / SEMITONE).floor() as usize).max(2);
            let hi = ((tau * SEMITONE).ceil() as usize).max(lo + 1);
            (lo, hi)
        };
        let (lo, hi) = band(tau0);
        let mut sub = Vec::new();
        if with_sub {
            let k_max = (cfg.f_max_hz / target_hz).floor() as u32;
            for k in 2..=k_max.min(255) {
                let tk = tau0 / k as f64;
                if tk / SEMITONE < 2.0 {
                    break;
                }
                let (l, h) = band(tk);
                sub.push((k as u8, l, h));
            }
        }
        Geometry { target_hz, w, lo, hi, n: w + hi, sub }
    }
}

/// a(τ) = d/m over `w` samples starting at `x[0]`; needs `x.len() ≥ w + lag`.
fn aperiodicity(x: &[f32], w: usize, lag: usize) -> f64 {
    let (mut d, mut m) = (0.0f64, 0.0f64);
    for j in 0..w {
        let a = x[j] as f64;
        let b = x[j + lag] as f64;
        let df = a - b;
        d += df * df;
        m += a * a + b * b;
    }
    // Digital silence / inaudible: no periodicity evidence.
    if m < 2.0 * w as f64 * 1e-9 {
        return 1.0;
    }
    d / m
}

#[derive(Debug, Clone, Copy)]
enum HopStatus {
    Pass { cents: f64, a: f64 },
    Harmonic { k: u8, cents: f64, a: f64 },
    Fail { cents: Option<f64>, a: f64 },
}

/// Evaluate one analysis window (`x` starts at the window start).
fn eval_window(cfg: &VerifierConfig, g: &Geometry, x: &[f32], scratch: &mut Vec<f64>) -> HopStatus {
    scratch.clear();
    for lag in g.lo..=g.hi {
        scratch.push(aperiodicity(x, g.w, lag));
    }
    let (mut idx, mut a_star) = (0usize, f64::MAX);
    for (i, &a) in scratch.iter().enumerate() {
        if a < a_star {
            a_star = a;
            idx = i;
        }
    }
    let mut tau = (g.lo + idx) as f64;
    if idx > 0 && idx + 1 < scratch.len() {
        let (y0, y1, y2) = (scratch[idx - 1], scratch[idx], scratch[idx + 1]);
        let denom = y0 - 2.0 * y1 + y2;
        if denom > 1e-12 {
            let delta = 0.5 * (y0 - y2) / denom;
            if delta.abs() <= 1.0 {
                tau += delta;
            }
        }
    }
    let f_hat = cfg.fs / tau;
    let cents = 1200.0 * (f_hat / g.target_hz).log2();

    if a_star > cfg.theta || cents.abs() > cfg.gate_cents {
        return HopStatus::Fail { cents: Some(cents), a: a_star };
    }
    // Harmonic sub-lag rejection: periodic at τ0/k ⇒ a k-times-higher note.
    // Report the largest such k (the played note's harmonic index).
    let mut harmonic = None;
    for &(k, lo, hi) in &g.sub {
        let mut a_k = f64::MAX;
        for lag in lo..=hi {
            a_k = a_k.min(aperiodicity(x, g.w, lag));
        }
        if a_k <= cfg.theta_k {
            harmonic = Some(k);
        }
    }
    match harmonic {
        Some(k) => HopStatus::Harmonic { k, cents, a: a_star },
        None => HopStatus::Pass { cents, a: a_star },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// No onset attributed yet.
    Waiting,
    /// Onset attributed; evaluating confirmation hops.
    Deciding,
    /// Present decided; collecting SC-1a refinement windows.
    Refining,
    Done,
}

struct NoteState {
    exp: ExpectedNote,
    geom: Geometry,
    refine_geom: Geometry,
    phase: Phase,
    onset: Option<Frame>,
    /// Start of the next confirmation window.
    next_start: Frame,
    /// Start of the first confirmation window (deadline reference).
    first_start: Frame,
    consec_pass: usize,
    pass_cents: Vec<f32>,
    harm_k: u8,
    consec_harm: usize,
    /// Start of the next refinement window / last allowed start.
    refine_next: Frame,
    refine_last: Frame,
    refine_cents: Vec<f32>,
    refine_last_end: Frame,
    last_a: f64,
    last_cents: Option<f64>,
}

pub struct ConstrainedNsdfVerifier {
    cfg: VerifierConfig,
    hist: Vec<f32>,
    hist_start: Frame,
    started: bool,
    states: Vec<NoteState>,
    finished: HashSet<u32>,
    /// (onset, already attributed).
    pool: Vec<(Onset, bool)>,
    scratch: Vec<f64>,
}

fn median(v: &mut [f32]) -> f32 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

impl ConstrainedNsdfVerifier {
    pub fn new(cfg: VerifierConfig) -> Self {
        ConstrainedNsdfVerifier {
            cfg,
            hist: Vec::new(),
            hist_start: 0,
            started: false,
            states: Vec::new(),
            finished: HashSet::new(),
            pool: Vec::new(),
            scratch: Vec::new(),
        }
    }

    pub fn config(&self) -> &VerifierConfig {
        &self.cfg
    }

    /// First hop-grid frame ≥ `f`.
    fn ceil_hop(&self, f: Frame) -> Frame {
        f.div_ceil(self.cfg.hop) * self.cfg.hop
    }

    fn hist_end(&self) -> Frame {
        self.hist_start + self.hist.len() as Frame
    }

    fn register(&mut self, expected: &[ExpectedNote]) {
        for e in expected {
            let id = e.note.id;
            if self.finished.contains(&id) || self.states.iter().any(|s| s.exp.note.id == id) {
                continue;
            }
            let target = e.note.target_hz as f64;
            let geom = Geometry::new(&self.cfg, target, None, true);
            let refine_geom = Geometry::new(&self.cfg, target, Some(self.cfg.w_refine), false);
            self.states.push(NoteState {
                exp: e.clone(),
                geom,
                refine_geom,
                phase: Phase::Waiting,
                onset: None,
                next_start: 0,
                first_start: 0,
                consec_pass: 0,
                pass_cents: Vec::new(),
                harm_k: 0,
                consec_harm: 0,
                refine_next: 0,
                refine_last: 0,
                refine_cents: Vec::new(),
                refine_last_end: 0,
                last_a: 1.0,
                last_cents: None,
            });
        }
    }

    /// Greedy one-onset-per-note attribution by smallest |onset − centre|.
    fn attribute(&mut self) {
        let mut pairs: Vec<(f64, usize, usize)> = Vec::new(); // (err, state, pool)
        for (si, s) in self.states.iter().enumerate() {
            if s.phase != Phase::Waiting {
                continue;
            }
            for (pi, (o, used)) in self.pool.iter().enumerate() {
                if !*used && o.frame >= s.exp.open && o.frame < s.exp.close {
                    pairs.push(((o.frame as f64 - s.exp.center()).abs(), si, pi));
                }
            }
        }
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        for (_, si, pi) in pairs {
            if self.pool[pi].1 || self.states[si].phase != Phase::Waiting {
                continue;
            }
            self.pool[pi].1 = true;
            let o = self.pool[pi].0.frame;
            let start = self.ceil_hop(o + self.cfg.g_attack);
            let refine_first = self.ceil_hop(o + self.cfg.refine_from);
            let sustain_end = o + (self.states[si].exp.note.sustain_s * self.cfg.fs) as Frame;
            let refine_last = (o + self.cfg.refine_to).min(sustain_end).max(refine_first);
            let s = &mut self.states[si];
            s.phase = Phase::Deciding;
            s.onset = Some(o);
            s.next_start = start;
            s.first_start = start;
            s.refine_next = refine_first;
            s.refine_last = refine_last;
        }
    }

    fn advance(&mut self, out: &mut Vec<NoteEvidence>) {
        let avail_end = self.hist_end();
        let cfg = self.cfg.clone();
        for si in 0..self.states.len() {
            // --- Waiting: no onset by the end of the window (+ grace) ⇒ Absent.
            if self.states[si].phase == Phase::Waiting {
                let s = &mut self.states[si];
                let deadline = s.exp.close + cfg.no_onset_grace;
                if avail_end >= deadline {
                    out.push(NoteEvidence {
                        note_id: s.exp.note.id,
                        kind: EvidenceKind::Decision,
                        presence: Presence::Absent,
                        onset: None,
                        decided_at: deadline,
                        cents_error: None,
                        aperiodicity: 1.0,
                    });
                    s.phase = Phase::Done;
                }
                continue;
            }

            // --- Deciding: confirmation hops.
            while self.states[si].phase == Phase::Deciding {
                let (start, n) = (self.states[si].next_start, self.states[si].geom.n);
                if start + n as Frame > avail_end {
                    break;
                }
                let status = {
                    let x = &self.hist[(start - self.hist_start) as usize..][..n];
                    eval_window(&cfg, &self.states[si].geom, x, &mut self.scratch)
                };
                let end = start + n as Frame;
                let s = &mut self.states[si];
                match status {
                    HopStatus::Pass { cents, a } => {
                        s.consec_pass += 1;
                        s.consec_harm = 0;
                        s.pass_cents.push(cents as f32);
                        s.last_a = a;
                        s.last_cents = Some(cents);
                    }
                    HopStatus::Harmonic { k, cents, a } => {
                        s.consec_pass = 0;
                        s.pass_cents.clear();
                        if s.harm_k == k {
                            s.consec_harm += 1;
                        } else {
                            s.harm_k = k;
                            s.consec_harm = 1;
                        }
                        s.last_a = a;
                        s.last_cents = Some(cents);
                    }
                    HopStatus::Fail { cents, a } => {
                        s.consec_pass = 0;
                        s.consec_harm = 0;
                        s.pass_cents.clear();
                        s.last_a = a;
                        s.last_cents = cents;
                    }
                }
                if s.consec_pass >= cfg.confirm {
                    let tail = &mut s.pass_cents[s.pass_cents.len() - cfg.confirm..].to_vec();
                    out.push(NoteEvidence {
                        note_id: s.exp.note.id,
                        kind: EvidenceKind::Decision,
                        presence: Presence::Present,
                        onset: s.onset,
                        decided_at: end,
                        cents_error: Some(median(tail)),
                        aperiodicity: s.last_a as f32,
                    });
                    s.phase = Phase::Refining;
                    break;
                }
                if s.consec_harm >= cfg.confirm {
                    out.push(NoteEvidence {
                        note_id: s.exp.note.id,
                        kind: EvidenceKind::Decision,
                        presence: Presence::HarmonicUp(s.harm_k),
                        onset: s.onset,
                        decided_at: end,
                        cents_error: s.last_cents.map(|c| c as f32),
                        aperiodicity: s.last_a as f32,
                    });
                    s.phase = Phase::Done;
                    break;
                }
                s.next_start += cfg.hop;
                if s.next_start > s.first_start + cfg.observe_frames {
                    out.push(NoteEvidence {
                        note_id: s.exp.note.id,
                        kind: EvidenceKind::Decision,
                        presence: Presence::Absent,
                        onset: s.onset,
                        decided_at: end,
                        cents_error: s.last_cents.map(|c| c as f32),
                        aperiodicity: s.last_a as f32,
                    });
                    s.phase = Phase::Done;
                }
            }

            // --- Refining: SC-1a windows (W_r = 2048) in the sustain.
            while self.states[si].phase == Phase::Refining {
                let (start, n) = (self.states[si].refine_next, self.states[si].refine_geom.n);
                if start > self.states[si].refine_last {
                    let s = &mut self.states[si];
                    let cents = if s.refine_cents.is_empty() { None } else { Some(median(&mut s.refine_cents)) };
                    out.push(NoteEvidence {
                        note_id: s.exp.note.id,
                        kind: EvidenceKind::Refinement,
                        presence: Presence::Present,
                        onset: s.onset,
                        decided_at: s.refine_last_end,
                        cents_error: cents,
                        aperiodicity: s.last_a as f32,
                    });
                    s.phase = Phase::Done;
                    break;
                }
                if start + n as Frame > avail_end {
                    break;
                }
                let status = {
                    let x = &self.hist[(start - self.hist_start) as usize..][..n];
                    eval_window(&cfg, &self.states[si].refine_geom, x, &mut self.scratch)
                };
                let s = &mut self.states[si];
                if let HopStatus::Pass { cents, a } | HopStatus::Harmonic { cents, a, .. } = status {
                    if a <= cfg.theta_refine {
                        s.refine_cents.push(cents as f32);
                        s.last_a = a;
                    }
                }
                s.refine_last_end = start + n as Frame;
                s.refine_next += cfg.hop;
            }
        }
    }

    fn finish_and_trim(&mut self) {
        let mut done_ids = Vec::new();
        self.states.retain(|s| {
            if s.phase == Phase::Done {
                done_ids.push(s.exp.note.id);
                false
            } else {
                true
            }
        });
        self.finished.extend(done_ids);

        let avail_end = self.hist_end();
        let mut needed = avail_end;
        for s in &self.states {
            let need = match s.phase {
                Phase::Waiting => s.exp.open,
                Phase::Deciding => s.next_start.min(s.refine_next),
                Phase::Refining => s.refine_next,
                Phase::Done => avail_end,
            };
            needed = needed.min(need);
        }
        needed = needed.max(self.hist_start);
        let drop = (needed - self.hist_start) as usize;
        if drop >= 16384 {
            self.hist.drain(..drop);
            self.hist_start += drop as Frame;
        }
        let horizon = avail_end.saturating_sub(2 * 44100);
        self.pool.retain(|(o, _)| o.frame >= horizon);
    }
}

impl NoteVerifier for ConstrainedNsdfVerifier {
    fn process(
        &mut self,
        block: &[f32],
        first: Frame,
        expected: &[ExpectedNote],
        onsets: &[Onset],
        out: &mut Vec<NoteEvidence>,
    ) {
        if !self.started || first != self.hist_end() {
            // First call, or a gap in the stream: history is not contiguous.
            self.hist.clear();
            self.hist_start = first;
            self.started = true;
        }
        self.hist.extend_from_slice(block);
        for &o in onsets {
            if !self.pool.iter().any(|(p, _)| p.frame == o.frame) {
                self.pool.push((o, false));
            }
        }
        self.register(expected);
        self.attribute();
        self.advance(out);
        self.finish_and_trim();
    }
}

//! Deterministic fixture generator (plan §2B.2, "Provenance Contract").
//!
//! Fixtures are *generated*, not committed as binaries: every signal is a pure
//! function of its parameters and an integer seed, so the provenance record is
//! (commit hash, generator seed). Randomness comes from `Rng` below, never the
//! OS.
//!
//! The plucked-string model is fractional-delay Karplus–Strong. Ground truth:
//!
//! * **f0** — the loop is tuned so the *actual* oscillation frequency of the
//!   loop (integer delay + two-point-average phase delay of exactly 0.5
//!   samples + the all-pass fractional delay evaluated at f0, not at DC)
//!   equals the requested f0. `tuning_selftest` in this crate checks that
//!   claim independently with a long-window DFT peak search.
//! * **onset** — the excitation start frame (the offset at which the pluck is
//!   mixed), which is also the first non-zero output sample.
//!
//! All timestamps are frames at `FS`.

use std::f64::consts::PI;

pub const FS: f64 = 44100.0;

/// xorshift64* — small, seedable, dependency-free.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Avoid the all-zero fixed point; mix the seed so nearby seeds diverge.
        let mut s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
        if s == 0 {
            s = 0x1234_5678_9ABC_DEF1;
        }
        Rng(s)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in [0, 1).
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in [-1, 1).
    pub fn bipolar(&mut self) -> f64 {
        self.uniform() * 2.0 - 1.0
    }
}

pub fn midi_to_hz(midi: f64) -> f64 {
    440.0 * 2f64.powf((midi - 69.0) / 12.0)
}

pub fn cents_between(measured_hz: f64, target_hz: f64) -> f64 {
    1200.0 * (measured_hz / target_hz).log2()
}

pub fn ms_to_frames(ms: f64) -> u64 {
    (ms * 1e-3 * FS).round() as u64
}

/// Pure sine, for T-001.
pub fn sine(f_hz: f64, amp: f32, len: usize) -> Vec<f32> {
    (0..len).map(|n| (amp as f64 * (2.0 * PI * f_hz * n as f64 / FS).sin()) as f32).collect()
}

/// White noise of the given RMS (uniform-distributed, so rms = peak/√3).
pub fn white_noise(len: usize, rms: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let peak = rms as f64 * 3f64.sqrt();
    (0..len).map(|_| (rng.bipolar() * peak) as f32).collect()
}

/// Add `gain * src` into `dst` starting at frame `offset`, growing `dst` with
/// silence if needed.
pub fn mix_at(dst: &mut Vec<f32>, src: &[f32], offset: usize, gain: f32) {
    if dst.len() < offset + src.len() {
        dst.resize(offset + src.len(), 0.0);
    }
    for (i, s) in src.iter().enumerate() {
        dst[offset + i] += gain * s;
    }
}

/// Add a stationary noise floor across the whole signal.
pub fn add_noise_floor(dst: &mut [f32], rms: f32, seed: u64) {
    let noise = white_noise(dst.len(), rms, seed);
    for (d, n) in dst.iter_mut().zip(noise) {
        *d += n;
    }
}

/// Phase delay (in samples) of the first-order all-pass
/// A(z) = (a + z⁻¹) / (1 + a·z⁻¹) at angular frequency `w` (rad/sample).
fn allpass_phase_delay(a: f64, w: f64) -> f64 {
    let phase = -(w.sin()).atan2(a + w.cos()) + (a * w.sin()).atan2(1.0 + a * w.cos());
    -phase / w
}

/// Solve for the all-pass coefficient whose phase delay *at `w`* equals
/// `delay` samples (delay in [0.5, 1.5] is well conditioned). Bisection; the
/// phase delay is monotonically decreasing in `a` on [-0.5, 0.5].
fn allpass_coeff_for_delay(delay: f64, w: f64) -> f64 {
    let (mut lo, mut hi) = (-0.5f64, 0.5f64);
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if allpass_phase_delay(mid, w) > delay {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Parameters of one synthesized pluck. Starts at frame 0 of the returned
/// buffer; place it in a mix with `mix_at`.
#[derive(Debug, Clone)]
pub struct Pluck {
    pub f0_hz: f64,
    pub amp: f32,
    /// Overall amplitude decay, dB per second (fundamental).
    pub decay_db_per_s: f64,
    pub seed: u64,
    /// Attack pitch glide: initial offset in cents, decaying exponentially
    /// with time constant `glide_tau_s`. 0 disables. (R-102, T-033.)
    pub glide_cents: f64,
    pub glide_tau_s: f64,
    /// Attenuate the fundamental in the *output* by this many dB (peaking EQ
    /// at f0), modelling pickups / pick position that weaken H1. (R-101,
    /// T-032.) 0 disables.
    pub h1_atten_db: f64,
}

impl Pluck {
    pub fn new(f0_hz: f64, seed: u64) -> Self {
        Pluck {
            f0_hz,
            amp: 0.5,
            decay_db_per_s: 8.0,
            seed,
            glide_cents: 0.0,
            glide_tau_s: 0.05,
            h1_atten_db: 0.0,
        }
    }

    pub fn amp(mut self, a: f32) -> Self {
        self.amp = a;
        self
    }
    pub fn glide(mut self, cents: f64, tau_s: f64) -> Self {
        self.glide_cents = cents;
        self.glide_tau_s = tau_s;
        self
    }
    pub fn h1_atten(mut self, db: f64) -> Self {
        self.h1_atten_db = db;
        self
    }
    pub fn decay(mut self, db_per_s: f64) -> Self {
        self.decay_db_per_s = db_per_s;
        self
    }

    /// Render `len` frames.
    pub fn render(&self, len: usize) -> Vec<f32> {
        // Extra head-room so the glide time-warp never reads past the end.
        let warp_extra = if self.glide_cents != 0.0 {
            (len as f64 * (2f64.powf(self.glide_cents.abs() / 1200.0) - 1.0)).ceil() as usize + 8
        } else {
            0
        };
        let raw = self.render_ks(len + warp_extra);
        let mut out = if self.glide_cents != 0.0 { self.apply_glide(&raw, len) } else { raw };
        out.truncate(len);
        if self.h1_atten_db != 0.0 {
            peaking_eq(&mut out, self.f0_hz, 2.0, -self.h1_atten_db.abs());
        }
        out
    }

    fn render_ks(&self, len: usize) -> Vec<f32> {
        let period = FS / self.f0_hz;
        let w0 = 2.0 * PI * self.f0_hz / FS;
        // Loop = N-sample delay + two-point average (exactly 0.5 samples of
        // phase delay at every frequency) + all-pass with delay D at f0.
        let n = (period - 1.0).floor() as usize;
        let d = period - 0.5 - n as f64;
        let a = allpass_coeff_for_delay(d, w0);
        // Loop gain so the fundamental decays at `decay_db_per_s`.
        let rho = 10f64.powf(-self.decay_db_per_s / 20.0 / self.f0_hz);

        let mut rng = Rng::new(self.seed);
        // Excitation: noise burst, lightly low-passed (pick softness), DC
        // removed, normalized to unit peak.
        let mut exc: Vec<f64> = (0..n).map(|_| rng.bipolar()).collect();
        let mut prev = 0.0;
        for e in exc.iter_mut() {
            let raw = *e;
            *e = 0.5 * (raw + prev);
            prev = raw;
        }
        let mean = exc.iter().sum::<f64>() / n as f64;
        exc.iter_mut().for_each(|e| *e -= mean);
        let peak = exc.iter().fold(0.0f64, |m, e| m.max(e.abs())).max(1e-12);
        exc.iter_mut().for_each(|e| *e /= peak);

        let mut line = exc; // circular delay line holding N samples
        let mut pos = 0usize;
        let mut avg_prev = 0.0f64;
        let (mut ap_x1, mut ap_y1) = (0.0f64, 0.0f64);
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            let y = line[pos];
            out.push((y * self.amp as f64) as f32);
            // Loop filter: two-point average, then all-pass, then gain.
            let avg = 0.5 * (y + avg_prev);
            avg_prev = y;
            let ap = a * avg + ap_x1 - a * ap_y1;
            ap_x1 = avg;
            ap_y1 = ap;
            line[pos] = ap * rho;
            pos += 1;
            if pos == n {
                pos = 0;
            }
        }
        out
    }

    /// Time-warp `raw` so its instantaneous pitch is scaled by
    /// r(t) = 2^(glide_cents/1200 · exp(−t/τ)); r → 1 after a few τ.
    /// Catmull-Rom interpolation. Frame 0 (the onset) is unchanged.
    fn apply_glide(&self, raw: &[f32], len: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(len);
        let mut phase = 0.0f64; // read position in `raw`
        for n in 0..len {
            let t = n as f64 / FS;
            out.push(catmull_rom(raw, phase));
            let cents = self.glide_cents * (-t / self.glide_tau_s).exp();
            phase += 2f64.powf(cents / 1200.0);
        }
        out
    }
}

fn catmull_rom(x: &[f32], pos: f64) -> f32 {
    let i = pos.floor() as isize;
    let f = pos - i as f64;
    let at = |k: isize| -> f64 {
        if k < 0 || k as usize >= x.len() {
            0.0
        } else {
            x[k as usize] as f64
        }
    };
    let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
    let v = 0.5
        * ((2.0 * p1)
            + (-p0 + p2) * f
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * f * f
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * f * f * f);
    v as f32
}

/// RBJ peaking EQ, in place.
fn peaking_eq(x: &mut [f32], f_hz: f64, q: f64, gain_db: f64) {
    let a = 10f64.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * f_hz / FS;
    let alpha = w0.sin() / (2.0 * q);
    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * w0.cos();
    let b2 = 1.0 - alpha * a;
    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * w0.cos();
    let a2 = 1.0 - alpha / a;
    let (b0, b1, b2, a1, a2) = (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0);
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for s in x.iter_mut() {
        let xn = *s as f64;
        let yn = b0 * xn + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
        x2 = x1;
        x1 = xn;
        y2 = y1;
        y1 = yn;
        *s = yn as f32;
    }
}

/// Raised-cosine fade-out over the last `n` samples (avoids a truncation
/// click being mistaken for an onset when a pluck buffer ends mid-ring).
pub fn fade_out(x: &mut [f32], n: usize) {
    let n = n.min(x.len());
    let len = x.len();
    for i in 0..n {
        let g = 0.5 + 0.5 * (PI * i as f64 / n as f64).cos();
        x[len - n + i] *= g as f32;
    }
}

/// One entry of a generated pluck sequence with its ground truth.
#[derive(Debug, Clone)]
pub struct PluckEvent {
    pub onset_frame: u64,
    pub f0_hz: f64,
    pub amp: f32,
}

/// The 50-pluck timing fixture (plan §2B.2 / T-004): plucks across E2–E5 with
/// varied velocity, spaced 350–550 ms apart (so earlier notes still ring
/// underneath later onsets), on a −70 dBFS noise floor. Deterministic in
/// `seed`.
pub fn fifty_pluck_set(seed: u64) -> (Vec<f32>, Vec<PluckEvent>) {
    let mut rng = Rng::new(seed);
    let lead = ms_to_frames(500.0) as usize;
    let mut t = lead;
    let mut events = Vec::new();
    let mut plucks = Vec::new();
    for k in 0..50u64 {
        let midi = 40.0 + (rng.uniform() * 40.0).floor(); // E2 (40) … E5 (79)
        let f0 = midi_to_hz(midi);
        let amp = (0.25 + 0.7 * rng.uniform()) as f32;
        let len = ms_to_frames(900.0) as usize;
        let p = Pluck::new(f0, seed.wrapping_add(1000 + k)).amp(amp);
        let mut s = p.render(len);
        fade_out(&mut s, ms_to_frames(150.0) as usize);
        plucks.push((t, s));
        events.push(PluckEvent { onset_frame: t as u64, f0_hz: f0, amp });
        t += ms_to_frames(350.0 + 200.0 * rng.uniform()) as usize;
    }
    let mut sig = vec![0.0f32; t + ms_to_frames(1500.0) as usize];
    for (off, s) in &plucks {
        mix_at(&mut sig, s, *off, 1.0);
    }
    add_noise_floor(&mut sig, 3e-4, seed ^ 0xABCD); // ≈ −70 dBFS rms
    (sig, events)
}

/// Frequency (Hz) of the strongest spectral component of `x[start..start+len]`
/// within ±`span_cents` of `f_guess`, by Hann-windowed DFT evaluated on a fine
/// grid plus parabolic refinement. Independent of any code in gtcore, used to
/// check the generator's f0 ground truth.
pub fn spectral_peak_hz(x: &[f32], start: usize, len: usize, f_guess: f64, span_cents: f64) -> f64 {
    let step = 0.25; // cents
    let n_pts = (2.0 * span_cents / step) as usize + 1;
    let mag = |f: f64| -> f64 {
        let w = 2.0 * PI * f / FS;
        let (mut re, mut im) = (0.0, 0.0);
        for i in 0..len {
            let win = 0.5 - 0.5 * (2.0 * PI * i as f64 / (len - 1) as f64).cos();
            let v = x[start + i] as f64 * win;
            re += v * (w * i as f64).cos();
            im -= v * (w * i as f64).sin();
        }
        (re * re + im * im).sqrt()
    };
    let mags: Vec<f64> = (0..n_pts)
        .map(|k| mag(f_guess * 2f64.powf((-span_cents + k as f64 * step) / 1200.0)))
        .collect();
    let (kmax, _) = mags
        .iter()
        .enumerate()
        .fold((0, f64::MIN), |(bk, bm), (k, &m)| if m > bm { (k, m) } else { (bk, bm) });
    let mut k_f = kmax as f64;
    if kmax > 0 && kmax + 1 < mags.len() {
        let (a, b, c) = (mags[kmax - 1], mags[kmax], mags[kmax + 1]);
        let denom = a - 2.0 * b + c;
        if denom.abs() > 1e-12 {
            k_f += 0.5 * (a - c) / denom;
        }
    }
    f_guess * 2f64.powf((-span_cents + k_f * step) / 1200.0)
}

#[cfg(test)]
mod tuning_selftest {
    use super::*;

    /// The generator's claim: the rendered fundamental sits at the requested
    /// f0. Checked across E2–E5 by an independent DFT peak search over
    /// 0.4 s of sustain.
    #[test]
    fn ks_fundamental_matches_requested_f0_within_2_cents() {
        for midi in [40, 45, 50, 55, 57, 62, 67, 69, 74, 79] {
            let f0 = midi_to_hz(midi as f64);
            let x = Pluck::new(f0, 7).render(ms_to_frames(700.0) as usize);
            let start = ms_to_frames(100.0) as usize;
            let len = ms_to_frames(400.0) as usize;
            let measured = spectral_peak_hz(&x, start, len, f0, 40.0);
            let err = cents_between(measured, f0);
            assert!(err.abs() <= 2.0, "midi {midi}: f0 {f0:.3} measured {measured:.3} ({err:+.2} cents)");
        }
    }

    #[test]
    fn generator_is_deterministic() {
        let a = Pluck::new(110.0, 42).render(5000);
        let b = Pluck::new(110.0, 42).render(5000);
        assert_eq!(a, b);
        let c = Pluck::new(110.0, 43).render(5000);
        assert_ne!(a, c);
        let (s1, e1) = fifty_pluck_set(9);
        let (s2, e2) = fifty_pluck_set(9);
        assert_eq!(s1, s2);
        assert_eq!(e1.len(), e2.len());
    }

    #[test]
    fn first_nonzero_sample_is_the_onset() {
        let x = Pluck::new(196.0, 1).render(1000);
        assert!(x[0].abs() > 0.0 || x[1].abs() > 0.0);
    }
}

/// One pluck to place in a rendered part.
#[derive(Debug, Clone, Copy)]
pub struct PluckAt {
    /// Frame of the excitation start (= ground-truth onset).
    pub frame: usize,
    pub f0_hz: f64,
    pub amp: f32,
}

/// Render a guitar part: each pluck synthesized with its own seed, faded out
/// after `ring_ms`, mixed on a −70 dBFS noise floor. `total_len` frames long.
pub fn render_part(plucks: &[PluckAt], total_len: usize, ring_ms: f64, seed: u64) -> Vec<f32> {
    let mut sig = vec![0.0f32; total_len];
    let ring = ms_to_frames(ring_ms) as usize;
    for (k, p) in plucks.iter().enumerate() {
        let len = ring.min(total_len.saturating_sub(p.frame));
        if len == 0 {
            continue;
        }
        let mut s = Pluck::new(p.f0_hz, seed.wrapping_add(k as u64 * 7919 + 1))
            .amp(p.amp)
            .decay(guitar_decay_db_per_s(p.f0_hz))
            .render(len);
        fade_out(&mut s, ms_to_frames(120.0) as usize);
        mix_at(&mut sig, &s, p.frame, 1.0);
    }
    add_noise_floor(&mut sig, 3e-4, seed ^ 0x5CA1E);
    sig
}

/// Amplitude decay of the fundamental, dB/s, used by `render_part`: slower for
/// low strings, faster for high ones (E2 ≈ 12, A3 ≈ 19, E4 ≈ 24, E5 ≈ 34).
/// ASSUMPTION: a plausible electric-guitar figure chosen by the implementer,
/// not taken from a measurement or a verified source (plan Phase 1 research
/// was never run). The verifier tests that depend on ring level (T-034, the
/// let-ring sweep in the spike) use explicit, stated levels instead.
pub fn guitar_decay_db_per_s(f0_hz: f64) -> f64 {
    12.0 * (f0_hz / 82.41).sqrt()
}

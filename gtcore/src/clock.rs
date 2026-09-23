//! Clock model (plan step S-003, plan/PLAN.md §2B.1d).
//!
//! This module is pure math: given latencies and a running frame index,
//! where is "song time"? It has no dependency on cubeb, a real audio
//! device, or Godot, so it is fully testable in a CI-tier sandbox without
//! hardware. The real-time seqlock publishing loop described in §2B.1e
//! (RT callback publishes `(monotonic_ns, frames_written)`) is NOT
//! implemented here — that requires a live cubeb duplex stream and is
//! part of `gtaudio`, deferred pending hardware access (see HANDOFF.md).

/// Running frame index since the duplex stream started. Input and output
/// share this index (C-011, confirmed by spike 1 — not yet run).
pub type Frame = u64;

/// Parameters fixed at stream start, per §2B.1d.
#[derive(Debug, Clone, Copy)]
pub struct ClockModel {
    pub fs: f64,
    /// Output frame at which the backing track's sample 0 plays.
    pub s0: Frame,
    /// Input latency reported by the backend at stream start (seconds).
    pub l_in: f64,
    /// Output latency reported by the backend at stream start (seconds).
    pub l_out: f64,
    /// Signed user audio offset, δ_a (seconds). Positive delays the audio
    /// interpretation relative to the video/song clock.
    pub delta_a: f64,
}

impl ClockModel {
    /// L_rt = L_in + L_out + δ_a (§2B.1d). This is the only latency term
    /// that affects scoring accuracy — everything else in the pipeline
    /// (§2B.1c) affects feedback delay only, because judgments are made
    /// from input frame indices, not wall-clock time.
    pub fn l_rt(&self) -> f64 {
        self.l_in + self.l_out + self.delta_a
    }

    /// song_time(i) = (i − s0)/fs − L_rt   (§2B.1d)
    ///
    /// `i` may be before `s0` (pre-roll / count-in), giving a negative
    /// song time, which is valid and expected.
    pub fn song_time(&self, i: Frame) -> f64 {
        (i as f64 - self.s0 as f64) / self.fs - self.l_rt()
    }

    /// Inverse of `song_time`: the input frame index at which a given
    /// song time is expected to be heard. Used to map `ChartNote::start_s`
    /// into an `ExpectedNote` window (§2B.1a).
    pub fn frame_for_song_time(&self, song_time_s: f64) -> Frame {
        let i = (song_time_s + self.l_rt()) * self.fs + self.s0 as f64;
        i.round().max(0.0) as Frame
    }
}

/// A single (monotonic_ns, frames_written) sample published by the RT
/// callback via the seqlock described in §2B.1d. In the real system these
/// arrive from `gtaudio`; here they are plain data so the fitting math can
/// be tested against synthetic, deterministically-jittered sequences.
#[derive(Debug, Clone, Copy)]
pub struct ClockSample {
    pub monotonic_ns: i64,
    pub frames_written: Frame,
}

/// Least-squares fit of `frames = alpha + beta * t_ns`, over the most
/// recent samples (§2B.1d: "the game thread fits ... by least squares
/// over the last 32 callbacks"). Returns `None` if fewer than 2 samples
/// or all timestamps coincide (degenerate fit).
#[derive(Debug, Clone, Copy)]
pub struct RenderClockFit {
    pub alpha: f64,
    pub beta: f64, // frames per nanosecond
    last_frames_hint: f64,
}

impl RenderClockFit {
    pub fn fit(samples: &[ClockSample]) -> Option<Self> {
        let n = samples.len();
        if n < 2 {
            return None;
        }
        let n_f = n as f64;
        let mean_t: f64 = samples.iter().map(|s| s.monotonic_ns as f64).sum::<f64>() / n_f;
        let mean_f: f64 =
            samples.iter().map(|s| s.frames_written as f64).sum::<f64>() / n_f;

        let mut num = 0.0;
        let mut den = 0.0;
        for s in samples {
            let dt = s.monotonic_ns as f64 - mean_t;
            let df = s.frames_written as f64 - mean_f;
            num += dt * df;
            den += dt * dt;
        }
        if den.abs() < f64::EPSILON {
            return None; // all timestamps identical
        }
        let beta = num / den;
        let alpha = mean_f - beta * mean_t;
        let last_frames_hint =
            samples.iter().map(|s| s.frames_written as f64).fold(f64::MIN, f64::max);
        Some(RenderClockFit { alpha, beta, last_frames_hint })
    }

    /// Estimated output frame count at monotonic time `t_ns`. Callers are
    /// responsible for clamping to monotonic non-decreasing output
    /// (§2B.1d: "clamped to be monotonic") across successive calls with
    /// possibly-refit models; `frames_at` alone does not track history.
    pub fn frames_at(&self, t_ns: i64) -> f64 {
        self.alpha + self.beta * t_ns as f64
    }

    /// Most recent observed frame count in the fitted window; useful as a
    /// monotonicity floor when a caller refits and must not regress.
    pub fn observed_floor(&self) -> f64 {
        self.last_frames_hint
    }
}

/// Applies the render clock to produce the value actually drawn, per
/// §2B.1d: song_time_render = (frames(now + δ_v − L_out) − s0)/fs,
/// clamped to be monotonic against `prev_frames`.
pub fn render_song_time(
    fit: &RenderClockFit,
    now_ns: i64,
    delta_v_s: f64,
    l_out_s: f64,
    clock: &ClockModel,
    prev_frames: Option<f64>,
) -> (f64, f64) {
    let query_ns = now_ns + ((delta_v_s - l_out_s) * 1e9) as i64;
    let mut frames = fit.frames_at(query_ns);
    if let Some(prev) = prev_frames {
        if frames < prev {
            frames = prev; // monotonic clamp
        }
    }
    let song_time = (frames - clock.s0 as f64) / clock.fs;
    (song_time, frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_clock() -> ClockModel {
        ClockModel { fs: 44100.0, s0: 44100, l_in: 0.010, l_out: 0.015, delta_a: 0.0 }
    }

    // T-023: song_time(i) = (i - s0)/fs - L_rt for synthetic latencies and
    // offsets. Exercised as a small deterministic property check across a
    // grid of latencies, offsets and frame indices (no proptest
    // dependency available in this sandbox — see network note in
    // HANDOFF.md).
    #[test]
    fn t023_song_time_matches_formula_across_parameter_grid() {
        for &l_in in &[0.0, 0.005, 0.010, 0.030] {
            for &l_out in &[0.0, 0.005, 0.015, 0.050] {
                for &delta_a in &[-0.050, 0.0, 0.020] {
                    for &s0 in &[0u64, 44100, 88200] {
                        let clock = ClockModel { fs: 44100.0, s0, l_in, l_out, delta_a };
                        for &i in &[0u64, 1, s0, s0 + 44100, s0 + 220500] {
                            let expected =
                                (i as f64 - s0 as f64) / 44100.0 - (l_in + l_out + delta_a);
                            let got = clock.song_time(i);
                            assert!(
                                (got - expected).abs() < 1e-9,
                                "song_time mismatch: l_in={l_in} l_out={l_out} delta_a={delta_a} s0={s0} i={i} got={got} expected={expected}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn t023_frame_for_song_time_is_inverse_of_song_time() {
        let clock = base_clock();
        for &t in &[-0.5, 0.0, 0.001, 1.234, 60.0] {
            let f = clock.frame_for_song_time(t);
            let back = clock.song_time(f);
            assert!((back - t).abs() < 1.0 / clock.fs, "round trip failed for t={t}: got {back}");
        }
    }

    #[test]
    fn l_rt_is_sum_of_components() {
        let clock = ClockModel { fs: 44100.0, s0: 0, l_in: 0.010, l_out: 0.020, delta_a: -0.003 };
        assert!((clock.l_rt() - 0.027).abs() < 1e-12);
    }

    // Partial coverage toward T-024 ("render clock under callback jitter:
    // monotonic, error <=1ms"): this tests only the pure least-squares fit
    // and monotonic clamp in isolation, against a synthetic jittered
    // sequence. It does NOT exercise the real seqlock/thread-timing path,
    // which needs a live audio callback (gtaudio, deferred to hardware).
    #[test]
    fn render_clock_fit_recovers_slope_under_synthetic_jitter() {
        let fs = 44100.0;
        let true_beta = fs / 1e9; // frames per ns, i.e. fs frames per second
        let mut samples = Vec::new();
        // Deterministic pseudo-jitter: +/- up to 2ms, no RNG dependency.
        let jitter_ns = [0i64, 1_800_000, -1_500_000, 900_000, -2_000_000, 400_000, -700_000, 1_200_000];
        for k in 0..32i64 {
            let nominal_ns = k * 10_000_000; // 10ms callback period
            let t_ns = nominal_ns + jitter_ns[(k as usize) % jitter_ns.len()];
            let frames = (true_beta * nominal_ns as f64).round() as u64;
            samples.push(ClockSample { monotonic_ns: t_ns, frames_written: frames });
        }
        let fit = RenderClockFit::fit(&samples).expect("fit should succeed");
        let rel_err = (fit.beta - true_beta).abs() / true_beta;
        assert!(rel_err < 0.01, "slope estimate off by {:.4}%, expected <1%", rel_err * 100.0);
    }

    #[test]
    fn render_clock_fit_none_below_two_samples() {
        assert!(RenderClockFit::fit(&[]).is_none());
        let one = [ClockSample { monotonic_ns: 0, frames_written: 0 }];
        assert!(RenderClockFit::fit(&one).is_none());
    }

    #[test]
    fn render_song_time_is_monotonic_under_clamp() {
        let clock = base_clock();
        let samples = [
            ClockSample { monotonic_ns: 0, frames_written: 0 },
            ClockSample { monotonic_ns: 10_000_000, frames_written: 441 },
        ];
        let fit = RenderClockFit::fit(&samples).unwrap();
        let (t1, f1) = render_song_time(&fit, 20_000_000, 0.0, 0.0, &clock, None);
        // Query an earlier time than the previous sample to simulate a
        // refit that would otherwise regress; must clamp forward.
        let (t2, f2) = render_song_time(&fit, 5_000_000, 0.0, 0.0, &clock, Some(f1));
        assert!(f2 >= f1, "render clock regressed: f1={f1} f2={f2}");
        assert!(t2 >= t1 - 1e-9);
    }
}

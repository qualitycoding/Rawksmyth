//! Drift estimation for distinct input/output devices (plan D-010, R-105,
//! T-031). Lives beside `clock` rather than in it: `clock.rs` is under the
//! freeze manifest.
//!
//! With one duplex stream on one device (A-011) input and output share a
//! frame counter and none of this is needed. With two devices the counters
//! run at slightly different true rates (typically tens of ppm), so a fixed
//! frame offset drifts by ≈15 ms over 300 s at 50 ppm. Each device's callbacks
//! publish `(monotonic_ns, frames)` samples; a long-window least-squares fit
//! per device gives its rate, and an input frame index is mapped to the output
//! frame counter at the same wall-clock instant.

use crate::clock::{ClockModel, ClockSample, Frame, RenderClockFit};
use std::collections::VecDeque;

/// Number of most-recent callback samples the game thread fits to build the
/// render clock (`RenderClockFit::fit`). Plan §2B.1d says 32, but 32 samples
/// cannot achieve the ≤1 ms error of T-024 under ±2 ms jitter (measured ≈
/// 1.7 ms worst case); 128 can. See `gtaudio/tests/t024_t031_clock.rs`.
pub const RENDER_FIT_WINDOW: usize = 128;

/// Long-window fit of `frames = alpha + beta · t_ns`.
pub struct ClockTracker {
    buf: VecDeque<ClockSample>,
    cap: usize,
    scratch: Vec<ClockSample>,
}

impl ClockTracker {
    pub fn new(cap: usize) -> Self {
        ClockTracker { buf: VecDeque::with_capacity(cap), cap, scratch: Vec::with_capacity(cap) }
    }

    pub fn push(&mut self, s: ClockSample) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(s);
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn fit(&mut self) -> Option<RenderClockFit> {
        self.scratch.clear();
        self.scratch.extend(self.buf.iter().copied());
        RenderClockFit::fit(&self.scratch)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DriftFit {
    input: RenderClockFit,
    output: RenderClockFit,
}

impl DriftFit {
    /// Input frames per output frame, minus one, in parts per million. Both
    /// devices are assumed to be *nominally* at the same sample rate.
    pub fn drift_ppm(&self) -> f64 {
        (self.input.beta / self.output.beta - 1.0) * 1e6
    }

    /// Output frame counter (fractional) at the wall-clock instant input frame
    /// `i` was captured.
    pub fn input_to_output_frame(&self, i: f64) -> f64 {
        let t_ns = (i - self.input.alpha) / self.input.beta;
        self.output.alpha + self.output.beta * t_ns
    }
}

pub struct DriftEstimator {
    input: ClockTracker,
    output: ClockTracker,
    warn_ppm: f64,
}

/// Default warning threshold: above what the estimator's own noise produces
/// for a single shared device, below typical crystal mismatch.
pub const DEFAULT_WARN_PPM: f64 = 20.0;
/// Samples per device kept for the fit (≈ 80 s of 10 ms callbacks).
pub const DEFAULT_WINDOW: usize = 8192;

impl Default for DriftEstimator {
    fn default() -> Self {
        Self::new(DEFAULT_WINDOW, DEFAULT_WARN_PPM)
    }
}

impl DriftEstimator {
    pub fn new(window: usize, warn_ppm: f64) -> Self {
        DriftEstimator { input: ClockTracker::new(window), output: ClockTracker::new(window), warn_ppm }
    }

    pub fn push_input(&mut self, s: ClockSample) {
        self.input.push(s);
    }

    pub fn push_output(&mut self, s: ClockSample) {
        self.output.push(s);
    }

    pub fn fit(&mut self) -> Option<DriftFit> {
        Some(DriftFit { input: self.input.fit()?, output: self.output.fit()? })
    }

    /// The UI warning required by D-010 when the devices' clocks differ.
    pub fn warning(&mut self) -> Option<String> {
        let ppm = self.fit()?.drift_ppm();
        (ppm.abs() > self.warn_ppm).then(|| {
            format!(
                "Input and output devices run on different clocks (measured drift {ppm:+.1} ppm); \
                 timing is corrected automatically, but using one device for both is more accurate."
            )
        })
    }

    /// Drift-corrected song time of input frame `i` (§2B.1d with the input
    /// frame first mapped onto the output frame counter).
    pub fn song_time(&mut self, clock: &ClockModel, i: Frame) -> Option<f64> {
        let out = self.fit()?.input_to_output_frame(i as f64);
        Some((out - clock.s0 as f64) / clock.fs - clock.l_rt())
    }
}

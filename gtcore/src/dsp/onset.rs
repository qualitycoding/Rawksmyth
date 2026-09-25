//! Onset detector (plan step S-004): 512-frame / 128-hop log-magnitude
//! spectral flux, peak-picked with a 3-hop lookahead, then *envelope
//! backtracked* to the sample where the attack actually starts (§1.4,
//! claim C-005).
//!
//! Causal and block-size independent: the output for a given input stream is
//! identical whether it is fed in 64-, 128- or 4096-frame blocks (checked by
//! a test). Latency from true onset to report is ≈ 512/fs + 3·128/fs
//! (`L_onset`, §2B.1b).

use super::fft::Fft;
use super::{Frame, Onset, OnsetDetector};
use std::f64::consts::PI;

#[derive(Debug, Clone)]
pub struct OnsetConfig {
    pub frame: usize,
    pub hop: usize,
    /// Highest analysed frequency for the flux sum (Hz).
    pub max_hz: f64,
    pub fs: f64,
    /// Log compression: L = ln(1 + gamma·|X|/Σw).
    pub gamma: f64,
    /// Max-filter radius (bins) applied to the previous frame's log spectrum.
    pub max_filter: usize,
    /// Peak-picking lookahead, in hops.
    pub lookahead: usize,
    /// Absolute flux floor: peaks below this are never onsets (silence and
    /// noise-floor fluctuation guard, T-005).
    pub thr_floor: f64,
    /// Adaptive part: peak must exceed `thr_floor + thr_ratio · mean(recent
    /// flux)`; `recent` excludes the rising edge of the candidate itself.
    pub thr_ratio: f64,
    /// Minimum spacing between reported onsets, in frames.
    pub refractory: u64,
}

impl Default for OnsetConfig {
    fn default() -> Self {
        OnsetConfig {
            frame: 512,
            hop: 128,
            max_hz: 6000.0,
            fs: 44100.0,
            gamma: 1000.0,
            max_filter: 1,
            lookahead: 3,
            thr_floor: 4.0,
            thr_ratio: 2.0,
            refractory: 1764, // 40 ms
        }
    }
}

/// Stride and length of the short-time energy used for envelope backtracking.
const ENV_LEN: usize = 64;
const ENV_STRIDE: usize = 16;
const KEEP_FRAMES: usize = 8192;

pub struct SpectralFluxOnset {
    cfg: OnsetConfig,
    fft: Fft,
    window: Vec<f64>,
    win_sum: f64,
    max_bin: usize,
    re: Vec<f64>,
    im: Vec<f64>,
    prev_log: Vec<f64>,
    cur_log: Vec<f64>,
    have_prev: bool,
    /// Raw history; `buf[0]` is frame `buf_start`.
    buf: Vec<f32>,
    buf_start: Frame,
    /// End frame (exclusive) of the next analysis window.
    next_end: Frame,
    /// (window end, flux) for recent hops.
    flux: Vec<(Frame, f64)>,
    last_onset: Option<Frame>,
    started: bool,
    /// Scratch for backtracking energies.
    env_db: Vec<f64>,
}

impl SpectralFluxOnset {
    pub fn new(cfg: OnsetConfig) -> Self {
        let n = cfg.frame;
        let window: Vec<f64> =
            (0..n).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f64 / n as f64).cos()).collect();
        let win_sum = window.iter().sum();
        let max_bin = ((cfg.max_hz / cfg.fs * n as f64) as usize).min(n / 2 - 1).max(1);
        SpectralFluxOnset {
            fft: Fft::new(n),
            window,
            win_sum,
            max_bin,
            re: vec![0.0; n],
            im: vec![0.0; n],
            prev_log: vec![0.0; max_bin + 1],
            cur_log: vec![0.0; max_bin + 1],
            have_prev: false,
            buf: Vec::new(),
            buf_start: 0,
            next_end: 0,
            flux: Vec::new(),
            last_onset: None,
            started: false,
            env_db: Vec::new(),
            cfg,
        }
    }

    /// Recent (window end, flux) pairs; exposed for calibration tooling.
    #[doc(hidden)]
    pub fn flux_history(&self) -> &[(Frame, f64)] {
        &self.flux
    }

    fn buf_end(&self) -> Frame {
        self.buf_start + self.buf.len() as Frame
    }

    fn reset_at(&mut self, first: Frame) {
        self.buf.clear();
        self.buf_start = first;
        self.next_end = first + self.cfg.frame as Frame;
        self.have_prev = false;
        self.flux.clear();
        self.last_onset = None;
        self.started = true;
    }

    /// First difference (pre-emphasis): suppresses the low-frequency tail
    /// of ringing notes so the attack transient dominates the envelope.
    fn sample(&self, frame: Frame) -> f64 {
        let i = (frame - self.buf_start) as usize;
        self.buf[i] as f64 - self.buf[i - 1] as f64
    }

    /// Log-magnitude spectrum of the window ending at `end`; returns the
    /// half-wave-rectified flux against the previous window, whose
    /// log-magnitudes are max-filtered over ±`max_filter` bins (SuperFlux,
    /// Böck & Widmer 2013) so that beating between closely spaced partials of
    /// ringing low notes does not register as new energy.
    fn analyse(&mut self, end: Frame) -> f64 {
        let n = self.cfg.frame;
        let start = (end - n as Frame - self.buf_start) as usize;
        for i in 0..n {
            self.re[i] = self.buf[start + i] as f64 * self.window[i];
            self.im[i] = 0.0;
        }
        self.fft.forward(&mut self.re, &mut self.im);
        for k in 1..=self.max_bin {
            let mag = (self.re[k] * self.re[k] + self.im[k] * self.im[k]).sqrt();
            self.cur_log[k] = (1.0 + self.cfg.gamma * mag / self.win_sum).ln();
        }
        let mut flux = 0.0;
        if self.have_prev {
            let r = self.cfg.max_filter;
            for k in 1..=self.max_bin {
                let lo = k.saturating_sub(r).max(1);
                let hi = (k + r).min(self.max_bin);
                let prev = self.prev_log[lo..=hi].iter().fold(0.0f64, |m, &v| m.max(v));
                let d = self.cur_log[k] - prev;
                if d > 0.0 {
                    flux += d;
                }
            }
        }
        std::mem::swap(&mut self.cur_log, &mut self.prev_log);
        self.have_prev = true;
        flux
    }

    fn try_pick(&mut self, out: &mut Vec<Onset>) {
        let la = self.cfg.lookahead;
        if self.flux.len() < la + 1 {
            return;
        }
        let c = self.flux.len() - 1 - la;
        let (end_c, f_c) = self.flux[c];
        // Local maximum over ±lookahead hops (strict on the right so a flat
        // top yields one pick).
        for j in 1..=la {
            if self.flux[c + j].1 >= f_c {
                return;
            }
            if c >= j && self.flux[c - j].1 > f_c {
                return;
            }
        }
        // Adaptive threshold over the hops before the candidate's rising edge.
        let hi = c.saturating_sub(la + 1);
        let lo = c.saturating_sub(12 + la + 1);
        let (mut sum, mut cnt) = (0.0, 0usize);
        if c > la {
            for i in lo..=hi {
                sum += self.flux[i].1;
                cnt += 1;
            }
        }
        let mean = if cnt > 0 { sum / cnt as f64 } else { 0.0 };
        if f_c <= self.cfg.thr_floor + self.cfg.thr_ratio * mean {
            return;
        }
        let frame = self.backtrack(end_c);
        if let Some(last) = self.last_onset {
            if frame < last + self.cfg.refractory {
                return;
            }
        }
        self.last_onset = Some(frame);
        out.push(Onset { frame, strength: f_c as f32 });
    }

    /// Envelope backtracking. Finds the steepest rise of the 64-sample
    /// short-time energy (dB) in the region the flux peak's window can
    /// contain an attack in, then localizes the first sample of the attack
    /// by an amplitude-step test against the pre-attack level.
    fn backtrack(&mut self, end: Frame) -> Frame {
        let fallback = end.saturating_sub((self.cfg.frame / 2 + self.cfg.hop) as Frame);
        let region_lo = end
            .saturating_sub((self.cfg.frame + self.cfg.hop) as Frame)
            .max(self.buf_start + ENV_STRIDE as Frame);
        let region_hi = end.saturating_sub(ENV_LEN as Frame);
        if region_hi <= region_lo + 2 * ENV_STRIDE as Frame {
            return fallback.max(self.buf_start);
        }
        // Energies at window starts t_i = region_lo + i·stride.
        self.env_db.clear();
        let mut t = region_lo;
        while t <= region_hi {
            let b = (t - self.buf_start) as usize;
            let e: f64 = (b..b + ENV_LEN).map(|k| { let d = self.buf[k] as f64 - self.buf[k - 1] as f64; d * d }).sum();
            self.env_db.push(10.0 * (e + 1e-10).log10());
            t += ENV_STRIDE as Frame;
        }
        let (mut best_i, mut best_rise) = (0usize, f64::MIN);
        for i in 0..self.env_db.len() - 1 {
            let rise = self.env_db[i + 1] - self.env_db[i];
            if rise > best_rise {
                best_rise = rise;
                best_i = i;
            }
        }
        if best_rise <= 0.0 {
            return fallback.max(self.buf_start);
        }
        // Window i is the last one *before* the attack entered window i+1's
        // final stride, so the attack starts in [t_i + ENV_LEN, t_i + ENV_LEN
        // + stride); scan a slightly wider range.
        let t_i = region_lo + (best_i * ENV_STRIDE) as Frame;
        let scan_lo = t_i + (ENV_LEN - ENV_STRIDE) as Frame;
        let scan_hi = (scan_lo + 160).min(self.buf_end());
        let pre_lo = scan_lo.saturating_sub(ENV_LEN as Frame).max(self.buf_start + 1);
        let mut pre = 0.0f64;
        for f in pre_lo..scan_lo {
            pre = pre.max(self.sample(f).abs());
        }
        let mut peak = 0.0f64;
        for f in scan_lo..scan_hi {
            peak = peak.max(self.sample(f).abs());
        }
        let thr = (0.1 * peak).max(2.0 * pre).max(1e-6);
        for f in scan_lo..scan_hi {
            if self.sample(f).abs() >= thr {
                return f;
            }
        }
        scan_lo
    }
}

impl OnsetDetector for SpectralFluxOnset {
    fn process(&mut self, block: &[f32], first: Frame, out: &mut Vec<Onset>) {
        if !self.started || first != self.buf_end() {
            self.reset_at(first);
        }
        self.buf.extend_from_slice(block);
        let end = self.buf_end();
        while self.next_end <= end {
            let e = self.next_end;
            let f = self.analyse(e);
            self.flux.push((e, f));
            if self.flux.len() > 64 {
                self.flux.remove(0);
            }
            self.try_pick(out);
            self.next_end += self.cfg.hop as Frame;
        }
        if self.buf.len() > 2 * KEEP_FRAMES {
            let drop = self.buf.len() - KEEP_FRAMES;
            self.buf.drain(..drop);
            self.buf_start += drop as Frame;
        }
    }
}

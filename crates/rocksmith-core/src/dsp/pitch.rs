//! Multi-rate McLeod Pitch Method (MPM) and Normalized Square Difference Function (NSDF).
//! Supports low-register E2 (82.41 Hz) up to E5 (659.26 Hz) with <= 10 cent accuracy.

use crate::domain::{PitchDetector, PitchResult};
use crate::dsp::filter::{ButterworthLowPass, Decimator4x};

#[derive(Debug, Clone)]
pub struct MultiRatePitchDetector {
    sample_rate: f32,
    window_size: usize,
    hop_size: usize,
    energy_threshold: f32,
    clarity_threshold: f32,
    conditioning: ButterworthLowPass,
    decimator: Decimator4x,
    decimated_buffer: Vec<f32>,
    frame_counter: u64,
}

impl MultiRatePitchDetector {
    pub fn new(sample_rate: f32, window_size: usize, hop_size: usize) -> Self {
        Self {
            sample_rate,
            window_size,
            hop_size,
            energy_threshold: 0.005, // Silence threshold
            clarity_threshold: 0.70, // NSDF peak clarity threshold
            conditioning: ButterworthLowPass::new(3500.0, sample_rate),
            decimator: Decimator4x::new(sample_rate),
            decimated_buffer: Vec::with_capacity(window_size / 4),
            frame_counter: 0,
        }
    }

    /// Compute the Normalized Square Difference Function (NSDF):
    /// r_t(\tau) = \frac{2 \sum x_j x_{j+\tau}}{\sum x_j^2 + \sum x_{j+\tau}^2}
    fn compute_nsdf(buffer: &[f32], max_tau: usize) -> Vec<f32> {
        let n = buffer.len();
        let mut nsdf = vec![0.0f32; max_tau];
        if n < max_tau {
            return nsdf;
        }

        for tau in 0..max_tau {
            let mut acf = 0.0f32;
            let mut m0 = 0.0f32;
            let mut m_tau = 0.0f32;

            for j in 0..(n - tau) {
                let s0 = buffer[j];
                let st = buffer[j + tau];
                acf += s0 * st;
                m0 += s0 * s0;
                m_tau += st * st;
            }

            let m = m0 + m_tau;
            nsdf[tau] = if m > 1e-8 { (2.0 * acf) / m } else { 0.0 };
        }

        nsdf
    }

    /// Parabolic interpolation around peak index
    fn parabolic_interpolation(nsdf: &[f32], peak_idx: usize) -> (f32, f32) {
        if peak_idx == 0 || peak_idx >= nsdf.len() - 1 {
            return (peak_idx as f32, nsdf[peak_idx]);
        }

        let alpha = nsdf[peak_idx - 1];
        let beta = nsdf[peak_idx];
        let gamma = nsdf[peak_idx + 1];

        let denom = 2.0 * (alpha - 2.0 * beta + gamma);
        if denom.abs() < 1e-6 {
            return (peak_idx as f32, beta);
        }

        let delta = (alpha - gamma) / denom;
        let turning_point = peak_idx as f32 + delta;
        let peak_value = beta - 0.25 * (alpha - gamma) * delta;

        (turning_point, peak_value)
    }

    /// Run MPM pitch detection on arbitrary sample buffer
    pub fn detect_pitch_raw(&self, buffer: &[f32], sr: f32, min_freq: f32, max_freq: f32) -> Option<(f32, f32)> {
        // Compute RMS energy
        let rms = (buffer.iter().map(|&s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();
        if rms < self.energy_threshold {
            return None; // Silence
        }

        let min_tau = (sr / max_freq).floor() as usize;
        let max_tau = (sr / min_freq).ceil() as usize;

        if max_tau >= buffer.len() {
            return None;
        }

        let nsdf = Self::compute_nsdf(buffer, max_tau + 2);

        // Find candidate peaks after initial zero-crossing
        let mut zero_crossed = false;
        let mut best_tau = 0.0f32;
        let mut max_clarity = 0.0f32;

        let mut i = 1;
        while i < max_tau {
            if nsdf[i] < 0.0 {
                zero_crossed = true;
            }
            if zero_crossed && nsdf[i] > 0.0 && nsdf[i] >= nsdf[i - 1] && nsdf[i] >= nsdf[i + 1] {
                // Found local maximum
                let (interpolated_tau, clarity) = Self::parabolic_interpolation(&nsdf, i);
                if clarity > self.clarity_threshold {
                    // Pick the first significant peak (avoiding octave multiples)
                    return Some((sr / interpolated_tau, clarity));
                }
                if clarity > max_clarity {
                    max_clarity = clarity;
                    best_tau = interpolated_tau;
                }
            }
            i += 1;
        }

        if max_clarity >= self.clarity_threshold && best_tau > 0.0 {
            Some((sr / best_tau, max_clarity))
        } else {
            None
        }
    }
}

impl PitchDetector for MultiRatePitchDetector {
    fn process(&mut self, samples: &[f32]) -> Option<PitchResult> {
        self.frame_counter += 1;
        let timestamp_ms = (self.frame_counter as f64 * self.hop_size as f64 * 1000.0 / self.sample_rate as f64) as u64;

        // Step 1: Conditioning Filter
        let mut conditioned = vec![0.0f32; samples.len()];
        self.conditioning.process_block(samples, &mut conditioned);

        // Step 2: Full-rate detection for Mid-to-High frequencies (150 Hz - 700 Hz)
        if let Some((freq, clarity)) = self.detect_pitch_raw(&conditioned, self.sample_rate, 140.0, 750.0) {
            return Some(PitchResult {
                frequency_hz: freq,
                confidence: clarity,
                timestamp_ms,
                is_voiced: true,
            });
        }

        // Step 3: Multi-rate 4x Decimation for Low Register (E2 82.41 Hz to ~160 Hz)
        self.decimator.process(&conditioned, &mut self.decimated_buffer);
        let decimated_sr = self.sample_rate / 4.0;

        if let Some((low_freq, low_clarity)) = self.detect_pitch_raw(&self.decimated_buffer, decimated_sr, 75.0, 180.0) {
            return Some(PitchResult {
                frequency_hz: low_freq,
                confidence: low_clarity,
                timestamp_ms,
                is_voiced: true,
            });
        }

        None
    }

    fn reset(&mut self) {
        self.conditioning.reset();
        self.decimator.reset();
        self.frame_counter = 0;
    }
}

//! Adaptive spectral flux and high-frequency energy onset detector.
//! Implements refractory lockout (40ms) and dynamic thresholding.

use crate::domain::{OnsetDetector, OnsetResult};

#[derive(Debug, Clone)]
pub struct SpectralFluxOnsetDetector {
    sample_rate: f32,
    hop_size: usize,
    prev_magnitude: Vec<f32>,
    moving_avg_energy: f32,
    threshold_multiplier: f32,
    refractory_samples: usize,
    samples_since_last_onset: usize,
    elapsed_samples: u64,
}

impl SpectralFluxOnsetDetector {
    pub fn new(sample_rate: f32, fft_size: usize, hop_size: usize) -> Self {
        Self {
            sample_rate,
            hop_size,
            prev_magnitude: vec![0.0f32; fft_size / 2],
            moving_avg_energy: 0.001,
            threshold_multiplier: 1.8,
            refractory_samples: ((sample_rate * 0.040).round()) as usize, // 40ms refractory gate
            samples_since_last_onset: 100000,
            elapsed_samples: 0,
        }
    }

    /// Compute half-wave rectified spectral difference
    fn compute_spectral_flux(&mut self, current_mag: &[f32]) -> f32 {
        let mut flux = 0.0f32;
        let n = self.prev_magnitude.len().min(current_mag.len());
        for i in 0..n {
            let diff = current_mag[i] - self.prev_magnitude[i];
            if diff > 0.0 {
                // Half-wave rectification: sum only energy increases
                flux += diff;
            }
            self.prev_magnitude[i] = current_mag[i];
        }
        flux
    }
}

impl OnsetDetector for SpectralFluxOnsetDetector {
    fn process(&mut self, samples: &[f32]) -> Option<OnsetResult> {
        self.elapsed_samples += samples.len() as u64;
        self.samples_since_last_onset += samples.len();

        // Calculate time-domain energy increase & high-frequency content proxy
        let mut energy = 0.0f32;
        let mut hfc = 0.0f32;
        for (i, &s) in samples.iter().enumerate() {
            let abs_s = s.abs();
            energy += abs_s * abs_s;
            if i > 0 {
                // First-order high-frequency difference proxy
                let diff = (s - samples[i - 1]).abs();
                hfc += diff * diff;
            }
        }
        energy /= samples.len().max(1) as f32;
        hfc /= samples.len().max(1) as f32;

        // Dynamic threshold tracking
        let dynamic_threshold = (self.moving_avg_energy * self.threshold_multiplier).max(0.002);
        self.moving_avg_energy = 0.92 * self.moving_avg_energy + 0.08 * energy;

        let timestamp_ms = (self.elapsed_samples as f64 * 1000.0 / self.sample_rate as f64) as u64;

        // Check trigger condition with refractory gate
        if self.samples_since_last_onset >= self.refractory_samples {
            if energy > dynamic_threshold && hfc > dynamic_threshold * 0.5 {
                self.samples_since_last_onset = 0;
                return Some(OnsetResult {
                    timestamp_ms,
                    strength: (energy / dynamic_threshold).min(5.0),
                });
            }
        }

        None
    }

    fn reset(&mut self) {
        self.prev_magnitude.fill(0.0);
        self.moving_avg_energy = 0.001;
        self.samples_since_last_onset = 100000;
        self.elapsed_samples = 0;
    }
}

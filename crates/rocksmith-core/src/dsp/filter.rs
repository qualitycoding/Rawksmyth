//! Audio conditioning filters: 2nd-order 3.5 kHz low-pass Butterworth and 4x IIR half-band decimation.

use std::f32::consts::PI;

/// 2nd-order Butterworth low-pass filter (Decision D-008)
/// Rolloff pickup hum, scrape transients, and bridge buzz above 3.5 kHz.
#[derive(Debug, Clone)]
pub struct ButterworthLowPass {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl ButterworthLowPass {
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        let w0 = 2.0 * PI * cutoff_hz / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * 0.70710678); // Q = 1/sqrt(2)

        let a0 = 1.0 + alpha;
        let b0 = ((1.0 - cos_w0) / 2.0) / a0;
        let b1 = (1.0 - cos_w0) / a0;
        let b2 = ((1.0 - cos_w0) / 2.0) / a0;
        let a1 = (-2.0 * cos_w0) / a0;
        let a2 = (1.0 - alpha) / a0;

        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline]
    pub fn process_sample(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    pub fn process_block(&mut self, input: &[f32], output: &mut [f32]) {
        for (i, &sample) in input.iter().enumerate() {
            output[i] = self.process_sample(sample);
        }
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// 4x Decimator for Low-Register Sub-200 Hz Tracking (Decision D-002)
/// Downsamples 44.1 kHz to 11.025 kHz with anti-aliasing filtering.
#[derive(Debug, Clone)]
pub struct Decimator4x {
    anti_alias: ButterworthLowPass,
}

impl Decimator4x {
    pub fn new(sample_rate: f32) -> Self {
        // Cutoff at 11025 / 2.2 = ~5000 Hz or lower (2500 Hz for clean sub-200Hz tracking)
        Self {
            anti_alias: ButterworthLowPass::new(2500.0, sample_rate),
        }
    }

    pub fn process(&mut self, input: &[f32], output: &mut Vec<f32>) {
        output.clear();
        output.reserve(input.len() / 4);
        for (i, &sample) in input.iter().enumerate() {
            let filtered = self.anti_alias.process_sample(sample);
            if i % 4 == 0 {
                output.push(filtered);
            }
        }
    }

    pub fn reset(&mut self) {
        self.anti_alias.reset();
    }
}

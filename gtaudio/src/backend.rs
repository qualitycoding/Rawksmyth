//! `AudioBackend` and the deterministic file backend (plan Â§2B.1e "File
//! backend"): drives the same callback the live device would, faster than
//! real time, from an in-memory input signal. A real cubeb backend implements
//! the same trait (not built here â€” it needs an audio interface).

use gtcore::clock::ClockSample;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    /// The device vanished (unplugged, driver reset) at this input frame.
    DeviceLost { at_frame: u64 },
    Other(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::DeviceLost { at_frame } => write!(f, "audio device lost at frame {at_frame}"),
            BackendError::Other(s) => write!(f, "audio backend error: {s}"),
        }
    }
}

impl std::error::Error for BackendError {}

/// Duplex callback: `(input mono block, output interleaved block, monotonic_ns)`.
pub type DuplexCallback<'a> = dyn FnMut(&[f32], &mut [f32], i64) + 'a;

pub trait AudioBackend {
    fn sample_rate(&self) -> f64;
    /// Input latency reported by the driver at stream start, seconds (L_in).
    fn input_latency_s(&self) -> f64;
    /// Output latency reported by the driver, seconds (L_out).
    fn output_latency_s(&self) -> f64;
    fn output_channels(&self) -> usize;
    /// Run the duplex stream, invoking `cb` per block, until the input ends or
    /// the device fails. Returns the number of frames delivered.
    fn run(&mut self, cb: &mut DuplexCallback<'_>) -> Result<u64, BackendError>;
}

/// Tiny deterministic generator (xorshift64*), independent of any other crate.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn bipolar(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let u = (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64;
        u * 2.0 - 1.0
    }
}

pub struct FileBackend {
    input: Vec<f32>,
    fs: f64,
    block: usize,
    channels: usize,
    l_in: f64,
    l_out: f64,
    jitter_ns: f64,
    seed: u64,
    fail_at: Option<u64>,
    tail_frames: u64,
    captured: Vec<f32>,
}

impl FileBackend {
    /// `input`: mono samples as the device would capture them, starting at
    /// frame 0 of the stream.
    pub fn new(input: Vec<f32>, sample_rate: f64) -> Self {
        FileBackend {
            input,
            fs: sample_rate,
            block: 256, // the plan's requested input buffer (Â§0.3.1)
            channels: 2,
            l_in: 0.0,
            l_out: 0.0,
            jitter_ns: 0.0,
            seed: 1,
            fail_at: None,
            tail_frames: 0,
            captured: Vec::new(),
        }
    }

    pub fn block_size(mut self, frames: usize) -> Self {
        self.block = frames.max(1);
        self
    }
    pub fn channels(mut self, c: usize) -> Self {
        self.channels = c.max(1);
        self
    }
    pub fn latencies(mut self, l_in_s: f64, l_out_s: f64) -> Self {
        self.l_in = l_in_s;
        self.l_out = l_out_s;
        self
    }
    /// Â± uniform scheduling jitter applied to each callback's timestamp.
    pub fn jitter(mut self, jitter_ns: f64, seed: u64) -> Self {
        self.jitter_ns = jitter_ns;
        self.seed = seed;
        self
    }
    /// Inject a device loss: the callback that would start at or after this
    /// input frame fails instead (T-011).
    pub fn fail_at_frame(mut self, frame: u64) -> Self {
        self.fail_at = Some(frame);
        self
    }
    /// Keep the stream running with silent input for this many frames after
    /// the input signal ends (lets the last notes' evidence complete).
    pub fn tail_frames(mut self, frames: u64) -> Self {
        self.tail_frames = frames;
        self
    }

    /// Everything the callback wrote to the output (interleaved).
    pub fn captured_output(&self) -> &[f32] {
        &self.captured
    }
}

impl AudioBackend for FileBackend {
    fn sample_rate(&self) -> f64 {
        self.fs
    }
    fn input_latency_s(&self) -> f64 {
        self.l_in
    }
    fn output_latency_s(&self) -> f64 {
        self.l_out
    }
    fn output_channels(&self) -> usize {
        self.channels
    }

    fn run(&mut self, cb: &mut DuplexCallback<'_>) -> Result<u64, BackendError> {
        self.captured.clear();
        let total = self.input.len() as u64 + self.tail_frames;
        let mut rng = Rng::new(self.seed);
        let mut silent = vec![0.0f32; self.block];
        let mut out = vec![0.0f32; self.block * self.channels];
        let mut pos: u64 = 0;
        while pos < total {
            if let Some(f) = self.fail_at {
                if pos >= f {
                    return Err(BackendError::DeviceLost { at_frame: pos });
                }
            }
            let n = ((total - pos) as usize).min(self.block);
            let start = pos as usize;
            let block: &[f32] = if start < self.input.len() {
                let end = (start + n).min(self.input.len());
                if end - start == n {
                    &self.input[start..end]
                } else {
                    // Straddles the end of the input: pad with silence.
                    silent[..n].fill(0.0);
                    silent[..end - start].copy_from_slice(&self.input[start..end]);
                    &silent[..n]
                }
            } else {
                silent[..n].fill(0.0);
                &silent[..n]
            };
            // Callback timestamp: when the block was completed, plus jitter.
            let t_ns = ((pos + n as u64) as f64 / self.fs * 1e9 + rng.bipolar() * self.jitter_ns) as i64;
            let o = &mut out[..n * self.channels];
            cb(block, o, t_ns);
            self.captured.extend_from_slice(o);
            pos += n as u64;
        }
        Ok(pos)
    }
}

/// Simulate the two independent clocks of separate input and output devices
/// (D-010, R-105): device `d` really runs at `rate_hz` while both nominally
/// run at the same rate. Each device delivers `block`-frame callbacks; every
/// callback's `(monotonic_ns, cumulative frames)` sample is timestamped with
/// Â± `jitter_ns` scheduling jitter. Returns (input samples, output samples).
pub fn simulate_two_devices(
    seconds: f64,
    in_hz: f64,
    out_hz: f64,
    block: u64,
    jitter_ns: f64,
    seed: u64,
) -> (Vec<ClockSample>, Vec<ClockSample>) {
    let mut rng = Rng::new(seed);
    let gen = |hz: f64, rng: &mut Rng| {
        let n = (seconds * hz / block as f64) as u64;
        (1..=n)
            .map(|k| ClockSample {
                monotonic_ns: ((k * block) as f64 / hz * 1e9 + rng.bipolar() * jitter_ns) as i64,
                frames_written: k * block,
            })
            .collect::<Vec<_>>()
    };
    let input = gen(in_hz, &mut rng);
    let output = gen(out_hz, &mut rng);
    (input, output)
}

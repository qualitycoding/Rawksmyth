# Architectural & Engineering Decisions (plan/DECISIONS.md)

## 1. Core Interfaces & Traits

```rust
pub struct AudioFrame {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub timestamp_samples: u64,
}

pub struct PitchResult {
    pub frequency_hz: f32,
    pub confidence: f32,
    pub timestamp_ms: u64,
    pub is_voiced: bool,
}

pub struct OnsetResult {
    pub timestamp_ms: u64,
    pub strength: f32,
}

pub trait PitchDetector: Send {
    /// Process incoming audio samples and emit pitch estimation
    fn process(&mut self, samples: &[f32]) -> Option<PitchResult>;
    fn reset(&mut self);
}

pub trait OnsetDetector: Send {
    /// Process incoming audio samples and detect attack transients
    fn process(&mut self, samples: &[f32]) -> Option<OnsetResult>;
    fn reset(&mut self);
}

/// Bridges the physical discrepancy between instantaneous onset transients
/// and stabilized fundamental pitch tracking (reconciles pick attack noise).
pub trait TemporalNoteTracker: Send {
    fn record_onset(&mut self, onset: OnsetResult);
    fn record_pitch(&mut self, pitch: PitchResult);
    /// Evaluate candidate notes once consensus window (20-40ms) has elapsed
    fn poll_resolved_notes(&mut self, current_time_ms: u64) -> Vec<ResolvedNoteEvent>;
}

pub struct ResolvedNoteEvent {
    pub onset_timestamp_ms: u64,
    pub resolved_frequency_hz: f32,
    pub confidence: f32,
    pub midi_note: u8,
    pub cents_deviation: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judgment {
    Perfect, // <= 15ms, <= 10 cents
    Great,   // <= 30ms, <= 20 cents
    Good,    // <= 50ms, <= 35 cents
    Miss,    // > 50ms or wrong pitch
}

pub struct NoteTarget {
    pub id: u64,
    pub target_timestamp_ms: u64,
    pub expected_midi_note: u8,
    pub string_index: u8,
    pub fret_number: u8,
    pub duration_ms: u32,
}

pub trait Scorer: Send {
    fn register_target(&mut self, target: NoteTarget);
    fn evaluate_event(&mut self, event: &ResolvedNoteEvent) -> Option<(u64, Judgment)>;
    fn check_expired_targets(&mut self, current_time_ms: u64) -> Vec<(u64, Judgment)>;
    fn get_stats(&self) -> SessionScoreStats;
}

pub struct SessionScoreStats {
    pub total_notes: u32,
    pub perfect_count: u32,
    pub great_count: u32,
    pub good_count: u32,
    pub miss_count: u32,
    pub current_streak: u32,
    pub max_streak: u32,
    pub accuracy_percentage: f32,
}
```

---

## 2. Decision Index

### D-001: Audio Engine & Duplex Stream
- **Decision:** Audio I/O is managed exclusively via `cubeb` (or `cpal` fallback) in a **full-duplex stream** (simultaneous input capture and output playback on the same hardware buffer clock).
- **Rationale:** Prevents clock drift between song playback and input monitoring. Sample clock is strictly derived from the hardware callback frame counter.

### D-002: Multi-Rate Pitch Detection (Sub-200 Hz Decimation)
- **Decision:** For low-register notes ($E_2 \approx 82.41\text{ Hz}$ to $B_2 \approx 123.47\text{ Hz}$), input audio is downsampled $4\times$ (to $11.025\text{ kHz}$) using an optimized 4th-order IIR half-band decimation filter.
- **Rationale:** A 1024-sample window at $11.025\text{ kHz}$ captures the identical physical duration ($92.8\text{ ms}$) as a 4096-sample window at $44.1\text{ kHz}$, but with $4\times$ lower compute and zero algorithmic latency penalty, completely preventing octave doubling/halving on $E_2$.
- **Algorithm:** Hybrid McLeod Pitch Method (MPM) / Normalized Square Difference Function (NSDF) with parabolic interpolation, falling back to YINFFT for upper registers.

### D-003: Engine Decoupling & GDExtension Boundary
- **Decision:** Godot 4.3 functions strictly as a headless-capable slave visual renderer. Godot's internal `AudioServer` is bypassed for real-time capture and game timing.
- **Rationale:** Godot's audio bus introduces buffer sizing optimized for desktop mix games ($>30\text{ ms}$ jitter).
- **Communication:** Communication between the real-time OS audio thread (Cubeb) and the Godot render loop is handled via a **lock-free SPSC (Single-Producer Single-Consumer) ring buffer** (`rtrb` crate in Rust). Zero memory allocation, zero locks, and zero syscalls occur in the audio callback.

### D-004: Adaptive Spectral Flux Onset Detection
- **Decision:** Onset detection utilizes spectral flux with dynamic thresholding based on moving-average RMS energy and a refractory lockout window ($40\text{ ms}$).
- **Rationale:** Avoids false re-triggers during string decay, finger vibrato, and bridge buzz.

### D-005: Low-Latency Mode & Windows Driver Strategy
- **Decision:** On Windows, WASAPI Exclusive Mode is requested by default for lowest achievable latency ($\le 10\text{ ms}$ buffer). If unavailable or rejected, graceful fallback to WASAPI Shared Mode is engaged with an automated prompt alerting the user to use the calibration tool.

### D-006: Transient/Pitch Reconciliation (Temporal Note Tracker)
- **Decision:** Implement `TemporalNoteTracker` to resolve the fundamental physical paradox of guitar pick attack transients.
- **Mechanism:**
  1. Onset detector fires at $t_0$, logging a candidate note.
  2. For the subsequent $15\text{ ms}$, raw pitch readings are discarded (broadband pick inharmonicity window).
  3. Over $15\text{--}35\text{ ms}$, stable pitch candidate frames are collected.
  4. Consensus pitch (median filtered with confidence weighting) is finalized and emitted as a `ResolvedNoteEvent`.

### D-007: Dual Latency Calibration Sliders
- **Decision:** Settings interface exposes two independent calibration parameters:
  1. **Audio Latency Offset (ms):** Compensates for audio interface driver input/output buffer round-trip.
  2. **Visual Display Offset (ms):** Compensates for display refresh rate, HDMI lag, and GPU double/triple buffering latency.

### D-008: DI Input Conditioning Filter
- **Decision:** Before routing to onset and pitch detection, the raw guitar DI signal passes through an analog-style 2nd-order low-pass filter at $3.5\text{ kHz}$.
- **Rationale:** Eliminates pickup hum harmonics, pick scrape high-frequency noise, and bridge fret buzz without removing fundamental or low-order harmonic guitar frequencies.

### D-009: Test Segregation for Headless CI
- **Decision:** 
  - `T-015a` is a deterministic, offline DSP throughput and latency benchmark running synthetic audio buffers in CI without audio hardware.
  - `T-015b` is a hardware loopback test gated behind `--profile manual-hardware` for local verification.
  - `T-011` (device disconnect) is validated via a `MockAudioBackend`.

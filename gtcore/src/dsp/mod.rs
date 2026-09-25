//! DSP interfaces and shared types (plan §2B.1a). Timestamps are frames.
//!
//! - `onset`: spectral-flux onset detector with envelope backtracking (S-004).
//! - `verifier`: chart-informed constrained-NSDF note verifier (S-005).

pub mod fft;
pub mod onset;
pub mod verifier;

use crate::chart::ChartNote;
pub use crate::clock::Frame;

/// A chart note mapped into input frames by the `ClockModel`.
///
/// `[open, close)` is the *onset acceptance window* (the judgment window of
/// D-008, ±50 ms around the expected onset), so its midpoint is the expected
/// onset frame. `sustain_s` is used only to clip the SC-1a refinement window.
#[derive(Debug, Clone)]
pub struct ExpectedNote {
    pub note: ChartNote,
    pub open: Frame,
    pub close: Frame,
}

impl ExpectedNote {
    pub fn center(&self) -> f64 {
        0.5 * (self.open as f64 + self.close as f64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Onset {
    pub frame: Frame,
    pub strength: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    Present,
    Absent,
    /// The signal is periodic at the k-th sub-lag: the player is sounding the
    /// k-th harmonic (or a note k× higher) of the charted pitch.
    HarmonicUp(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    Decision,
    Refinement,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoteEvidence {
    pub note_id: u32,
    pub kind: EvidenceKind,
    pub presence: Presence,
    /// The onset attributed to this note, if any.
    pub onset: Option<Frame>,
    /// Frame at which the last sample needed for this evidence became
    /// available. SC-1b latency is `decided_at − true onset`. (The onset
    /// detector's own latency, `L_onset`, is added by the pipeline, which
    /// takes `max(decided_at, frame the onset was reported)`.)
    pub decided_at: Frame,
    /// Decision: coarse median over the confirmation hops. Refinement: the
    /// SC-1a value (median over W_r = 2048 windows in the sustain).
    pub cents_error: Option<f32>,
    /// a(τ*) at the deciding hop; lower = more periodic.
    pub aperiodicity: f32,
}

pub trait OnsetDetector {
    fn process(&mut self, block: &[f32], first: Frame, out: &mut Vec<Onset>);
}

pub trait NoteVerifier {
    fn process(
        &mut self,
        block: &[f32],
        first: Frame,
        expected: &[ExpectedNote],
        onsets: &[Onset],
        out: &mut Vec<NoteEvidence>,
    );
}

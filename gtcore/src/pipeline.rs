//! Evidence pipeline (plan step S-006): onset detector + note verifier +
//! attribution, driven block by block. Deterministic and hardware-free — the
//! same code runs behind the file backend (tests) and, later, the DSP thread
//! of the live engine (§2B.1e).
//!
//! The pipeline maps chart notes into input frames with the `ClockModel`
//! (an `ExpectedNote` is the ±window around the note's expected onset),
//! feeds the detector and verifier, and stamps each piece of evidence's
//! `decided_at` with the frame at which the *whole* pipeline could have known
//! it: `max(verifier decided_at, frame at which the attributed onset was
//! reported)` — i.e. `L_onset` is included (SC-1b).

use crate::chart::ChartNote;
use crate::clock::{ClockModel, Frame};
use crate::dsp::onset::{OnsetConfig, SpectralFluxOnset};
use crate::dsp::verifier::{ConstrainedNsdfVerifier, VerifierConfig};
use crate::dsp::{ExpectedNote, NoteEvidence, NoteVerifier, Onset, OnsetDetector};

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Half-width of the onset acceptance window (D-008), ms.
    pub window_ms: f64,
    pub onset: OnsetConfig,
    pub verifier: VerifierConfig,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        PipelineConfig { window_ms: 50.0, onset: OnsetConfig::default(), verifier: VerifierConfig::default() }
    }
}

pub struct Pipeline {
    det: SpectralFluxOnset,
    ver: ConstrainedNsdfVerifier,
    /// Sorted by `open`.
    expected: Vec<ExpectedNote>,
    next_register: usize,
    /// (onset frame, frame at which it was reported).
    reported: Vec<(Frame, Frame)>,
    onsets: Vec<Onset>,
    fresh: Vec<ExpectedNote>,
    evidence: Vec<NoteEvidence>,
}

impl Pipeline {
    pub fn new(chart: &[ChartNote], clock: &ClockModel) -> Self {
        Self::with_config(chart, clock, PipelineConfig::default())
    }

    pub fn with_config(chart: &[ChartNote], clock: &ClockModel, cfg: PipelineConfig) -> Self {
        let w = (cfg.window_ms * 1e-3 * clock.fs).round() as Frame;
        let mut expected: Vec<ExpectedNote> = chart
            .iter()
            .map(|n| {
                let t = clock.frame_for_song_time(n.start_s);
                ExpectedNote { note: n.clone(), open: t.saturating_sub(w), close: t + w }
            })
            .collect();
        expected.sort_by_key(|e| (e.open, e.note.id));
        Pipeline {
            det: SpectralFluxOnset::new(cfg.onset),
            ver: ConstrainedNsdfVerifier::new(cfg.verifier),
            expected,
            next_register: 0,
            reported: Vec::new(),
            onsets: Vec::new(),
            fresh: Vec::new(),
            evidence: Vec::new(),
        }
    }

    pub fn expected(&self) -> &[ExpectedNote] {
        &self.expected
    }

    /// Feed the next contiguous block of input (`first` = frame index of its
    /// first sample). Evidence produced is appended to `out`.
    pub fn process(&mut self, block: &[f32], first: Frame, out: &mut Vec<NoteEvidence>) {
        let end = first + block.len() as Frame;

        self.onsets.clear();
        self.det.process(block, first, &mut self.onsets);
        for o in &self.onsets {
            self.reported.push((o.frame, end));
        }

        self.fresh.clear();
        while self.next_register < self.expected.len() && self.expected[self.next_register].open <= end {
            self.fresh.push(self.expected[self.next_register].clone());
            self.next_register += 1;
        }

        self.evidence.clear();
        self.ver.process(block, first, &self.fresh, &self.onsets, &mut self.evidence);

        for ev in self.evidence.drain(..) {
            let mut ev = ev;
            if let Some(o) = ev.onset {
                if let Some(&(_, at)) = self.reported.iter().find(|(f, _)| *f == o) {
                    ev.decided_at = ev.decided_at.max(at);
                }
            }
            out.push(ev);
        }
        let horizon = end.saturating_sub(10 * 44100);
        self.reported.retain(|(f, _)| *f >= horizon);
    }
}

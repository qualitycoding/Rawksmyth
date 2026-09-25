//! Scoring engine (plan step S-009, D-008, A-007, A-012).
//!
//! Rocksmith-style: the pitch gate is the ±50-cent identity band already
//! enforced by the verifier (`Presence::Present` implies it); the *grade*
//! comes from onset timing, computed in **song time** from the input-stream
//! frame index via the `ClockModel` — never from wall-clock time — so device
//! latency does not change judgments once L_rt is calibrated (SC-4).
//!
//! | \|onset − expected\| | Judgment |
//! |---|---|
//! | ≤ 15 ms | Perfect |
//! | ≤ 30 ms | Great |
//! | ≤ 50 ms | Good |
//! | > 50 ms, Absent, HarmonicUp, no attributed onset, or window closed with no evidence | Miss |

use crate::chart::ChartNote;
use crate::clock::{ClockModel, Frame};
use crate::dsp::{EvidenceKind, NoteEvidence, Presence};
use std::collections::HashSet;

pub const PERFECT_MS: f64 = 15.0;
pub const GREAT_MS: f64 = 30.0;
pub const GOOD_MS: f64 = 50.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judgment {
    Perfect,
    Great,
    Good,
    Miss,
}

impl Judgment {
    pub fn is_hit(self) -> bool {
        self != Judgment::Miss
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Judgment::Perfect => "Perfect",
            Judgment::Great => "Great",
            Judgment::Good => "Good",
            Judgment::Miss => "Miss",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JudgedNote {
    pub note_id: u32,
    pub judgment: Judgment,
    /// Signed, ms: positive = late relative to the charted time (song time).
    pub timing_error_ms: Option<f32>,
    pub cents_error: Option<f32>,
}

/// Grade an absolute timing error in milliseconds.
pub fn grade(abs_error_ms: f64) -> Judgment {
    if abs_error_ms <= PERFECT_MS {
        Judgment::Perfect
    } else if abs_error_ms <= GREAT_MS {
        Judgment::Great
    } else if abs_error_ms <= GOOD_MS {
        Judgment::Good
    } else {
        Judgment::Miss
    }
}

pub trait Scorer {
    fn on_evidence(&mut self, ev: &NoteEvidence, chart: &ChartNote, clock: &ClockModel) -> Option<JudgedNote>;
    /// Miss for every note whose window closed without a judgment.
    fn expire(&mut self, now: Frame, clock: &ClockModel) -> Vec<JudgedNote>;
}

#[derive(Debug, Clone)]
pub struct ScorerConfig {
    /// Half-width of the onset acceptance window (D-008).
    pub window_ms: f64,
    /// A note whose window closed is only expired after this much further
    /// time, so that evidence still being computed for a late onset (detector
    /// + verifier latency, §2B.1c) is not pre-empted.
    pub expire_slack_ms: f64,
}

impl Default for ScorerConfig {
    fn default() -> Self {
        ScorerConfig { window_ms: 50.0, expire_slack_ms: 400.0 }
    }
}

pub struct WindowScorer {
    cfg: ScorerConfig,
    pending: Vec<ChartNote>,
    judged: HashSet<u32>,
}

impl WindowScorer {
    pub fn new(chart: &[ChartNote]) -> Self {
        Self::with_config(chart, ScorerConfig::default())
    }

    pub fn with_config(chart: &[ChartNote], cfg: ScorerConfig) -> Self {
        WindowScorer { cfg, pending: chart.to_vec(), judged: HashSet::new() }
    }

    pub fn is_judged(&self, note_id: u32) -> bool {
        self.judged.contains(&note_id)
    }

    fn finish(&mut self, note_id: u32, j: JudgedNote) -> Option<JudgedNote> {
        self.judged.insert(note_id);
        self.pending.retain(|n| n.id != note_id);
        Some(j)
    }
}

impl Scorer for WindowScorer {
    fn on_evidence(&mut self, ev: &NoteEvidence, chart: &ChartNote, clock: &ClockModel) -> Option<JudgedNote> {
        // Only the fast Decision judges; Refinement (SC-1a accuracy) is logged
        // by the session, not re-judged. A note is judged exactly once.
        if ev.kind != EvidenceKind::Decision || self.judged.contains(&ev.note_id) {
            return None;
        }
        let timing_error_ms = ev.onset.map(|o| (clock.song_time(o) - chart.start_s) * 1e3);
        let judgment = match (ev.presence, timing_error_ms) {
            // D-011: Present without an attributed onset is a Miss (legato is
            // excluded in milestone 1, A-014).
            (Presence::Present, Some(e)) => grade(e.abs()),
            (Presence::Present, None) => Judgment::Miss,
            (Presence::Absent, _) | (Presence::HarmonicUp(_), _) => Judgment::Miss,
        };
        let cents_error = if judgment.is_hit() { ev.cents_error } else { None };
        self.finish(
            ev.note_id,
            JudgedNote {
                note_id: ev.note_id,
                judgment,
                timing_error_ms: timing_error_ms.map(|e| e as f32),
                cents_error,
            },
        )
    }

    fn expire(&mut self, now: Frame, clock: &ClockModel) -> Vec<JudgedNote> {
        let closes_after_s = (self.cfg.window_ms + self.cfg.expire_slack_ms) * 1e-3;
        let mut expired = Vec::new();
        for n in &self.pending {
            let deadline = clock.frame_for_song_time(n.start_s + closes_after_s);
            if now >= deadline {
                expired.push(JudgedNote { note_id: n.id, judgment: Judgment::Miss, timing_error_ms: None, cents_error: None });
            }
        }
        for j in &expired {
            self.judged.insert(j.note_id);
        }
        self.pending.retain(|n| !self.judged.contains(&n.id));
        expired
    }
}

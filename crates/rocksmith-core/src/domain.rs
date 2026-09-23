//! Core domain data structures, traits, and error types for rocksmith-core.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFrame {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub timestamp_samples: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchResult {
    pub frequency_hz: f32,
    pub confidence: f32,
    pub timestamp_ms: u64,
    pub is_voiced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnsetResult {
    pub timestamp_ms: u64,
    pub strength: f32,
}

pub trait PitchDetector: Send {
    fn process(&mut self, samples: &[f32]) -> Option<PitchResult>;
    fn reset(&mut self);
}

pub trait OnsetDetector: Send {
    fn process(&mut self, samples: &[f32]) -> Option<OnsetResult>;
    fn reset(&mut self);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedNoteEvent {
    pub onset_timestamp_ms: u64,
    pub resolved_frequency_hz: f32,
    pub confidence: f32,
    pub midi_note: u8,
    pub cents_deviation: f32,
}

pub trait TemporalNoteTracker: Send {
    fn record_onset(&mut self, onset: OnsetResult);
    fn record_pitch(&mut self, pitch: PitchResult);
    fn poll_resolved_notes(&mut self, current_time_ms: u64) -> Vec<ResolvedNoteEvent>;
    fn reset(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Judgment {
    Perfect, // <= 15ms, <= 10 cents
    Great,   // <= 30ms, <= 20 cents
    Good,    // <= 50ms, <= 35 cents
    Miss,    // > 50ms or wrong pitch
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteTarget {
    pub id: u64,
    pub target_timestamp_ms: u64,
    pub expected_midi_note: u8,
    pub string_index: u8,
    pub fret_number: u8,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

pub trait Scorer: Send {
    fn register_target(&mut self, target: NoteTarget);
    fn evaluate_event(&mut self, event: &ResolvedNoteEvent) -> Option<(u64, Judgment)>;
    fn check_expired_targets(&mut self, current_time_ms: u64) -> Vec<(u64, Judgment)>;
    fn get_stats(&self) -> SessionScoreStats;
    fn reset(&mut self);
}

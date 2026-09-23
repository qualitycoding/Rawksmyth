//! Temporal Note Tracker: Attack/Pitch Reconciler (Decision D-006).
//! Reconciles instantaneous pick strike transients with stabilized fundamental pitch.

use crate::domain::{OnsetResult, PitchResult, ResolvedNoteEvent, TemporalNoteTracker};

#[derive(Debug, Clone)]
struct CandidateNote {
    onset_timestamp_ms: u64,
    recorded_pitches: Vec<(u64, f32, f32)>, // (timestamp_ms, freq_hz, confidence)
    resolved: bool,
}

#[derive(Debug, Clone)]
pub struct TemporalConsensusTracker {
    transient_blank_ms: u64, // 0-15ms: discard raw chaotic pick scrape
    consensus_window_ms: u64, // 15-35ms: gather pitch estimates
    max_history_ms: u64,
    candidates: Vec<CandidateNote>,
}

impl Default for TemporalConsensusTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TemporalConsensusTracker {
    pub fn new() -> Self {
        Self {
            transient_blank_ms: 15,
            consensus_window_ms: 35,
            max_history_ms: 120,
            candidates: Vec::with_capacity(16),
        }
    }

    /// Convert frequency in Hz to MIDI note number and cents deviation
    pub fn hz_to_midi(hz: f32) -> (u8, f32) {
        if hz <= 0.0 {
            return (0, 0.0);
        }
        let midi_float = 69.0 + 12.0 * (hz / 440.0).log2();
        let rounded_midi = midi_float.round() as i32;
        let cents = (midi_float - rounded_midi as f32) * 100.0;
        (rounded_midi.clamp(0, 127) as u8, cents)
    }
}

impl TemporalNoteTracker for TemporalConsensusTracker {
    fn record_onset(&mut self, onset: OnsetResult) {
        // Prevent duplicate onset flooding within 30ms
        if let Some(last) = self.candidates.last() {
            if onset.timestamp_ms.saturating_sub(last.onset_timestamp_ms) < 30 {
                return;
            }
        }

        self.candidates.push(CandidateNote {
            onset_timestamp_ms: onset.timestamp_ms,
            recorded_pitches: Vec::with_capacity(8),
            resolved: false,
        });
    }

    fn record_pitch(&mut self, pitch: PitchResult) {
        for candidate in self.candidates.iter_mut() {
            if candidate.resolved {
                continue;
            }

            let delta = pitch.timestamp_ms.saturating_sub(candidate.onset_timestamp_ms);

            // Ignore initial chaotic pick transient (0-15ms)
            if delta >= self.transient_blank_ms && delta <= self.max_history_ms {
                candidate.recorded_pitches.push((pitch.timestamp_ms, pitch.frequency_hz, pitch.confidence));
            }
        }
    }

    fn poll_resolved_notes(&mut self, current_time_ms: u64) -> Vec<ResolvedNoteEvent> {
        let mut resolved_events = Vec::new();

        for candidate in self.candidates.iter_mut() {
            if candidate.resolved {
                continue;
            }

            let age = current_time_ms.saturating_sub(candidate.onset_timestamp_ms);
            // Once candidate has aged past the consensus window (35ms), compute median pitch
            if age >= self.consensus_window_ms && !candidate.recorded_pitches.is_empty() {
                // Sort by frequency to find median (filtering outliers and harmonic jitter)
                candidate.recorded_pitches.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                let mid = candidate.recorded_pitches.len() / 2;
                let median_pitch = candidate.recorded_pitches[mid].1;

                // Compute average confidence
                let avg_confidence = candidate.recorded_pitches.iter().map(|p| p.2).sum::<f32>()
                    / candidate.recorded_pitches.len() as f32;

                let (midi_note, cents) = Self::hz_to_midi(median_pitch);

                resolved_events.push(ResolvedNoteEvent {
                    onset_timestamp_ms: candidate.onset_timestamp_ms,
                    resolved_frequency_hz: median_pitch,
                    confidence: avg_confidence,
                    midi_note,
                    cents_deviation: cents,
                });

                candidate.resolved = true;
            }
        }

        // Clean up resolved candidates older than max_history_ms
        self.candidates.retain(|c| !c.resolved || current_time_ms.saturating_sub(c.onset_timestamp_ms) < 300);

        resolved_events
    }

    fn reset(&mut self) {
        self.candidates.clear();
    }
}

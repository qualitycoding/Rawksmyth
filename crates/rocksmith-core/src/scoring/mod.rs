//! Real-time scoring and judgment engine (T-008, T-009, T-022).

use std::collections::VecDeque;
use crate::domain::{Judgment, NoteTarget, ResolvedNoteEvent, Scorer, SessionScoreStats};

#[derive(Debug, Clone)]
pub struct RocksmithScorer {
    targets: VecDeque<NoteTarget>,
    stats: SessionScoreStats,
    perfect_window_ms: i64, // ±15 ms
    great_window_ms: i64,   // ±30 ms
    good_window_ms: i64,    // ±50 ms
    pitch_tolerance_cents: f32,
}

impl Default for RocksmithScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl RocksmithScorer {
    pub fn new() -> Self {
        Self {
            targets: VecDeque::new(),
            stats: SessionScoreStats::default(),
            perfect_window_ms: 15,
            great_window_ms: 30,
            good_window_ms: 50,
            pitch_tolerance_cents: 35.0,
        }
    }

    fn update_stats(&mut self, judgment: Judgment) {
        match judgment {
            Judgment::Perfect => {
                self.stats.perfect_count += 1;
                self.stats.current_streak += 1;
            }
            Judgment::Great => {
                self.stats.great_count += 1;
                self.stats.current_streak += 1;
            }
            Judgment::Good => {
                self.stats.good_count += 1;
                self.stats.current_streak += 1;
            }
            Judgment::Miss => {
                self.stats.miss_count += 1;
                self.stats.current_streak = 0;
            }
        }
        if self.stats.current_streak > self.stats.max_streak {
            self.stats.max_streak = self.stats.current_streak;
        }

        let hits = self.stats.perfect_count + self.stats.great_count + self.stats.good_count;
        let total = hits + self.stats.miss_count;
        self.stats.total_notes = total;
        self.stats.accuracy_percentage = if total > 0 {
            (hits as f32 / total as f32) * 100.0
        } else {
            100.0
        };
    }
}

impl Scorer for RocksmithScorer {
    fn register_target(&mut self, target: NoteTarget) {
        self.targets.push_back(target);
    }

    fn evaluate_event(&mut self, event: &ResolvedNoteEvent) -> Option<(u64, Judgment)> {
        // Find the closest matching target in time
        let mut best_idx = None;
        let mut min_abs_diff = i64::MAX;

        for (idx, target) in self.targets.iter().enumerate() {
            let diff = event.onset_timestamp_ms as i64 - target.target_timestamp_ms as i64;
            if diff.abs() < min_abs_diff {
                min_abs_diff = diff.abs();
                best_idx = Some(idx);
            }
        }

        if let Some(idx) = best_idx {
            let target = &self.targets[idx];
            let time_diff = (event.onset_timestamp_ms as i64 - target.target_timestamp_ms as i64).abs();
            let note_matches = event.midi_note == target.expected_midi_note;
            let cents_ok = event.cents_deviation.abs() <= self.pitch_tolerance_cents;

            if time_diff <= self.good_window_ms && note_matches && cents_ok {
                let judgment = if time_diff <= self.perfect_window_ms && event.cents_deviation.abs() <= 10.0 {
                    Judgment::Perfect
                } else if time_diff <= self.great_window_ms && event.cents_deviation.abs() <= 20.0 {
                    Judgment::Great
                } else {
                    Judgment::Good
                };

                let target_id = target.id;
                self.targets.remove(idx);
                self.update_stats(judgment);
                return Some((target_id, judgment));
            }
        }

        None
    }

    fn check_expired_targets(&mut self, current_time_ms: u64) -> Vec<(u64, Judgment)> {
        let mut missed = Vec::new();
        while let Some(front) = self.targets.front() {
            if current_time_ms as i64 - front.target_timestamp_ms as i64 > self.good_window_ms {
                let target = self.targets.pop_front().unwrap();
                self.update_stats(Judgment::Miss);
                missed.push((target.id, Judgment::Miss));
            } else {
                break;
            }
        }
        missed
    }

    fn get_stats(&self) -> SessionScoreStats {
        self.stats.clone()
    }

    fn reset(&mut self) {
        self.targets.clear();
        self.stats = SessionScoreStats::default();
    }
}

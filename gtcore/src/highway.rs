//! Note-highway layout (plan step S-011, the engine-agnostic half; T-007).
//!
//! Pure function of (chart, song time): which notes are visible and where.
//! The Godot scene (not built here) only draws the returned `NotePose`s, so
//! everything that can be wrong about *where a note is* is testable in CI.
//!
//! Coordinates: `lane` is the string index (0 = lowest string), `depth` is
//! 0 at the strike line and 1 at the far end of the highway; notes that have
//! passed the strike line have negative depth.

use crate::chart::ChartNote;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteState {
    /// Still to be played.
    Approaching,
    /// At or past the strike line and inside its sustain (or the short
    /// post-hit trail).
    Passing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NotePose {
    pub note_id: u32,
    pub lane: u8,
    pub fret: u8,
    /// (start − song_time) / lookahead. May be negative (already struck).
    pub depth: f32,
    /// Length of the sustain tail in depth units (0 for a plain note); the
    /// tail extends from `depth` towards the far end, clipped at 1.
    pub sustain_depth: f32,
    pub state: NoteState,
}

pub trait HighwayLayout {
    /// Clears `out`, then fills it with the notes visible at `song_time_s`,
    /// ordered by start time.
    fn visible_notes(&self, song_time_s: f64, out: &mut Vec<NotePose>);
}

#[derive(Debug, Clone)]
pub struct HighwayConfig {
    /// How far ahead notes are shown, seconds.
    pub lookahead_s: f64,
    /// How long a struck note stays visible after its start, seconds.
    pub trail_s: f64,
}

impl Default for HighwayConfig {
    fn default() -> Self {
        HighwayConfig { lookahead_s: 3.0, trail_s: 0.25 }
    }
}

pub struct ChartHighway {
    notes: Vec<ChartNote>,
    cfg: HighwayConfig,
    max_visible_after_start: f64,
}

impl ChartHighway {
    pub fn new(chart: &[ChartNote]) -> Self {
        Self::with_config(chart, HighwayConfig::default())
    }

    pub fn with_config(chart: &[ChartNote], cfg: HighwayConfig) -> Self {
        let mut notes = chart.to_vec();
        notes.sort_by(|a, b| a.start_s.partial_cmp(&b.start_s).unwrap_or(std::cmp::Ordering::Equal).then(a.id.cmp(&b.id)));
        let max_sustain = notes.iter().map(|n| n.sustain_s.max(0.0)).fold(0.0, f64::max);
        let max_visible_after_start = max_sustain.max(cfg.trail_s);
        ChartHighway { notes, cfg, max_visible_after_start }
    }
}

impl HighwayLayout for ChartHighway {
    fn visible_notes(&self, t: f64, out: &mut Vec<NotePose>) {
        out.clear();
        if !t.is_finite() {
            return;
        }
        // Earliest start that could still be visible: a note is shown until
        // start + max(sustain, trail), so nothing starting before
        // t − max_visible_after_start can be.
        let first = self.notes.partition_point(|n| n.start_s < t - self.max_visible_after_start);
        for n in &self.notes[first..] {
            let dt = n.start_s - t;
            if dt > self.cfg.lookahead_s {
                break; // sorted by start
            }
            let sustain = n.sustain_s.max(0.0);
            let visible_until = n.start_s + sustain.max(self.cfg.trail_s);
            if t > visible_until {
                continue;
            }
            out.push(NotePose {
                note_id: n.id,
                lane: n.string,
                fret: n.fret,
                depth: (dt / self.cfg.lookahead_s) as f32,
                sustain_depth: (sustain / self.cfg.lookahead_s) as f32,
                state: if dt > 0.0 { NoteState::Approaching } else { NoteState::Passing },
            });
        }
    }
}

//! Session logger (plan step S-012). Collects judgments and refinement
//! evidence into a JSON session log; written atomically (temp file + rename,
//! §3.4) so a crash or device loss never leaves a truncated log.
//!
//! Also carries the D-013 offset-assist statistic: the median signed timing
//! error over hit notes, offered to the user only with n ≥ 30.

use crate::scoring::{JudgedNote, Judgment};
use serde::Serialize;
use std::io::Write;
use std::path::Path;

/// Minimum number of hit notes before the offset assist is offered (D-013).
pub const OFFSET_ASSIST_MIN_NOTES: usize = 30;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClockSettings {
    pub sample_rate: f64,
    pub l_in_s: f64,
    pub l_out_s: f64,
    /// Signed user audio offset δ_a, seconds.
    pub delta_a_s: f64,
    /// Signed user video offset δ_v, seconds.
    pub delta_v_s: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind")]
pub enum SessionStatus {
    Completed,
    /// The audio device disappeared mid-session (T-011).
    DeviceLost { at_frame: u64 },
    Aborted,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NoteRecord {
    pub note_id: u32,
    /// `null` when the session ended before this note was judged.
    pub judgment: Option<String>,
    pub timing_error_ms: Option<f32>,
    /// Coarse pitch error from the fast decision.
    pub cents_error: Option<f32>,
    /// SC-1a-grade pitch error from the W_r = 2048 refinement, when available.
    pub refined_cents: Option<f32>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Summary {
    pub total: usize,
    pub judged: usize,
    pub perfect: usize,
    pub great: usize,
    pub good: usize,
    pub miss: usize,
    pub longest_streak: usize,
    /// 0–100; Perfect = 100, Great = 80, Good = 50, Miss = 0 points per note,
    /// as a percentage of the maximum over *all* notes in the chart.
    pub score_percent: f64,
    pub hit_rate: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OffsetAssist {
    /// Median signed timing error over hit notes, ms (positive = late).
    pub median_timing_error_ms: f64,
    pub n: usize,
    /// True when n ≥ `OFFSET_ASSIST_MIN_NOTES`; settings may then offer it.
    pub offer: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SessionLog {
    pub song_title: String,
    /// Supplied by the caller (gtcore has no clock source), ISO-8601.
    pub started_at: String,
    pub clock: ClockSettings,
    pub status: SessionStatus,
    pub notes: Vec<NoteRecord>,
    pub summary: Summary,
    pub offset_assist: Option<OffsetAssist>,
}

impl SessionLog {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("session log is always serializable")
    }

    /// Write atomically: `<path>.tmp` is written and synced, then renamed
    /// over `path`.
    pub fn write_atomic(&self, path: &Path) -> std::io::Result<()> {
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = std::path::PathBuf::from(tmp);
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(self.to_json().as_bytes())?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, path).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
    }
}

pub struct SessionRecorder {
    song_title: String,
    started_at: String,
    clock: ClockSettings,
    notes: Vec<NoteRecord>,
}

impl SessionRecorder {
    /// `chart_note_ids` in chart order.
    pub fn new(song_title: &str, started_at: &str, clock: ClockSettings, chart_note_ids: &[u32]) -> Self {
        SessionRecorder {
            song_title: song_title.to_string(),
            started_at: started_at.to_string(),
            clock,
            notes: chart_note_ids
                .iter()
                .map(|&id| NoteRecord {
                    note_id: id,
                    judgment: None,
                    timing_error_ms: None,
                    cents_error: None,
                    refined_cents: None,
                })
                .collect(),
        }
    }

    fn record_mut(&mut self, note_id: u32) -> Option<&mut NoteRecord> {
        self.notes.iter_mut().find(|n| n.note_id == note_id)
    }

    pub fn record_judgment(&mut self, j: &JudgedNote) {
        if let Some(r) = self.record_mut(j.note_id) {
            r.judgment = Some(j.judgment.as_str().to_string());
            r.timing_error_ms = j.timing_error_ms;
            r.cents_error = j.cents_error;
        }
    }

    pub fn record_refinement(&mut self, note_id: u32, cents: Option<f32>) {
        if let Some(r) = self.record_mut(note_id) {
            r.refined_cents = cents;
        }
    }

    pub fn finish(self, status: SessionStatus) -> SessionLog {
        let summary = summarize(&self.notes);
        let offset_assist = offset_assist(&self.notes);
        SessionLog {
            song_title: self.song_title,
            started_at: self.started_at,
            clock: self.clock,
            status,
            notes: self.notes,
            summary,
            offset_assist,
        }
    }
}

fn parse(j: &Option<String>) -> Option<Judgment> {
    match j.as_deref() {
        Some("Perfect") => Some(Judgment::Perfect),
        Some("Great") => Some(Judgment::Great),
        Some("Good") => Some(Judgment::Good),
        Some("Miss") => Some(Judgment::Miss),
        _ => None,
    }
}

fn summarize(notes: &[NoteRecord]) -> Summary {
    let (mut perfect, mut great, mut good, mut miss) = (0, 0, 0, 0);
    let (mut streak, mut longest) = (0usize, 0usize);
    for n in notes {
        match parse(&n.judgment) {
            Some(Judgment::Perfect) => perfect += 1,
            Some(Judgment::Great) => great += 1,
            Some(Judgment::Good) => good += 1,
            Some(Judgment::Miss) => miss += 1,
            None => {}
        }
        match parse(&n.judgment) {
            Some(j) if j.is_hit() => {
                streak += 1;
                longest = longest.max(streak);
            }
            _ => streak = 0,
        }
    }
    let total = notes.len();
    let points = 100 * perfect + 80 * great + 50 * good;
    let hits = perfect + great + good;
    Summary {
        total,
        judged: perfect + great + good + miss,
        perfect,
        great,
        good,
        miss,
        longest_streak: longest,
        score_percent: if total == 0 { 0.0 } else { points as f64 / (100 * total) as f64 * 100.0 },
        hit_rate: if total == 0 { 0.0 } else { hits as f64 / total as f64 },
    }
}

fn offset_assist(notes: &[NoteRecord]) -> Option<OffsetAssist> {
    let mut errs: Vec<f64> = notes
        .iter()
        .filter(|n| parse(&n.judgment).is_some_and(|j| j.is_hit()))
        .filter_map(|n| n.timing_error_ms.map(|e| e as f64))
        .collect();
    if errs.is_empty() {
        return None;
    }
    errs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = errs.len();
    let median = if n % 2 == 1 { errs[n / 2] } else { 0.5 * (errs[n / 2 - 1] + errs[n / 2]) };
    Some(OffsetAssist { median_timing_error_ms: median, n, offer: n >= OFFSET_ASSIST_MIN_NOTES })
}

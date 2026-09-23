//! Chart file parser and data structures with security validation (T-012, T-013, T-020).

use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::domain::NoteTarget;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub bpm: f32,
    pub tuning: String, // e.g., "E Standard"
    pub audio_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartNoteRaw {
    pub id: u64,
    pub timestamp_ms: u64,
    pub string: u8, // 1 to 6 (1=High E, 6=Low E)
    pub fret: u8,   // 0 to 24
    pub midi_note: u8,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongChart {
    pub metadata: SongMetadata,
    pub notes: Vec<ChartNoteRaw>,
}

pub struct ChartParser;

impl ChartParser {
    pub fn parse_json(json_str: &str) -> Result<SongChart, String> {
        // Enforce maximum chart JSON length to avoid memory exhaustion attacks
        if json_str.len() > 10 * 1024 * 1024 {
            return Err("Chart file exceeds maximum allowed size (10 MB)".to_string());
        }

        let chart: SongChart = serde_json::from_str(json_str)
            .map_err(|e| format!("Malformed chart JSON: {}", e))?;

        // Security check: Guard against Path Traversal in audio_file asset reference (T-013)
        Self::validate_audio_path(&chart.metadata.audio_file)?;

        Ok(chart)
    }

    /// Reject path traversal characters like `..`, leading slashes, or drive letters
    pub fn validate_audio_path(path_str: &str) -> Result<(), String> {
        let path = Path::new(path_str);
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    return Err(format!("Security violation: Path traversal '..' rejected in '{}'", path_str));
                }
                std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                    return Err(format!("Security violation: Absolute path rejected in '{}'", path_str));
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn to_note_targets(chart: &SongChart) -> Vec<NoteTarget> {
        chart.notes.iter().map(|n| NoteTarget {
            id: n.id,
            target_timestamp_ms: n.timestamp_ms,
            expected_midi_note: n.midi_note,
            string_index: n.string,
            fret_number: n.fret,
            duration_ms: n.duration_ms,
        }).collect()
    }
}

//! Session logging & performance export (S-010, T-010).

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use crate::domain::{Judgment, SessionScoreStats};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredNoteRecord {
    pub target_id: u64,
    pub expected_timestamp_ms: u64,
    pub actual_timestamp_ms: Option<u64>,
    pub expected_midi_note: u8,
    pub detected_midi_note: Option<u8>,
    pub cents_deviation: Option<f32>,
    pub judgment: Judgment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLog {
    pub song_title: String,
    pub session_start_iso: String,
    pub duration_seconds: f32,
    pub stats: SessionScoreStats,
    pub notes: Vec<ScoredNoteRecord>,
}

pub struct SessionLogger;

impl SessionLogger {
    pub fn write_atomic_json<P: AsRef<Path>>(path: P, log: &SessionLog) -> Result<(), String> {
        let json_data = serde_json::to_string_pretty(log)
            .map_err(|e| format!("Failed to serialize session log: {}", e))?;

        let final_path = path.as_ref();
        let temp_path = final_path.with_extension("tmp");

        let mut file = File::create(&temp_path)
            .map_err(|e| format!("Failed to create temporary session file: {}", e))?;
        file.write_all(json_data.as_bytes())
            .map_err(|e| format!("Failed to write session data: {}", e))?;
        file.sync_all()
            .map_err(|e| format!("Failed to sync session file: {}", e))?;

        std::fs::rename(&temp_path, final_path)
            .map_err(|e| format!("Failed to atomically rename session log file: {}", e))?;

        Ok(())
    }
}

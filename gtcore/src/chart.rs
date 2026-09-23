//! Chart format and parser (plan step S-007).
//!
//! Implements the `ChartNote` data model from plan/PLAN.md §2B.1a and the
//! parser hardening required by T-012 (malformed input), T-013 (path
//! traversal in asset references) and T-029 (oversized / deeply nested
//! input, without stack overflow).
//!
//! Scope note: this module is pure data + parsing, no audio or engine
//! dependency, so it is fully testable in a CI-tier sandbox (see plan
//! §2B.3, "Env" column). `target_hz` derivation (A-013: string identity is
//! not verified, only pitch) uses 12-TET from a per-string open tuning.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Hard limits from T-029. Chosen well above anything a real chart needs,
/// so legitimate files are never rejected, while a pathological or
/// adversarial file is rejected before it can exhaust memory or the stack.
pub const MAX_CHART_BYTES: usize = 5 * 1024 * 1024;
pub const MAX_NESTING_DEPTH: usize = 32;

#[derive(Debug)]
pub enum ChartError {
    TooLarge { bytes: usize, limit: usize },
    TooDeep { depth: usize, limit: usize },
    Malformed(String),
    InvalidString { note_id: u32, string: u8, num_strings: usize },
    UnsafeAssetPath { field: String, path: String },
    Legato { note_id: u32 },
}

impl fmt::Display for ChartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChartError::TooLarge { bytes, limit } => {
                write!(f, "chart file is {bytes} bytes, exceeds limit of {limit}")
            }
            ChartError::TooDeep { depth, limit } => {
                write!(f, "chart JSON nesting depth {depth} exceeds limit of {limit}")
            }
            ChartError::Malformed(msg) => write!(f, "malformed chart: {msg}"),
            ChartError::InvalidString { note_id, string, num_strings } => write!(
                f,
                "note {note_id} references string {string}, but tuning has {num_strings} strings"
            ),
            ChartError::UnsafeAssetPath { field, path } => {
                write!(f, "asset path in field '{field}' is unsafe: {path}")
            }
            ChartError::Legato { note_id } => write!(
                f,
                "note {note_id} is marked legato; legato notes are excluded from milestone-1 charts (A-014)"
            ),
        }
    }
}

impl std::error::Error for ChartError {}

/// Open-string frequencies in Hz, low to high. A-013: only pitch is
/// verified, not string identity, so this is used solely to derive
/// `target_hz` at parse time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tuning(pub Vec<f32>);

impl Tuning {
    pub fn standard_e() -> Self {
        Tuning(vec![82.407, 110.0, 146.832, 195.998, 246.942, 329.628])
    }

    fn fret_hz(&self, string: u8, fret: u8) -> Option<f32> {
        let open = *self.0.get(string as usize)?;
        Some(open * 2f32.powf(fret as f32 / 12.0))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartNoteRaw {
    pub id: u32,
    pub start_s: f64,
    pub sustain_s: f64,
    pub string: u8,
    pub fret: u8,
    #[serde(default)]
    pub legato: bool,
}

/// Fully resolved note, as consumed by the verifier / scorer (matches
/// `ChartNote` in plan §2B.1a).
#[derive(Debug, Clone, PartialEq)]
pub struct ChartNote {
    pub id: u32,
    pub start_s: f64,
    pub sustain_s: f64,
    pub string: u8,
    pub fret: u8,
    pub target_hz: f32,
    pub legato: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetRefs {
    #[serde(default)]
    pub backing_track: Option<String>,
    #[serde(default)]
    pub guitar_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartFileRaw {
    pub title: String,
    #[serde(default = "Tuning::standard_e")]
    pub tuning: Tuning,
    #[serde(default)]
    pub assets: AssetRefs,
    pub notes: Vec<ChartNoteRaw>,
}

#[derive(Debug, Clone)]
pub struct ChartFile {
    pub title: String,
    pub tuning: Tuning,
    pub assets: AssetRefs,
    pub notes: Vec<ChartNote>,
}

/// Iteratively scans raw JSON bytes for the maximum bracket/brace nesting
/// depth, without recursion — so this check itself cannot stack-overflow
/// regardless of how deeply nested (or how large) the input is. Runs
/// before `serde_json` ever sees the bytes.
fn max_nesting_depth(bytes: &[u8]) -> usize {
    let mut depth: usize = 0;
    let mut max_depth: usize = 0;
    let mut in_string = false;
    let mut escaped = false;

    for &b in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > max_depth {
                    max_depth = depth;
                }
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    max_depth
}

/// Rejects anything that is not a plain relative filename within the
/// asset directory: no absolute paths, no `..` traversal, no embedded
/// path separators that could climb out, no null bytes. (T-013)
fn validate_asset_path(field: &str, path: &str) -> Result<(), ChartError> {
    let unsafe_ = path.is_empty()
        || path.contains('\0')
        || path.starts_with('/')
        || path.starts_with('\\')
        || (path.len() >= 2 && path.as_bytes()[1] == b':') // e.g. C:\...
        || path.split(['/', '\\']).any(|seg| seg == "..")
        || path.starts_with("~");

    if unsafe_ {
        return Err(ChartError::UnsafeAssetPath {
            field: field.to_string(),
            path: path.to_string(),
        });
    }
    Ok(())
}

/// Parses and validates a chart file's raw bytes. Never panics: every
/// failure mode returns `Err(ChartError)` (T-012), including a file that
/// is too large (T-029), too deeply nested (T-029), has unsafe asset
/// paths (T-013), references a nonexistent string, or contains a legato
/// note (A-014, milestone-1 charts must not use legato).
pub fn parse_chart_bytes(bytes: &[u8]) -> Result<ChartFile, ChartError> {
    if bytes.len() > MAX_CHART_BYTES {
        return Err(ChartError::TooLarge { bytes: bytes.len(), limit: MAX_CHART_BYTES });
    }

    let depth = max_nesting_depth(bytes);
    if depth > MAX_NESTING_DEPTH {
        return Err(ChartError::TooDeep { depth, limit: MAX_NESTING_DEPTH });
    }

    let raw: ChartFileRaw = serde_json::from_slice(bytes)
        .map_err(|e| ChartError::Malformed(e.to_string()))?;

    if let Some(p) = &raw.assets.backing_track {
        validate_asset_path("backing_track", p)?;
    }
    if let Some(p) = &raw.assets.guitar_reference {
        validate_asset_path("guitar_reference", p)?;
    }

    let mut notes = Vec::with_capacity(raw.notes.len());
    for n in raw.notes {
        if n.legato {
            return Err(ChartError::Legato { note_id: n.id });
        }
        let target_hz = raw.tuning.fret_hz(n.string, n.fret).ok_or(ChartError::InvalidString {
            note_id: n.id,
            string: n.string,
            num_strings: raw.tuning.0.len(),
        })?;
        notes.push(ChartNote {
            id: n.id,
            start_s: n.start_s,
            sustain_s: n.sustain_s,
            string: n.string,
            fret: n.fret,
            target_hz,
            legato: n.legato,
        });
    }

    Ok(ChartFile { title: raw.title, tuning: raw.tuning, assets: raw.assets, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_e2_a4() -> String {
        // string 0 open = E2 (82.407 Hz); string 5 fret 5 = A4 (440 Hz, 12-TET)
        r#"{
            "title": "test",
            "notes": [
                {"id": 1, "start_s": 0.0, "sustain_s": 0.5, "string": 0, "fret": 0},
                {"id": 2, "start_s": 1.0, "sustain_s": 0.5, "string": 5, "fret": 5}
            ]
        }"#
        .to_string()
    }

    #[test]
    fn parses_valid_chart_and_derives_target_hz() {
        let chart = parse_chart_bytes(open_e2_a4().as_bytes()).expect("should parse");
        assert_eq!(chart.notes.len(), 2);
        assert!((chart.notes[0].target_hz - 82.407).abs() < 0.01);
        // A4 = 440 Hz from B3 (246.942) open + 5 frets, 12-TET
        assert!((chart.notes[1].target_hz - 440.0).abs() < 0.5);
    }

    // T-012: malformed input never panics, always returns Err.
    #[test]
    fn t012_malformed_json_returns_err_not_panic() {
        let cases = [
            b"".as_slice(),
            b"{".as_slice(),
            b"not json at all".as_slice(),
            b"{\"title\": 123}".as_slice(), // wrong type
            b"{\"title\": \"x\", \"notes\": \"not an array\"}".as_slice(),
            b"\xFF\xFE\x00\x01".as_slice(), // invalid UTF-8 / binary garbage
        ];
        for bytes in cases {
            let result = std::panic::catch_unwind(|| parse_chart_bytes(bytes));
            assert!(result.is_ok(), "parse_chart_bytes panicked on malformed input");
            assert!(result.unwrap().is_err(), "expected Err for malformed input {bytes:?}");
        }
    }

    #[test]
    fn t012_invalid_string_index_is_err_not_panic() {
        let bad = r#"{"title":"x","notes":[{"id":1,"start_s":0,"sustain_s":0.5,"string":9,"fret":0}]}"#;
        let err = parse_chart_bytes(bad.as_bytes()).unwrap_err();
        assert!(matches!(err, ChartError::InvalidString { .. }));
    }

    // T-013: path traversal in asset references is rejected.
    #[test]
    fn t013_rejects_path_traversal() {
        let cases = [
            r#"{"title":"x","assets":{"backing_track":"../../etc/passwd"},"notes":[]}"#,
            r#"{"title":"x","assets":{"backing_track":"/etc/passwd"},"notes":[]}"#,
            r#"{"title":"x","assets":{"backing_track":"..\\..\\secrets.json"},"notes":[]}"#,
            r#"{"title":"x","assets":{"guitar_reference":"~/.ssh/id_rsa"},"notes":[]}"#,
            r#"{"title":"x","assets":{"backing_track":"C:\\Windows\\system.ini"},"notes":[]}"#,
        ];
        for c in cases {
            let err = parse_chart_bytes(c.as_bytes()).unwrap_err();
            assert!(
                matches!(err, ChartError::UnsafeAssetPath { .. }),
                "expected UnsafeAssetPath for {c}, got {err:?}"
            );
        }
    }

    #[test]
    fn t013_accepts_plain_relative_asset_path() {
        let ok = r#"{"title":"x","assets":{"backing_track":"backing.wav"},"notes":[]}"#;
        let chart = parse_chart_bytes(ok.as_bytes()).expect("plain relative path should parse");
        assert_eq!(chart.assets.backing_track.as_deref(), Some("backing.wav"));
    }

    // T-029: oversized input rejected without attempting to parse.
    #[test]
    fn t029_rejects_oversized_input() {
        let huge = vec![b'a'; MAX_CHART_BYTES + 1];
        let err = parse_chart_bytes(&huge).unwrap_err();
        assert!(matches!(err, ChartError::TooLarge { .. }));
    }

    // T-029: deeply nested input rejected, and the depth scan itself does
    // not blow the stack even for pathological depth (it's iterative).
    #[test]
    fn t029_rejects_deep_nesting_without_stack_overflow() {
        let depth = 100_000; // far beyond MAX_NESTING_DEPTH and beyond what
                              // a recursive-descent parser could survive
        let mut s = String::with_capacity(depth * 2 + 16);
        s.push_str("{\"title\":\"x\",\"notes\":");
        for _ in 0..depth {
            s.push('[');
        }
        for _ in 0..depth {
            s.push(']');
        }
        s.push('}');

        let result = std::panic::catch_unwind(|| parse_chart_bytes(s.as_bytes()));
        assert!(result.is_ok(), "parser panicked/stack-overflowed on deep nesting");
        let err = result.unwrap().unwrap_err();
        assert!(matches!(err, ChartError::TooDeep { .. }));
    }

    #[test]
    fn t029_within_limit_nesting_is_accepted_by_the_depth_check() {
        // MAX_NESTING_DEPTH counts brace/bracket depth in the raw bytes;
        // a normal chart's structural depth is well under 32.
        let chart = open_e2_a4();
        assert!(max_nesting_depth(chart.as_bytes()) <= MAX_NESTING_DEPTH);
    }

    // A-014: legato notes are excluded from milestone-1 charts.
    #[test]
    fn a014_legato_note_is_rejected() {
        let c = r#"{"title":"x","notes":[{"id":1,"start_s":0,"sustain_s":0.5,"string":0,"fret":0,"legato":true}]}"#;
        let err = parse_chart_bytes(c.as_bytes()).unwrap_err();
        assert!(matches!(err, ChartError::Legato { note_id: 1 }));
    }
}

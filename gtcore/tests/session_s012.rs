//! S-012 tests: session log content, D-013 offset-assist statistic, and
//! atomic writing (plan §3.4).

use gtcore::scoring::{JudgedNote, Judgment};
use gtcore::session::*;

fn clock() -> ClockSettings {
    ClockSettings { sample_rate: 44100.0, l_in_s: 0.01, l_out_s: 0.02, delta_a_s: 0.0, delta_v_s: 0.0 }
}

fn judged(id: u32, j: Judgment, err: Option<f32>) -> JudgedNote {
    JudgedNote { note_id: id, judgment: j, timing_error_ms: err, cents_error: Some(1.0) }
}

#[test]
fn summary_counts_streak_and_score() {
    let ids: Vec<u32> = (1..=6).collect();
    let mut rec = SessionRecorder::new("Song", "2026-09-25T00:00:00Z", clock(), &ids);
    for (id, j) in [
        (1, Judgment::Perfect),
        (2, Judgment::Great),
        (3, Judgment::Good),
        (4, Judgment::Miss),
        (5, Judgment::Perfect),
        (6, Judgment::Perfect),
    ] {
        rec.record_judgment(&judged(id, j, Some(3.0)));
    }
    let log = rec.finish(SessionStatus::Completed);
    let s = &log.summary;
    assert_eq!((s.total, s.judged, s.perfect, s.great, s.good, s.miss), (6, 6, 3, 1, 1, 1));
    assert_eq!(s.longest_streak, 3);
    // (300 + 80 + 50) / 600
    assert!((s.score_percent - 430.0 / 600.0 * 100.0).abs() < 1e-9);
    assert!((s.hit_rate - 5.0 / 6.0).abs() < 1e-9);
}

/// D-013: median *signed* error over hit notes; offered only with n ≥ 30.
#[test]
fn offset_assist_is_offered_only_with_enough_notes() {
    let build = |n: u32| {
        let ids: Vec<u32> = (1..=n).collect();
        let mut rec = SessionRecorder::new("Song", "t", clock(), &ids);
        for id in 1..=n {
            // Systematically 12 ms late, with one wild miss that must not count.
            rec.record_judgment(&judged(id, Judgment::Great, Some(12.0 + (id % 3) as f32 - 1.0)));
        }
        rec.record_judgment(&judged(1, Judgment::Miss, Some(400.0)));
        rec.finish(SessionStatus::Completed)
    };
    let few = build(29);
    let oa = few.offset_assist.as_ref().unwrap();
    assert!(!oa.offer && oa.n == 28, "{oa:?}"); // note 1 became a Miss
    let many = build(40);
    let oa = many.offset_assist.as_ref().unwrap();
    assert!(oa.offer && oa.n == 39);
    assert!((oa.median_timing_error_ms - 12.0).abs() <= 1.0, "{oa:?}");
}

#[test]
fn unjudged_notes_and_refinement_are_recorded() {
    let mut rec = SessionRecorder::new("Song", "t", clock(), &[1, 2, 3]);
    rec.record_judgment(&judged(1, Judgment::Perfect, Some(1.0)));
    rec.record_refinement(1, Some(-2.5));
    let log = rec.finish(SessionStatus::DeviceLost { at_frame: 123 });
    assert_eq!(log.notes[0].judgment.as_deref(), Some("Perfect"));
    assert_eq!(log.notes[0].refined_cents, Some(-2.5));
    assert_eq!(log.notes[1].judgment, None);
    assert_eq!(log.summary.judged, 1);
    assert_eq!(log.summary.total, 3);
    let json: serde_json::Value = serde_json::from_str(&log.to_json()).unwrap();
    assert_eq!(json["status"]["kind"], "DeviceLost");
    assert_eq!(json["status"]["at_frame"], 123);
    assert!(json["notes"][1]["judgment"].is_null());
}

#[test]
fn write_atomic_round_trips_and_leaves_no_temp_file() {
    let dir = std::env::temp_dir().join(format!("gtcore-session-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("session.json");

    let mut rec = SessionRecorder::new("Song", "t", clock(), &[1]);
    rec.record_judgment(&judged(1, Judgment::Good, Some(40.0)));
    let first = rec.finish(SessionStatus::Completed);
    first.write_atomic(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(parsed["summary"]["good"], 1);

    // Overwriting replaces the previous log completely.
    let second = SessionRecorder::new("Song", "t", clock(), &[1, 2]).finish(SessionStatus::Aborted);
    second.write_atomic(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(parsed["summary"]["total"], 2);
    assert_eq!(parsed["status"]["kind"], "Aborted");

    let leftovers: Vec<_> =
        std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(leftovers, vec!["session.json".to_string()], "temp file left behind");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn write_to_an_unwritable_location_is_an_error_not_a_panic() {
    let log = SessionRecorder::new("Song", "t", clock(), &[1]).finish(SessionStatus::Completed);
    let bad = std::env::temp_dir().join("gtcore-no-such-dir-xyz").join("nested").join("s.json");
    assert!(log.write_atomic(&bad).is_err());
}

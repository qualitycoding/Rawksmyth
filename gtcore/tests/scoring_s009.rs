//! S-009 tests (plan §2B.3): T-008, T-009 — scorer boundaries and Miss
//! conditions, computed from onset timestamps in song time (SC-4).

mod common;
use common::*;
use gtcore::clock::ClockModel;
use gtcore::dsp::{EvidenceKind, NoteEvidence, Presence};
use gtcore::scoring::{Judgment, Scorer, WindowScorer};

const START_S: f64 = 2.0;

fn clock(l_in: f64, l_out: f64, delta_a: f64) -> ClockModel {
    ClockModel { fs: 44100.0, s0: 44100, l_in, l_out, delta_a }
}

/// Decision evidence whose onset sits `err_ms` after the charted time.
fn present_at(clock: &ClockModel, err_ms: f64, cents: f32) -> NoteEvidence {
    let onset = clock.frame_for_song_time(START_S + err_ms * 1e-3);
    NoteEvidence {
        note_id: 1,
        kind: EvidenceKind::Decision,
        presence: Presence::Present,
        onset: Some(onset),
        decided_at: onset + 2000,
        cents_error: Some(cents),
        aperiodicity: 0.02,
    }
}

fn score(ev: &NoteEvidence, clock: &ClockModel) -> gtcore::scoring::JudgedNote {
    let note = note_at(1, START_S, 57.0);
    let mut s = WindowScorer::new(&[note.clone()]);
    s.on_evidence(ev, &note, clock).expect("a Decision judges the note")
}

#[test]
fn t008_timing_boundaries_perfect_great_good() {
    let c = clock(0.010, 0.015, 0.0);
    // (error ms, expected). Margins of 0.5 ms around the 15/30/50 boundaries
    // (one frame is 0.023 ms).
    let cases = [
        (0.0, Judgment::Perfect),
        (14.5, Judgment::Perfect),
        (15.5, Judgment::Great),
        (29.5, Judgment::Great),
        (30.5, Judgment::Good),
        (49.5, Judgment::Good),
        (50.5, Judgment::Miss),
        (120.0, Judgment::Miss),
    ];
    for &(err, want) in &cases {
        for sign in [1.0, -1.0] {
            let j = score(&present_at(&c, sign * err, 3.0), &c);
            assert_eq!(j.judgment, want, "error {:+} ms", sign * err);
            let reported = j.timing_error_ms.unwrap() as f64;
            assert!((reported - sign * err).abs() < 0.05, "reported {reported} for {}", sign * err);
        }
    }
}

/// The signed timing error is positive when late, and pitch error is carried
/// through for hits.
#[test]
fn t008_reports_signed_timing_and_cents() {
    let c = clock(0.0, 0.0, 0.0);
    let late = score(&present_at(&c, 20.0, -7.5), &c);
    assert_eq!(late.judgment, Judgment::Great);
    assert!(late.timing_error_ms.unwrap() > 19.9);
    assert_eq!(late.cents_error, Some(-7.5));
    let early = score(&present_at(&c, -20.0, 0.0), &c);
    assert!(early.timing_error_ms.unwrap() < -19.9);
}

/// SC-4: judgments are computed from frames in song time, so the *same
/// physical timing* is judged the same whatever latency the device reports,
/// provided the clock model carries that latency.
#[test]
fn t008_judgment_is_independent_of_device_latency() {
    for &(l_in, l_out, da) in &[(0.0, 0.0, 0.0), (0.010, 0.015, 0.0), (0.030, 0.050, -0.005), (0.002, 0.120, 0.020)] {
        let c = clock(l_in, l_out, da);
        assert_eq!(score(&present_at(&c, 12.0, 0.0), &c).judgment, Judgment::Perfect);
        assert_eq!(score(&present_at(&c, 25.0, 0.0), &c).judgment, Judgment::Great);
        assert_eq!(score(&present_at(&c, -45.0, 0.0), &c).judgment, Judgment::Good);
    }
}

#[test]
fn t009_miss_conditions() {
    let c = clock(0.010, 0.015, 0.0);
    let note = note_at(1, START_S, 57.0);
    let onset = Some(c.frame_for_song_time(START_S));
    let mk = |presence, onset| NoteEvidence {
        note_id: 1,
        kind: EvidenceKind::Decision,
        presence,
        onset,
        decided_at: 0,
        cents_error: Some(2.0),
        aperiodicity: 0.5,
    };

    // Absent.
    let mut s = WindowScorer::new(&[note.clone()]);
    let j = s.on_evidence(&mk(Presence::Absent, onset), &note, &c).unwrap();
    assert_eq!(j.judgment, Judgment::Miss);
    assert_eq!(j.cents_error, None, "a Miss carries no pitch credit");

    // HarmonicUp.
    let mut s = WindowScorer::new(&[note.clone()]);
    assert_eq!(s.on_evidence(&mk(Presence::HarmonicUp(2), onset), &note, &c).unwrap().judgment, Judgment::Miss);

    // Present without an attributed onset on a non-legato note (D-011).
    let mut s = WindowScorer::new(&[note.clone()]);
    assert_eq!(s.on_evidence(&mk(Presence::Present, None), &note, &c).unwrap().judgment, Judgment::Miss);

    // Onset error > 50 ms.
    let mut s = WindowScorer::new(&[note.clone()]);
    assert_eq!(s.on_evidence(&present_at(&c, 60.0, 0.0), &note, &c).unwrap().judgment, Judgment::Miss);
}

#[test]
fn t009_window_closing_with_no_evidence_is_a_miss() {
    let c = clock(0.010, 0.015, 0.0);
    let notes = [note_at(1, 2.0, 57.0), note_at(2, 3.0, 59.0)];
    let mut s = WindowScorer::new(&notes);
    // Just after note 1's window closes but inside the slack that protects
    // evidence still being computed: nothing expires yet.
    assert!(s.expire(c.frame_for_song_time(2.0 + 0.060), &c).is_empty());
    // Well after: note 1 expires, note 2 does not.
    let expired = s.expire(c.frame_for_song_time(2.0 + 0.6), &c);
    assert_eq!(expired.len(), 1);
    assert_eq!((expired[0].note_id, expired[0].judgment), (1, Judgment::Miss));
    assert_eq!(expired[0].timing_error_ms, None);
    // Expiry is once-only.
    assert!(s.expire(c.frame_for_song_time(2.0 + 0.9), &c).is_empty());
    let expired = s.expire(c.frame_for_song_time(4.0), &c);
    assert_eq!(expired.iter().map(|j| j.note_id).collect::<Vec<_>>(), vec![2]);
}

#[test]
fn a_note_is_judged_exactly_once_and_refinement_does_not_rejudge() {
    let c = clock(0.010, 0.015, 0.0);
    let note = note_at(1, START_S, 57.0);
    let mut s = WindowScorer::new(&[note.clone()]);
    let dec = present_at(&c, 5.0, 1.0);
    assert!(s.on_evidence(&dec, &note, &c).is_some());
    assert!(s.on_evidence(&dec, &note, &c).is_none(), "duplicate Decision");
    let refinement = NoteEvidence { kind: EvidenceKind::Refinement, ..dec.clone() };
    assert!(s.on_evidence(&refinement, &note, &c).is_none());
    // A Refinement alone never judges.
    let mut s2 = WindowScorer::new(&[note.clone()]);
    assert!(s2.on_evidence(&refinement, &note, &c).is_none());
    assert!(!s2.is_judged(1));
    // Expiry does not double-judge a scored note.
    assert!(s.expire(c.frame_for_song_time(10.0), &c).is_empty());
}

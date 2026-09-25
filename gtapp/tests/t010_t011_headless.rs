//! T-010 (headless core on the file backend: load the example song, play
//! through, write a session log matching the expected judgments), T-011 (CI
//! part: injected device loss → no panic, session saved, user notified), and
//! S-008 (the committed example song loads through the S-007 parser).

use gtapp::*;
use gtaudio::backend::FileBackend;
use gtcore::scoring::Judgment;
use gtfixtures::*;
use std::path::{Path, PathBuf};

fn song_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/songs/example")
}

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("gtapp-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn backend(input: Vec<f32>, s: &Settings, block: usize) -> FileBackend {
    FileBackend::new(input, SAMPLE_RATE)
        .block_size(block)
        .channels(2)
        .latencies(s.l_in_s, s.l_out_s)
        .tail_frames((TAIL_S * SAMPLE_RATE) as u64)
}

fn perfect_input(song: &Song, s: &Settings) -> Vec<f32> {
    simulated_input(song.guitar_reference.as_ref().unwrap(), &s.clock())
}

fn run(song: &Song, s: Settings, input: Vec<f32>, block: usize, log: Option<PathBuf>) -> SessionOutcome {
    let mut b = backend(input, &s, block);
    run_session(&mut b, SessionOptions { song, settings: s, started_at: "test".into(), log_path: log })
}

fn judgments(o: &SessionOutcome) -> Vec<(u32, Judgment)> {
    let mut v: Vec<_> = o.judged.iter().map(|j| (j.note_id, j.judgment)).collect();
    v.sort_by_key(|(id, _)| *id);
    v
}

// ------------------------------------------------------------------ S-008

#[test]
fn s008_example_song_loads_and_is_well_formed() {
    let song = Song::load(&song_dir()).expect("committed example song must load");
    let notes = song.notes();
    assert!(notes.len() >= 30, "{} notes", notes.len());
    assert!(!song.backing.is_empty());
    let guitar = song.guitar_reference.as_ref().unwrap();
    assert_eq!(guitar.len(), song.backing.len(), "guitar reference is aligned to the backing track");
    // Range E2–E5 (±1 semitone slack), strictly increasing times, unique ids.
    for n in notes {
        assert!((80.0..=700.0).contains(&n.target_hz), "note {} at {} Hz", n.id, n.target_hz);
        assert!(!n.legato);
    }
    for w in notes.windows(2) {
        assert!(w[1].start_s > w[0].start_s);
        assert!(w[1].id != w[0].id);
        assert!(w[1].start_s - w[0].start_s >= 0.33, "notes {} and {} too close", w[0].id, w[1].id);
    }
    let last_end = notes.last().unwrap().start_s + notes.last().unwrap().sustain_s;
    assert!(song.backing.len() as f64 / SAMPLE_RATE > last_end);
    // Spans the range the plan cares about (E2 … E5).
    let lo = notes.iter().map(|n| n.target_hz).fold(f32::MAX, f32::min);
    let hi = notes.iter().map(|n| n.target_hz).fold(0.0, f32::max);
    assert!(lo < 90.0 && hi > 600.0, "range {lo}–{hi} Hz");
}

#[test]
fn song_loading_failures_are_errors_not_panics() {
    let d = temp_dir("badsong");
    // Missing chart.
    assert!(matches!(Song::load(&d), Err(SongError::Io(_))));
    // Traversal in an asset path is rejected by the chart parser (T-013).
    std::fs::write(
        d.join("chart.json"),
        r#"{"title":"x","assets":{"backing_track":"../evil.wav"},"notes":[]}"#,
    )
    .unwrap();
    assert!(matches!(Song::load(&d), Err(SongError::Chart(_))));
    // Missing asset file.
    std::fs::write(d.join("chart.json"), r#"{"title":"x","assets":{"backing_track":"nope.wav"},"notes":[]}"#).unwrap();
    assert!(matches!(Song::load(&d), Err(SongError::Wav(_))));
    // Wrong sample rate.
    std::fs::write(d.join("nope.wav"), gtaudio::wav::encode_pcm16(22050, 1, &[0.0; 100])).unwrap();
    assert!(matches!(Song::load(&d), Err(SongError::SampleRate { .. })));
    // Garbage audio.
    std::fs::write(d.join("nope.wav"), b"not a wav at all").unwrap();
    assert!(matches!(Song::load(&d), Err(SongError::Wav(_))));
    std::fs::remove_dir_all(&d).unwrap();
}

// ------------------------------------------------------------------ T-010

#[test]
fn t010_perfect_playthrough_scores_every_note_perfect_and_writes_the_log() {
    let song = Song::load(&song_dir()).unwrap();
    let s = Settings::default();
    let dir = temp_dir("perfect");
    let log_path = dir.join("session.json");
    let out = run(&song, s, perfect_input(&song, &s), 256, Some(log_path.clone()));
    assert_eq!(out.notice, None);
    assert_eq!(out.log.status, gtcore::session::SessionStatus::Completed);
    let n = song.notes().len();
    assert_eq!(out.judged.len(), n, "every note judged exactly once");
    assert!(out.judged.iter().all(|j| j.judgment == Judgment::Perfect), "{:?}", judgments(&out));
    assert_eq!(out.log.summary.perfect, n);
    assert_eq!(out.log.summary.score_percent, 100.0);
    assert_eq!(out.log.summary.longest_streak, n);
    // SC-1a on the refinement: every note's sustain pitch within ±10 cents.
    for r in &out.log.notes {
        let c = r.refined_cents.unwrap_or_else(|| panic!("note {} has no refinement", r.note_id));
        assert!(c.abs() <= 10.0, "note {}: {c} cents", r.note_id);
    }
    // The log on disk is what the run returned.
    let on_disk: serde_json::Value = serde_json::from_slice(&std::fs::read(&log_path).unwrap()).unwrap();
    assert_eq!(on_disk, serde_json::from_str::<serde_json::Value>(&out.log.to_json()).unwrap());
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "no temp file left behind");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn t010_runs_are_deterministic() {
    let song = Song::load(&song_dir()).unwrap();
    let s = Settings::default();
    let a = run(&song, s, perfect_input(&song, &s), 256, None);
    let b = run(&song, s, perfect_input(&song, &s), 256, None);
    assert_eq!(a.log.to_json(), b.log.to_json());
}

/// Re-render the guitar part with deliberate mistakes; the session log must
/// contain exactly the judgments those mistakes imply.
#[test]
fn t010_imperfect_playthrough_matches_expected_judgments() {
    let song = Song::load(&song_dir()).unwrap();
    let s = Settings::default();
    let clock = s.clock();
    // note id → (what the player does)
    enum Play {
        Skip,
        Transpose(f64),
        Offset(f64),
    }
    use Play::*;
    let plan: Vec<(u32, Play, Judgment)> = vec![
        (5, Skip, Judgment::Miss),
        (12, Skip, Judgment::Miss),
        (8, Transpose(1.0), Judgment::Miss),      // semitone sharp
        (15, Transpose(12.0), Judgment::Miss),    // octave up
        (18, Transpose(-1.0), Judgment::Miss),    // semitone flat
        (20, Offset(0.040), Judgment::Good),      // 40 ms late
        (21, Offset(-0.025), Judgment::Great),    // 25 ms early
        (22, Offset(0.010), Judgment::Perfect),   // 10 ms late
        (26, Offset(-0.046), Judgment::Good),     // 46 ms early: inside the window
    ];
    let mut plucks = Vec::new();
    for n in song.notes() {
        let mut t = n.start_s;
        let mut f = n.target_hz as f64;
        match plan.iter().find(|(id, _, _)| *id == n.id) {
            Some((_, Skip, _)) => continue,
            Some((_, Transpose(st), _)) => f *= 2f64.powf(st / 12.0),
            Some((_, Offset(dt), _)) => t += dt,
            None => {}
        }
        plucks.push(PluckAt { frame: input_frame_for(&clock, t), f0_hz: f, amp: 0.55 });
    }
    let total = input_frame_for(&clock, 0.0) + song.backing.len();
    let input = render_part(&plucks, total, 1800.0, 555);

    let out = run(&song, s, input, 256, None);
    let got = judgments(&out);
    assert_eq!(got.len(), song.notes().len());
    for (id, j) in &got {
        let want = plan.iter().find(|(pid, _, _)| pid == id).map(|(_, _, w)| *w).unwrap_or(Judgment::Perfect);
        assert_eq!(*j, want, "note {id}: got {j:?}, want {want:?}\nall: {got:?}");
    }
    // The signed timing errors carry the intended offsets.
    let err = |id: u32| out.judged.iter().find(|j| j.note_id == id).unwrap().timing_error_ms.unwrap();
    assert!((err(20) - 40.0).abs() < 1.5 && (err(21) + 25.0).abs() < 1.5 && (err(26) + 46.0).abs() < 1.5);
    // Summary consistency.
    assert_eq!(out.log.summary.miss, 5);
    assert_eq!(out.log.summary.good, 2);
    assert_eq!(out.log.summary.great, 1);
}

/// Judgments do not depend on how the device chops the stream into blocks.
#[test]
fn t010_judgments_are_independent_of_block_size() {
    let song = Song::load(&song_dir()).unwrap();
    let s = Settings::default();
    let reference = run(&song, s, perfect_input(&song, &s), 256, None);
    for block in [64usize, 128, 480, 1024] {
        let o = run(&song, s, perfect_input(&song, &s), block, None);
        assert_eq!(judgments(&o), judgments(&reference), "block {block}");
        for (a, b) in o.judged.iter().zip(&reference.judged) {
            if a.note_id == b.note_id {
                assert_eq!(a.timing_error_ms, b.timing_error_ms, "note {} block {block}", a.note_id);
            }
        }
    }
}

/// SC-4: with the clock model carrying the device's latencies, the same
/// physical performance is judged the same whatever latency the device has.
#[test]
fn sc4_judgments_are_independent_of_device_latency() {
    let song = Song::load(&song_dir()).unwrap();
    let reference = Settings::default();
    let base = run(&song, reference, perfect_input(&song, &reference), 256, None);
    for (l_in, l_out) in [(0.002, 0.005), (0.030, 0.060), (0.005, 0.150)] {
        let s = Settings { l_in_s: l_in, l_out_s: l_out, ..Settings::default() };
        let o = run(&song, s, perfect_input(&song, &s), 256, None);
        assert_eq!(judgments(&o), judgments(&base), "L_in {l_in} L_out {l_out}");
        assert!(o.judged.iter().all(|j| j.timing_error_ms.unwrap().abs() < 1.0));
    }
}

/// D-013: a driver that misreports latency shows up as a systematic timing
/// bias; the offset assist recovers it in one step.
#[test]
fn d013_offset_assist_corrects_a_misreported_latency() {
    let song = Song::load(&song_dir()).unwrap();
    let truth = Settings::default();
    let input = perfect_input(&song, &truth);
    // The driver under-reports output latency by 40 ms: the player is judged 40 ms late.
    let wrong = Settings { l_out_s: truth.l_out_s - 0.040, ..truth };
    let first = run(&song, wrong, input.clone(), 256, None);
    let oa = first.log.offset_assist.as_ref().unwrap();
    assert!(oa.offer, "33 hit notes ≥ 30");
    assert_eq!(first.log.summary.good, song.notes().len(), "a 40 ms bias turns every Perfect into a Good");
    assert!((oa.median_timing_error_ms - 40.0).abs() < 2.0, "median {:.2}", oa.median_timing_error_ms);
    // Apply the assist: δ_a += median (song_time = … − (…+δ_a): a *late*-reading player needs larger L_rt).
    let fixed = Settings { delta_a_s: wrong.delta_a_s + oa.median_timing_error_ms / 1000.0, ..wrong };
    let second = run(&song, fixed, input, 256, None);
    assert!(second.judged.iter().all(|j| j.judgment == Judgment::Perfect), "{:?}", judgments(&second));
}

/// KNOWN LIMITATION (R-104, recorded in HANDOFF.md): D-013's offset assist
/// learns from *hit* notes only, and a note is only attributed an onset inside
/// its ±50 ms window. A misreported latency larger than the window therefore
/// yields all Misses and no calibration data. If a wide-window calibration
/// mode is added, this test should be replaced by one asserting recovery.
#[test]
fn known_limitation_latency_error_beyond_the_window_gives_no_offset_assist_data() {
    let song = Song::load(&song_dir()).unwrap();
    let truth = Settings::default();
    let wrong = Settings { l_out_s: truth.l_out_s - 0.120, ..truth };
    let o = run(&song, wrong, perfect_input(&song, &truth), 256, None);
    assert_eq!(o.log.summary.miss, song.notes().len());
    assert!(o.log.offset_assist.is_none());
}

// ------------------------------------------------------------------ T-011

#[test]
fn t011_device_loss_saves_the_partial_session_and_notifies_the_user() {
    let song = Song::load(&song_dir()).unwrap();
    let s = Settings::default();
    let dir = temp_dir("loss");
    let log_path = dir.join("session.json");
    let mut b = backend(perfect_input(&song, &s), &s, 256).fail_at_frame((12.0 * SAMPLE_RATE) as u64);
    let out = run_session(
        &mut b,
        SessionOptions { song: &song, settings: s, started_at: "test".into(), log_path: Some(log_path.clone()) },
    );
    let at = match out.log.status {
        gtcore::session::SessionStatus::DeviceLost { at_frame } => at_frame,
        ref other => panic!("expected DeviceLost, got {other:?}"),
    };
    assert!((12.0 * SAMPLE_RATE) as u64 <= at && at < (12.0 * SAMPLE_RATE) as u64 + 256);
    let notice = out.notice.as_ref().expect("the user must be told");
    assert!(notice.to_lowercase().contains("device") && notice.contains("saved"), "{notice}");
    // Notes before the loss were judged (and were Perfect); later ones are unjudged, not Miss.
    let cutoff = s.clock().song_time(at);
    for r in &out.log.notes {
        let n = song.notes().iter().find(|n| n.id == r.note_id).unwrap();
        if n.start_s + 0.6 < cutoff {
            assert_eq!(r.judgment.as_deref(), Some("Perfect"), "note {}", n.id);
        }
        if n.start_s > cutoff + 0.05 {
            assert_eq!(r.judgment, None, "note {} judged after the device was lost", n.id);
        }
    }
    assert!(out.log.summary.judged > 0 && out.log.summary.judged < out.log.summary.total);
    assert_eq!(out.log.summary.miss, 0);
    // The partial session is on disk and valid.
    let on_disk: serde_json::Value = serde_json::from_slice(&std::fs::read(&log_path).unwrap()).unwrap();
    assert_eq!(on_disk["status"]["kind"], "DeviceLost");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn t011_unwritable_log_location_is_reported_not_fatal() {
    let song = Song::load(&song_dir()).unwrap();
    let s = Settings::default();
    let bad = std::env::temp_dir().join("gtapp-no-such-dir-abc").join("x").join("session.json");
    let out = run(&song, s, perfect_input(&song, &s), 256, Some(bad));
    let notice = out.notice.expect("a failed save must be reported");
    assert!(notice.contains("could not be saved"), "{notice}");
    assert_eq!(out.log.summary.perfect, song.notes().len(), "the session itself still completed");
}

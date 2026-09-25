//! S-006 tests (plan §2B.3): T-006 (scale through the pipeline), T-020
//! (decision latency in frames), T-028 (repeated same-pitch notes). CI tier;
//! the "file backend" is a plain loop over blocks of a generated signal.

mod common;
use common::*;
use gtcore::chart::ChartNote;
use gtcore::clock::ClockModel;
use gtcore::dsp::{EvidenceKind, Presence};
use gtcore::scoring::Judgment;
use gtfixtures::*;

fn clock() -> ClockModel {
    // Nonzero latencies so the clock mapping is genuinely exercised: the
    // player "aligns with what they hear", i.e. plucks L_rt after song time.
    ClockModel { fs: 44100.0, s0: 44100, l_in: 0.012, l_out: 0.018, delta_a: 0.0 }
}

/// Frame at which a player hearing the backing track plucks a note charted at
/// `start_s` (inverse of §2B.1d).
fn pluck_frame(c: &ClockModel, start_s: f64) -> usize {
    c.frame_for_song_time(start_s) as usize
}

/// T-006: C-major scale C3..C5 (15 notes) → every note Present with the
/// correct note_id; timing error p95 ≤ 10 ms.
#[test]
fn t006_c_major_scale_every_note_present_and_on_time() {
    let c = clock();
    let steps = [0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21, 23, 24]; // C3..C5
    let chart: Vec<ChartNote> =
        steps.iter().enumerate().map(|(i, s)| note_at(i as u32 + 1, 1.0 + 0.45 * i as f64, 48.0 + *s as f64)).collect();
    let plucks: Vec<PluckAt> = chart
        .iter()
        .map(|n| PluckAt { frame: pluck_frame(&c, n.start_s), f0_hz: n.target_hz as f64, amp: 0.6 })
        .collect();
    let total = pluck_frame(&c, 1.0 + 0.45 * 15.0 + 2.0);
    let sig = render_part(&plucks, total, 900.0, 606);

    let (ev, judged) = run_scored(&chart, &c, &sig, 256);
    let mut errs = Vec::new();
    for n in &chart {
        let d = decision(&ev, n.id);
        assert_eq!(d.presence, Presence::Present, "note {}: {d:?}", n.id);
        let onset = d.onset.expect("Present carries its onset");
        errs.push(((c.song_time(onset) - n.start_s) * 1e3).abs());
        let j = judged.iter().find(|j| j.note_id == n.id).unwrap_or_else(|| panic!("note {} unjudged", n.id));
        assert_eq!(j.judgment, Judgment::Perfect, "note {}: {j:?}", n.id);
    }
    assert_eq!(judged.len(), chart.len(), "each note judged exactly once");
    errs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95 = errs[((errs.len() as f64 * 0.95).ceil() as usize) - 1];
    assert!(p95 <= 10.0, "timing error p95 {p95:.2} ms");
}

/// T-020 / SC-1b: worst-case time from the true onset to the pipeline's
/// decision, in frames, on the fixture set: ≤ 55 ms for E2–G#3, ≤ 35 ms for
/// A3–E5. Blocks are one hop (128 frames), so block granularity does not
/// hide latency.
#[test]
fn t020_decision_latency_in_frames_meets_sc1b() {
    let c = ClockModel { fs: 44100.0, s0: 0, l_in: 0.0, l_out: 0.0, delta_a: 0.0 };
    let (limit_low, limit_high) = (ms_to_frames(55.0), ms_to_frames(35.0));
    let mut worst = (0u64, 0u64);
    for midi in 40..=76 {
        for seed in 0..2u64 {
            let t = 44100u64;
            let n = note_at(1, t as f64 / 44100.0, midi as f64);
            let sig = render_part(
                &[PluckAt { frame: t as usize, f0_hz: n.target_hz as f64, amp: 0.3 + 0.3 * seed as f32 }],
                44100 * 3,
                1500.0,
                700 + midi as u64 * 10 + seed,
            );
            let (ev, _) = run_scored(&[n], &c, &sig, 128);
            let d = decision(&ev, 1);
            assert_eq!(d.presence, Presence::Present, "midi {midi}: {d:?}");
            let latency = d.decided_at - t;
            let (limit, slot) = if midi <= 56 { (limit_low, &mut worst.0) } else { (limit_high, &mut worst.1) };
            *slot = (*slot).max(latency);
            assert!(
                latency <= limit,
                "midi {midi}: decision latency {latency} frames = {:.1} ms exceeds {:.0} ms",
                latency as f64 / 44.1,
                limit as f64 / 44.1
            );
        }
    }
    eprintln!(
        "worst decision latency: E2–G#3 {} frames ({:.1} ms), A3–E5 {} frames ({:.1} ms)",
        worst.0,
        worst.0 as f64 / 44.1,
        worst.1,
        worst.1 as f64 / 44.1
    );
}

/// The reported decided_at includes the onset detector's own latency: it is
/// never earlier than the block in which the onset was reported.
#[test]
fn decided_at_is_never_before_the_onset_was_reported() {
    let c = ClockModel { fs: 44100.0, s0: 0, l_in: 0.0, l_out: 0.0, delta_a: 0.0 };
    let t = 44100usize;
    let n = note_at(1, 1.0, 76.0); // E5: shortest verifier latency, so L_onset could dominate
    let sig = render_part(&[PluckAt { frame: t, f0_hz: n.target_hz as f64, amp: 0.5 }], 44100 * 3, 1500.0, 9);
    let (ev, _) = run_scored(&[n], &c, &sig, 128);
    let d = decision(&ev, 1);
    // 512-frame window + 3-hop lookahead is ≈ 20 ms; the onset cannot be
    // known before roughly that long after it happened.
    assert!(d.decided_at - t as u64 >= ms_to_frames(10.0), "decided {} frames after onset", d.decided_at - t as u64);
}

/// T-028: two same-pitch chart notes 80 ms apart.
#[test]
fn t028_same_pitch_notes_80ms_apart() {
    let c = ClockModel { fs: 44100.0, s0: 0, l_in: 0.0, l_out: 0.0, delta_a: 0.0 };
    let chart = [note_at(1, 1.0, 45.0), note_at(2, 1.080, 45.0)];
    let f = midi_to_hz(45.0);

    // Two plucks → each judged once, both hits, distinct onsets.
    let two = render_part(
        &[
            PluckAt { frame: 44100, f0_hz: f, amp: 0.5 },
            PluckAt { frame: 44100 + ms_to_frames(80.0) as usize, f0_hz: f, amp: 0.5 },
        ],
        44100 * 4,
        1500.0,
        28,
    );
    let (ev, judged) = run_scored(&chart, &c, &two, 256);
    assert_eq!(judged.len(), 2, "{judged:?}");
    assert!(judged.iter().all(|j| j.judgment.is_hit()), "{judged:?}");
    let (o1, o2) = (decision(&ev, 1).onset.unwrap(), decision(&ev, 2).onset.unwrap());
    assert_ne!(o1, o2);

    // One pluck → one hit and one Miss, and the ringing note does not
    // satisfy the second.
    let one = render_part(&[PluckAt { frame: 44100, f0_hz: f, amp: 0.5 }], 44100 * 4, 1500.0, 29);
    let (_, judged) = run_scored(&chart, &c, &one, 256);
    assert_eq!(judged.len(), 2, "{judged:?}");
    let hits = judged.iter().filter(|j| j.judgment.is_hit()).count();
    assert_eq!(hits, 1, "{judged:?}");
    let miss = judged.iter().find(|j| j.judgment == Judgment::Miss).unwrap();
    assert_eq!(miss.note_id, 2);
}

/// Refinement evidence flows through the pipeline as well.
#[test]
fn pipeline_emits_refinement_after_present() {
    let c = ClockModel { fs: 44100.0, s0: 0, l_in: 0.0, l_out: 0.0, delta_a: 0.0 };
    let n = note_at(1, 1.0, 52.0);
    let sig = render_part(&[PluckAt { frame: 44100, f0_hz: n.target_hz as f64, amp: 0.5 }], 44100 * 3, 1500.0, 3);
    let (ev, _) = run_scored(&[n], &c, &sig, 256);
    let r = ev.iter().find(|e| e.kind == EvidenceKind::Refinement).expect("refinement");
    assert!(r.cents_error.unwrap().abs() <= 10.0);
}

//! S-005 tests (plan §2B.3): T-001–T-003, T-017, T-018, T-025–T-027, T-032–T-034
//! plus attribution and streaming-independence checks. CI tier, generated
//! fixtures (tests/fixtures/gen).
//!
//! Caveat recorded in HANDOFF.md: the fixtures are idealized Karplus–Strong
//! plucks. Passing here shows the verifier meets the specification on those
//! fixtures; behaviour on a real guitar (R-101, R-108) is only established at
//! gate G-001.

mod common;
use common::*;
use gtcore::dsp::{EvidenceKind, Presence};
use gtfixtures::*;

const E2: f64 = 82.4069;
const A2: f64 = 110.0;
const E3: f64 = 164.8138;
const B3: f64 = 246.9417;
const A4: f64 = 440.0;
const E5: f64 = 659.2551;

fn cents_offset(f: f64, c: f64) -> f64 {
    f * 2f64.powf(c / 1200.0)
}

fn assert_present_with_sustain_accuracy(ev: &[gtcore::dsp::NoteEvidence], id: u32, tol_cents: f32, what: &str) {
    let d = decision(ev, id);
    assert_eq!(d.presence, Presence::Present, "{what}: {ev:?}");
    let r = refinement(ev, id).unwrap_or_else(|| panic!("{what}: no Refinement evidence: {ev:?}"));
    let c = r.cents_error.unwrap_or_else(|| panic!("{what}: Refinement has no cents: {r:?}"));
    assert!(c.abs() <= tol_cents, "{what}: sustain cents error {c} > {tol_cents}");
}

// ---------------------------------------------------------------- SC-1a

#[test]
fn t001_a4_sine_is_present_within_5_cents() {
    let mut sig = vec![0.0f32; 44100 * 3];
    mix_at(&mut sig, &sine(A4, 0.5, 44100), T0 as usize, 1.0);
    add_noise_floor(&mut sig, 3e-4, 1);
    let ev = run(&sig, &[expect(1, A4, T0)], 256);
    assert_eq!(decision(&ev, 1).presence, Presence::Present, "{ev:?}");
    assert!(decision(&ev, 1).cents_error.unwrap().abs() <= 5.0);
    assert_present_with_sustain_accuracy(&ev, 1, 5.0, "A4 sine");
}

#[test]
fn t002_e2_ks_pluck_is_present_within_10_cents() {
    let sig = single_pluck(Pluck::new(E2, 2), T0 as usize);
    let ev = run(&sig, &[expect(1, E2, T0)], 256);
    assert_present_with_sustain_accuracy(&ev, 1, 10.0, "E2 pluck");
}

#[test]
fn t003_e5_ks_pluck_is_present_within_10_cents() {
    let sig = single_pluck(Pluck::new(E5, 3), T0 as usize);
    let ev = run(&sig, &[expect(1, E5, T0)], 256);
    assert_present_with_sustain_accuracy(&ev, 1, 10.0, "E5 pluck");
}

/// Every semitone E2–E5 is Present with sustain accuracy ≤ 10 cents.
#[test]
fn sc1a_every_semitone_e2_to_e5() {
    for midi in 40..=76 {
        let f = midi_to_hz(midi as f64);
        let sig = single_pluck(Pluck::new(f, 100 + midi as u64), T0 as usize);
        let ev = run(&sig, &[expect(1, f, T0)], 256);
        assert_present_with_sustain_accuracy(&ev, 1, 10.0, &format!("midi {midi} ({f:.1} Hz)"));
    }
}

#[test]
fn t025_harmonic_up_when_an_octave_or_twelfth_above_is_played() {
    let e3 = single_pluck(Pluck::new(E3, 4), T0 as usize);
    let ev = run(&e3, &[expect(1, E2, T0)], 256);
    assert_eq!(decision(&ev, 1).presence, Presence::HarmonicUp(2), "expected E2, played E3: {ev:?}");

    let b3 = single_pluck(Pluck::new(B3, 5), T0 as usize);
    let ev = run(&b3, &[expect(1, E2, T0)], 256);
    assert_eq!(decision(&ev, 1).presence, Presence::HarmonicUp(3), "expected E2, played B3: {ev:?}");
}

#[test]
fn t026_lower_note_played_is_absent() {
    let e2 = single_pluck(Pluck::new(E2, 6), T0 as usize);
    let ev = run(&e2, &[expect(1, E3, T0)], 256);
    assert_eq!(decision(&ev, 1).presence, Presence::Absent, "expected E3, played E2: {ev:?}");
    assert!(refinement(&ev, 1).is_none());
}

#[test]
fn t027_semitone_off_is_absent_and_forty_cents_off_is_present() {
    let a_sharp = single_pluck(Pluck::new(midi_to_hz(70.0), 7), T0 as usize);
    let ev = run(&a_sharp, &[expect(1, A4, T0)], 256);
    assert_eq!(decision(&ev, 1).presence, Presence::Absent, "expected A4, played A#4: {ev:?}");

    for offset in [40.0, -40.0] {
        let sig = single_pluck(Pluck::new(cents_offset(A4, offset), 8), T0 as usize);
        let ev = run(&sig, &[expect(1, A4, T0)], 256);
        let d = decision(&ev, 1);
        assert_eq!(d.presence, Presence::Present, "A4 {offset:+} cents: {ev:?}");
        let c = d.cents_error.unwrap() as f64;
        assert!((c - offset).abs() <= 5.0, "decision cents {c}, expected {offset} ±5");
        let r = refinement(&ev, 1).and_then(|r| r.cents_error).unwrap() as f64;
        assert!((r - offset).abs() <= 5.0, "refinement cents {r}, expected {offset} ±5");
    }
    // 60 cents is outside the ±50-cent identity band (A-012).
    let sig = single_pluck(Pluck::new(cents_offset(A4, 60.0), 9), T0 as usize);
    let ev = run(&sig, &[expect(1, A4, T0)], 256);
    assert_eq!(decision(&ev, 1).presence, Presence::Absent, "A4 +60 cents: {ev:?}");
}

/// R-101: a weak fundamental must not be read as an octave up.
#[test]
fn t032_attenuated_fundamental_is_present_not_harmonic_up() {
    for &(f, name) in &[(E2, "E2"), (A2, "A2")] {
        for db in [20.0, 30.0] {
            let sig = single_pluck(Pluck::new(f, 10).h1_atten(db), T0 as usize);
            let ev = run(&sig, &[expect(1, f, T0)], 256);
            assert_eq!(decision(&ev, 1).presence, Presence::Present, "{name}, H1 −{db} dB: {ev:?}");
        }
    }
}

/// R-102: a +25-cent attack glide decaying over 50 ms.
#[test]
fn t033_attack_glide_is_decided_correctly_and_sustain_is_accurate() {
    for &f in &[E2, A2, 220.0, A4, E5] {
        let sig = single_pluck(Pluck::new(f, 11).glide(25.0, 0.05), T0 as usize);
        let ev = run(&sig, &[expect(1, f, T0)], 256);
        assert_present_with_sustain_accuracy(&ev, 1, 10.0, &format!("glide at {f:.1} Hz"));
    }
}

/// R-108: A2 expected while a previous E2 still rings 6 dB below it.
#[test]
fn t034_a2_over_a_ringing_e2_is_present() {
    let gap_ms = 300.0;
    let t_a2 = T0 + ms_to_frames(gap_ms);
    let (decay_db_s, amp_a2) = (8.0f64, 0.5f64);
    // Choose the E2 pluck level so that, at the moment A2 is plucked, the
    // ringing E2 is exactly 6 dB below A2.
    let amp_e2 = amp_a2 * 10f64.powf(-6.0 / 20.0) * 10f64.powf(decay_db_s * gap_ms / 1000.0 / 20.0);
    let mut sig = vec![0.0f32; 44100 * 3];
    mix_at(&mut sig, &Pluck::new(E2, 21).amp(amp_e2 as f32).render(44100 * 2), T0 as usize, 1.0);
    mix_at(&mut sig, &Pluck::new(A2, 22).amp(amp_a2 as f32).render(44100), t_a2 as usize, 1.0);
    add_noise_floor(&mut sig, 3e-4, 5);
    let ev = run(&sig, &[expect(1, A2, t_a2)], 256);
    assert_eq!(decision(&ev, 1).presence, Presence::Present, "{ev:?}");
}

// ------------------------------------------------------------- boundaries

#[test]
fn t017_silence_produces_no_present_evidence() {
    let sig = vec![0.0f32; 44100 * 3];
    let expected = [expect(1, E2, T0), expect(2, A4, T0 + 22050), expect(3, E5, T0 + 44100)];
    let ev = run(&sig, &expected, 256);
    assert!(ev.iter().all(|e| e.presence != Presence::Present), "{ev:?}");
    for id in 1..=3 {
        assert_eq!(decision(&ev, id).presence, Presence::Absent);
    }
}

#[test]
fn t018_white_noise_is_absent_for_all_expected_notes() {
    // −20 dBFS rms white noise over the whole signal, and a second signal
    // where the noise starts abruptly inside the expected window (so the
    // onset detector fires and the verifier must reject on periodicity).
    let mut always = white_noise(44100 * 4, 0.1, 31);
    let mut burst = vec![0.0f32; 44100 * 4];
    let noise = white_noise(44100 * 2, 0.1, 32);
    mix_at(&mut burst, &noise, T0 as usize, 1.0);
    for sig in [&mut always, &mut burst] {
        let expected: Vec<_> = [E2, A2, E3, A4, E5]
            .iter()
            .enumerate()
            .map(|(i, &f)| expect(i as u32 + 1, f, T0 + i as u64 * 8000))
            .collect();
        let ev = run(sig, &expected, 256);
        for id in 1..=5 {
            assert_eq!(decision(&ev, id).presence, Presence::Absent, "note {id}: {ev:?}");
        }
    }
}

// ----------------------------------------------------------- attribution

/// A note needs an onset attributed to it (D-011): a ringing previous note of
/// the same pitch must not satisfy a repeated note that was not re-plucked.
#[test]
fn repeated_same_pitch_note_without_a_second_pluck_is_absent() {
    let t1 = T0;
    let t2 = T0 + ms_to_frames(80.0);
    let sig = single_pluck(Pluck::new(A2, 41), t1 as usize);
    let ev = run(&sig, &[expect(1, A2, t1), expect(2, A2, t2)], 256);
    assert_eq!(decision(&ev, 1).presence, Presence::Present, "{ev:?}");
    assert_eq!(decision(&ev, 2).presence, Presence::Absent, "{ev:?}");
    assert_eq!(decision(&ev, 2).onset, None);
}

#[test]
fn two_plucks_80ms_apart_are_each_attributed_their_own_onset() {
    let t1 = T0;
    let t2 = T0 + ms_to_frames(80.0);
    let mut sig = vec![0.0f32; 44100 * 3];
    mix_at(&mut sig, &Pluck::new(A2, 42).render(44100), t1 as usize, 1.0);
    mix_at(&mut sig, &Pluck::new(A2, 43).render(44100), t2 as usize, 1.0);
    add_noise_floor(&mut sig, 3e-4, 6);
    let ev = run(&sig, &[expect(1, A2, t1), expect(2, A2, t2)], 256);
    let (d1, d2) = (decision(&ev, 1), decision(&ev, 2));
    let (o1, o2) = (d1.onset.expect("note 1 has an onset"), d2.onset.expect("note 2 has an onset"));
    assert_ne!(o1, o2, "one onset must not serve two notes");
    assert!((o1 as i64 - t1 as i64).abs() <= 441 && (o2 as i64 - t2 as i64).abs() <= 441, "{o1} {o2}");
}

// ------------------------------------------------------------- streaming

#[test]
fn evidence_is_independent_of_block_size() {
    let sig = single_pluck(Pluck::new(E3, 51).glide(25.0, 0.05), T0 as usize);
    let expected = [expect(1, E3, T0)];
    let reference = run(&sig, &expected, 128);
    assert!(reference.iter().any(|e| e.kind == EvidenceKind::Refinement));
    for block in [64usize, 100, 256, 1000, 4096] {
        assert_eq!(run(&sig, &expected, block), reference, "block size {block}");
    }
}

/// An octave error above E5 is still caught: the sub-lag search range covers
/// everything a 24-fret guitar can sound (deviation from §2B.1b, see HANDOFF).
#[test]
fn octave_up_above_e5_is_harmonic_up() {
    for &(expected, played, k) in &[(A4, 880.0, 2u8), (392.0, 784.0, 2), (329.6276, 988.0, 3)] {
        let sig = single_pluck(Pluck::new(played, 61), T0 as usize);
        let ev = run(&sig, &[expect(1, expected, T0)], 256);
        assert_eq!(decision(&ev, 1).presence, Presence::HarmonicUp(k), "expected {expected}, played {played}: {ev:?}");
    }
}

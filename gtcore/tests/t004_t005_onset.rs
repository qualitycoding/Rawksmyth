//! T-004, T-005 (plan §2B.3): onset detector, CI tier, on generated fixtures
//! (tests/fixtures/gen). SC-2.
//!
//! Caveat recorded in HANDOFF.md: fixtures are Karplus–Strong plucks with an
//! instantaneous noise-burst attack, which is a sharper transient than a real
//! guitar's. These tests establish the detector's behaviour on the fixtures;
//! claim C-005 on real instruments is only checked at G-001.

use gtcore::dsp::onset::{OnsetConfig, SpectralFluxOnset};
use gtcore::dsp::{Frame, Onset, OnsetDetector};
use gtfixtures::*;

fn detect(sig: &[f32], block: usize) -> Vec<Onset> {
    let mut det = SpectralFluxOnset::new(OnsetConfig::default());
    let mut out = Vec::new();
    let mut first: Frame = 0;
    for blk in sig.chunks(block) {
        det.process(blk, first, &mut out);
        first += blk.len() as Frame;
    }
    out
}

/// T-004a: a single KS pluck at frame 44100 is reported within ±441 frames
/// (±10 ms), once.
#[test]
fn t004_single_pluck_onset_within_441_frames() {
    for &(f0, seed) in &[(82.41, 1u64), (110.0, 2), (196.0, 3), (440.0, 4), (659.26, 5)] {
        let mut sig = vec![0.0f32; 44100 * 3];
        mix_at(&mut sig, &Pluck::new(f0, seed).render(44100), 44100, 1.0);
        add_noise_floor(&mut sig, 3e-4, seed);
        let onsets = detect(&sig, 256);
        assert_eq!(onsets.len(), 1, "f0 {f0}: expected exactly one onset, got {onsets:?}");
        let err = onsets[0].frame as i64 - 44100;
        assert!(err.abs() <= 441, "f0 {f0}: onset at {} (error {err} frames)", onsets[0].frame);
    }
}

/// T-004b: over the 50-pluck set, every pluck is detected exactly once, with
/// median |error| ≤ 5 ms and p95 ≤ 10 ms.
#[test]
fn t004_fifty_pluck_set_timing_statistics() {
    let (sig, events) = fifty_pluck_set(100);
    let onsets = detect(&sig, 256);
    let mut errs_ms = Vec::new();
    for ev in &events {
        let nearest = onsets
            .iter()
            .min_by_key(|o| (o.frame as i64 - ev.onset_frame as i64).abs())
            .expect("detector reported no onsets at all");
        let e = (nearest.frame as i64 - ev.onset_frame as i64) as f64 / FS * 1e3;
        assert!(
            e.abs() < 45.0,
            "pluck at frame {} (f0 {:.1}, amp {:.2}) not detected: nearest onset {:+.1} ms away",
            ev.onset_frame,
            ev.f0_hz,
            ev.amp,
            e
        );
        errs_ms.push(e.abs());
    }
    assert_eq!(onsets.len(), events.len(), "spurious or duplicate onsets: {} vs {}", onsets.len(), events.len());
    errs_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = errs_ms[errs_ms.len() / 2];
    let p95 = errs_ms[((errs_ms.len() as f64 * 0.95).ceil() as usize) - 1];
    assert!(median <= 5.0, "median |error| {median:.2} ms > 5 ms");
    assert!(p95 <= 10.0, "p95 |error| {p95:.2} ms > 10 ms");
}

/// The detector is causal and streaming: results must not depend on how the
/// input is chopped into blocks.
#[test]
fn onsets_are_independent_of_block_size() {
    let (sig, _) = fifty_pluck_set(101);
    let reference = detect(&sig, 128);
    assert!(!reference.is_empty());
    for block in [1usize, 37, 64, 256, 512, 4096, sig.len()] {
        assert_eq!(detect(&sig, block), reference, "block size {block}");
    }
}

/// T-005a: no onset on digital silence or on a −70 dBFS noise floor.
#[test]
fn t005_no_onset_on_silence_or_noise_floor() {
    assert!(detect(&vec![0.0f32; 44100 * 5], 256).is_empty());
    let mut noise = vec![0.0f32; 44100 * 5];
    add_noise_floor(&mut noise, 3e-4, 77);
    assert!(detect(&noise, 256).is_empty());
}

/// T-005b: a decaying sustain yields the pluck's onset and nothing afterwards
/// (including while it rings out, and for a steady sine).
#[test]
fn t005_no_onset_on_decaying_sustain() {
    for &(f0, seed) in &[(82.41, 11u64), (146.83, 12), (329.63, 13), (659.26, 14)] {
        let mut sig = vec![0.0f32; 44100 * 6];
        let mut p = Pluck::new(f0, seed).render(44100 * 5);
        fade_out(&mut p, 44100 / 2);
        mix_at(&mut sig, &p, 22050, 1.0);
        add_noise_floor(&mut sig, 3e-4, seed);
        let onsets = detect(&sig, 256);
        assert_eq!(onsets.len(), 1, "f0 {f0}: onsets {onsets:?}");
        assert!((onsets[0].frame as i64 - 22050).abs() <= 441);
    }

    // Steady sine that starts at frame 0 with a fade-in: one onset at most
    // during the fade, none once steady.
    let mut sine_sig = sine(220.0, 0.5, 44100 * 4);
    for i in 0..4410 {
        sine_sig[i] *= i as f32 / 4410.0;
    }
    let onsets = detect(&sine_sig, 256);
    assert!(onsets.iter().all(|o| o.frame < 44100), "onset after the sine reached steady state: {onsets:?}");
}

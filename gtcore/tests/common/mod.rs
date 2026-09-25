//! Shared harness for CI-tier DSP tests: runs the onset detector and the note
//! verifier over a signal the way the pipeline (S-006) will, block by block.
#![allow(dead_code)]

use gtcore::chart::ChartNote;
use gtcore::dsp::onset::{OnsetConfig, SpectralFluxOnset};
use gtcore::dsp::verifier::{ConstrainedNsdfVerifier, VerifierConfig};
use gtcore::dsp::*;
use gtfixtures::*;

pub fn chart_note(id: u32, target_hz: f64) -> ChartNote {
    ChartNote { id, start_s: 0.0, sustain_s: 0.5, string: 0, fret: 0, target_hz: target_hz as f32, legato: false }
}

/// A note expected to be plucked at frame `t`, with the ±50 ms acceptance
/// window of D-008.
pub fn expect(id: u32, target_hz: f64, t: u64) -> ExpectedNote {
    let w = ms_to_frames(50.0);
    ExpectedNote { note: chart_note(id, target_hz), open: t.saturating_sub(w), close: t + w }
}

pub fn run(sig: &[f32], expected: &[ExpectedNote], block: usize) -> Vec<NoteEvidence> {
    let mut det = SpectralFluxOnset::new(OnsetConfig::default());
    let mut ver = ConstrainedNsdfVerifier::new(VerifierConfig::default());
    let mut evidence = Vec::new();
    let mut first: Frame = 0;
    for blk in sig.chunks(block) {
        let mut onsets = Vec::new();
        det.process(blk, first, &mut onsets);
        ver.process(blk, first, expected, &onsets, &mut evidence);
        first += blk.len() as Frame;
    }
    evidence
}

pub fn decision(ev: &[NoteEvidence], id: u32) -> &NoteEvidence {
    let mut it = ev.iter().filter(|e| e.note_id == id && e.kind == EvidenceKind::Decision);
    let d = it.next().unwrap_or_else(|| panic!("no Decision evidence for note {id}: {ev:?}"));
    assert!(it.next().is_none(), "more than one Decision for note {id}: {ev:?}");
    d
}

pub fn refinement(ev: &[NoteEvidence], id: u32) -> Option<&NoteEvidence> {
    ev.iter().find(|e| e.note_id == id && e.kind == EvidenceKind::Refinement)
}

/// One pluck of `f_played` at frame `t` on a −70 dBFS noise floor, 3 s long
/// with the pluck starting at 1 s (t = 44100) unless overridden.
pub fn single_pluck(p: Pluck, t: usize) -> Vec<f32> {
    let mut sig = vec![0.0f32; t + 44100 * 2];
    mix_at(&mut sig, &p.render(44100), t, 1.0);
    add_noise_floor(&mut sig, 3e-4, 0x5EED);
    sig
}

pub const T0: u64 = 44100;

use gtcore::clock::ClockModel;
use gtcore::pipeline::Pipeline;
use gtcore::scoring::{JudgedNote, Scorer, WindowScorer};

/// Run the full pipeline + scorer over `sig` in blocks of `block` frames.
/// Returns (all evidence, all judgments in the order issued).
pub fn run_scored(
    chart: &[ChartNote],
    clock: &ClockModel,
    sig: &[f32],
    block: usize,
) -> (Vec<NoteEvidence>, Vec<JudgedNote>) {
    let mut pipe = Pipeline::new(chart, clock);
    let mut scorer = WindowScorer::new(chart);
    let (mut all_ev, mut judged) = (Vec::new(), Vec::new());
    let mut first: Frame = 0;
    for blk in sig.chunks(block) {
        let mut ev = Vec::new();
        pipe.process(blk, first, &mut ev);
        first += blk.len() as Frame;
        for e in &ev {
            let note = chart.iter().find(|n| n.id == e.note_id).unwrap();
            if let Some(j) = scorer.on_evidence(e, note, clock) {
                judged.push(j);
            }
        }
        judged.extend(scorer.expire(first, clock));
        all_ev.extend(ev);
    }
    (all_ev, judged)
}

/// A chart note at song time `start_s` for `midi`.
pub fn note_at(id: u32, start_s: f64, midi: f64) -> ChartNote {
    ChartNote { id, start_s, sustain_s: 0.3, string: 0, fret: 0, target_hz: midi_to_hz(midi) as f32, legato: false }
}

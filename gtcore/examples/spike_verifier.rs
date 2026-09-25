//! Spike 2 (plan §1.3 R6): constrained-NSDF verifier on generated fixtures.
//! Prints accuracy, decisions and decision latency in frames. Not a test; the
//! frozen tests live in gtcore/tests/. Run: cargo run --release -p gtcore
//! --example spike_verifier

use gtcore::chart::ChartNote;
use gtcore::dsp::onset::{OnsetConfig, SpectralFluxOnset};
use gtcore::dsp::verifier::{ConstrainedNsdfVerifier, VerifierConfig};
use gtcore::dsp::*;
use gtfixtures::*;

pub fn note(id: u32, target_hz: f64) -> ChartNote {
    ChartNote { id, start_s: 0.0, sustain_s: 0.5, string: 0, fret: 0, target_hz: target_hz as f32, legato: false }
}

/// Expected onset at `t` frames with the ±50 ms window.
pub fn expect(id: u32, target_hz: f64, t: u64) -> ExpectedNote {
    let w = ms_to_frames(50.0);
    ExpectedNote { note: note(id, target_hz), open: t - w, close: t + w }
}

pub fn run(sig: &[f32], expected: &[ExpectedNote], cfg: VerifierConfig) -> Vec<NoteEvidence> {
    let mut det = SpectralFluxOnset::new(OnsetConfig::default());
    let mut ver = ConstrainedNsdfVerifier::new(cfg);
    let mut evidence = Vec::new();
    let mut first: Frame = 0;
    for blk in sig.chunks(256) {
        let mut onsets = Vec::new();
        det.process(blk, first, &mut onsets);
        ver.process(blk, first, expected, &onsets, &mut evidence);
        first += blk.len() as Frame;
    }
    evidence
}

fn single(f_played: f64, f_expected: f64, seed: u64, pluck: impl Fn(Pluck) -> Pluck) -> Vec<NoteEvidence> {
    let t = 44100u64;
    let mut sig = vec![0.0f32; 44100 * 2];
    mix_at(&mut sig, &pluck(Pluck::new(f_played, seed)).render(44100), t as usize, 1.0);
    add_noise_floor(&mut sig, 3e-4, seed);
    run(&sig, &[expect(1, f_expected, t)], VerifierConfig::default())
}

fn describe(ev: &[NoteEvidence]) -> String {
    ev.iter()
        .map(|e| {
            format!(
                "{:?}/{:?} cents={:?} a={:.3} lat={:?}ms",
                e.kind,
                e.presence,
                e.cents_error.map(|c| (c * 10.0).round() / 10.0),
                e.aperiodicity,
                e.onset.map(|_o| ((e.decided_at as i64 - 44100) as f64 / 44.1 * 10.0).round() / 10.0),
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    if mode == "rates" { rates(); return; }
    if mode == "letring" { let_ring(); return; }
    if mode == "sweep" { let_ring_sweep(); return; }
    if mode == "scale" { scale_by_decay(); return; }
    if mode == "scaleseeds" { scale_seeds(); return; }
    println!("== clean plucks, expected = played");
    for midi in (40..=79).step_by(3) {
        let f = midi_to_hz(midi as f64);
        println!("midi {midi} {f:.1} Hz: {}", describe(&single(f, f, 5, |p| p)));
    }
    println!("== harmonic confusions (T-025)");
    let e2 = midi_to_hz(40.0);
    println!("exp E2 played E3: {}", describe(&single(midi_to_hz(52.0), e2, 5, |p| p)));
    println!("exp E2 played B3: {}", describe(&single(midi_to_hz(59.0), e2, 5, |p| p)));
    println!("exp E2 played E4: {}", describe(&single(midi_to_hz(64.0), e2, 5, |p| p)));
    println!("== lower note (T-026): exp E3 played E2");
    println!("{}", describe(&single(e2, midi_to_hz(52.0), 5, |p| p)));
    println!("== semitone/detune (T-027)");
    let a4 = 440.0;
    println!("exp A4 played A#4: {}", describe(&single(midi_to_hz(70.0), a4, 5, |p| p)));
    println!("exp A4 played A4+40c: {}", describe(&single(a4 * 2f64.powf(40.0 / 1200.0), a4, 5, |p| p)));
    println!("exp A4 played A4-40c: {}", describe(&single(a4 * 2f64.powf(-40.0 / 1200.0), a4, 5, |p| p)));
    println!("exp A4 played A4+60c: {}", describe(&single(a4 * 2f64.powf(60.0 / 1200.0), a4, 5, |p| p)));
    println!("== weak fundamental (T-032)");
    for &f in &[midi_to_hz(40.0), midi_to_hz(45.0)] {
        for db in [20.0, 30.0] {
            println!("H1 -{db} dB @ {f:.1}: {}", describe(&single(f, f, 5, |p| p.h1_atten(db))));
        }
    }
    println!("== attack glide (T-033)");
    for &f in &[midi_to_hz(40.0), midi_to_hz(57.0), midi_to_hz(76.0)] {
        println!("glide +25c/50ms @ {f:.1}: {}", describe(&single(f, f, 5, |p| p.glide(25.0, 0.05))));
    }
}

#[allow(dead_code)]
pub fn rates() {
    // Correct notes: Present rate + accuracy across E2..E5, several seeds, velocities, variants.
    let variants: Vec<(&str, Box<dyn Fn(Pluck) -> Pluck>)> = vec![
        ("clean", Box::new(|p| p)),
        ("quiet(0.1)", Box::new(|p| p.amp(0.1))),
        ("H1-20", Box::new(|p| p.h1_atten(20.0))),
        ("H1-30", Box::new(|p| p.h1_atten(30.0))),
        ("glide+25/50ms", Box::new(|p| p.glide(25.0, 0.05))),
        ("glide+40/80ms", Box::new(|p| p.glide(40.0, 0.08))),
        ("fast-decay 30dB/s", Box::new(|p| p.decay(30.0))),
    ];
    for (name, mk) in &variants {
        let (mut n, mut present, mut harm, mut absent) = (0, 0, 0, 0);
        let mut worst_refine = 0.0f32;
        let mut worst_lat_low = 0.0f64;
        let mut worst_lat_high = 0.0f64;
        for midi in 40..=79 {
            for seed in 0..3u64 {
                let f = midi_to_hz(midi as f64);
                let ev = single(f, f, 100 + seed, |p| mk(p));
                n += 1;
                let dec = ev.iter().find(|e| e.kind == EvidenceKind::Decision).unwrap();
                match dec.presence {
                    Presence::Present => {
                        present += 1;
                        let lat = (dec.decided_at as i64 - 44100) as f64 / 44.1;
                        if midi <= 56 { worst_lat_low = worst_lat_low.max(lat) } else { worst_lat_high = worst_lat_high.max(lat) }
                        if let Some(r) = ev.iter().find(|e| e.kind == EvidenceKind::Refinement) {
                            if let Some(c) = r.cents_error { worst_refine = worst_refine.max(c.abs()); }
                        }
                    }
                    Presence::HarmonicUp(_) => harm += 1,
                    Presence::Absent => absent += 1,
                }
            }
        }
        println!("[{name}] n={n} present={present} harmonicUp={harm} absent={absent} worst|refine cents|={worst_refine:.2} worst latency E2-G#3={worst_lat_low:.1}ms A3-E5={worst_lat_high:.1}ms");
    }
    // Wrong notes: expected vs played across intervals; count false accepts.
    let mut false_accept = std::collections::BTreeMap::new();
    let mut trials = std::collections::BTreeMap::new();
    for expected_midi in (40..=79).step_by(3) {
        for interval in [-12i32, -7, -5, -2, -1, 1, 2, 5, 7, 12, 19, 24] {
            let played = expected_midi + interval;
            if played < 36 || played > 88 { continue; }
            let ev = single(midi_to_hz(played as f64), midi_to_hz(expected_midi as f64), 300, |p| p);
            let dec = ev.iter().find(|e| e.kind == EvidenceKind::Decision).unwrap();
            *trials.entry(interval).or_insert(0) += 1;
            if dec.presence == Presence::Present { *false_accept.entry(interval).or_insert(0) += 1; }
        }
    }
    for (i, t) in &trials {
        println!("wrong-note interval {i:+}: trials {t} false-Present {}", false_accept.get(i).unwrap_or(&0));
    }
}

#[allow(dead_code)]
pub fn let_ring() {
    // T-034: expected A2 while a prior E2 rings. Vary how loud the ring is at the moment of the A2 pluck.
    let a2 = midi_to_hz(45.0);
    let e2 = midi_to_hz(40.0);
    for &(gap_ms, ring_db) in &[(300.0, -6.0), (300.0, -3.0), (300.0, 0.0), (300.0, -12.0), (600.0, -6.0)] {
        let t_a2 = 44100u64 + ms_to_frames(gap_ms);
        let decay = 8.0f64;
        let amp_a2 = 0.5f64;
        let target_level = amp_a2 * 10f64.powf(ring_db / 20.0);
        let amp_e2 = target_level * 10f64.powf(decay * gap_ms / 1000.0 / 20.0);
        let mut sig = vec![0.0f32; 44100 * 3];
        mix_at(&mut sig, &Pluck::new(e2, 21).amp(amp_e2 as f32).render(44100 * 2), 44100, 1.0);
        mix_at(&mut sig, &Pluck::new(a2, 22).amp(amp_a2 as f32).render(44100), t_a2 as usize, 1.0);
        add_noise_floor(&mut sig, 3e-4, 5);
        let ev = run(&sig, &[expect(1, a2, t_a2)], VerifierConfig::default());
        let s: Vec<String> = ev.iter().map(|e| format!("{:?}/{:?} a={:.3} cents={:?}", e.kind, e.presence, e.aperiodicity, e.cents_error.map(|c| (c*10.0).round()/10.0))).collect();
        println!("A2 over E2 ring {ring_db:+} dB (gap {gap_ms} ms): {}", s.join(" | "));
    }
    // Same pitch repeated: E2 rings, E2 replucked expected -> requires attributed onset
}

/// R-108 sweep: a previous note (interval semitones below/above the expected
/// note) rings `ring_db` below the new pluck at the moment it is plucked.
/// Prints the Present rate over registers; the row shows where it breaks.
#[allow(dead_code)]
pub fn let_ring_sweep() {
    let decay = 8.0f64;
    let gap_ms = 450.0;
    let rings = [-1.0f64, -3.0, -4.0, -5.0, -6.0, -8.0, -10.0, -14.0];
    print!("interval / ring dB:");
    for r in rings { print!("{r:>7}"); }
    println!();
    for interval in [-12i32, -7, -5, -2, -1, 1, 2, 5, 7, 12] {
        print!("prev {interval:+3} st       ");
        for &ring_db in &rings {
            let (mut ok, mut n) = (0, 0);
            for midi in [45i32, 50, 57, 62, 69, 74] {
                let prev_midi = midi + interval;
                if !(40..=80).contains(&prev_midi) { continue; }
                let t_new = T0_S + ms_to_frames(gap_ms);
                let amp_new = 0.5f64;
                let level = amp_new * 10f64.powf(ring_db / 20.0);
                let amp_prev = level * 10f64.powf(decay * gap_ms / 1000.0 / 20.0);
                let mut sig = vec![0.0f32; 44100 * 3];
                mix_at(&mut sig, &Pluck::new(midi_to_hz(prev_midi as f64), 31).amp(amp_prev as f32).render(44100 * 2), T0_S as usize, 1.0);
                mix_at(&mut sig, &Pluck::new(midi_to_hz(midi as f64), 32).amp(amp_new as f32).render(44100), t_new as usize, 1.0);
                add_noise_floor(&mut sig, 3e-4, 5);
                let ev = run(&sig, &[expect(1, midi_to_hz(midi as f64), t_new)], VerifierConfig::default());
                n += 1;
                if ev.iter().any(|e| e.kind == EvidenceKind::Decision && e.presence == Presence::Present) { ok += 1; }
            }
            print!("{:>4}/{:<2}", ok, n);
        }
        println!();
    }
}
const T0_S: u64 = 44100;

/// C-major scale C3..C5 at 0.45 s spacing through the full verifier, for a
/// range of string decay rates: which notes are rejected?
#[allow(dead_code)]
pub fn scale_by_decay() {
    let steps = [0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21, 23, 24];
    for spacing in [0.45f64, 0.30] {
        for decay in [8.0f64, 10.0, 12.0, 16.0, 24.0] {
            let mut sig = vec![0.0f32; 44100 * 12];
            let mut expected = Vec::new();
            for (i, s) in steps.iter().enumerate() {
                let t = T0_S + ms_to_frames(spacing * 1000.0 * i as f64);
                let f = midi_to_hz(48.0 + *s as f64);
                let mut p = Pluck::new(f, 1000 + i as u64).amp(0.6).decay(decay).render(ms_to_frames(900.0) as usize);
                fade_out(&mut p, ms_to_frames(120.0) as usize);
                mix_at(&mut sig, &p, t as usize, 1.0);
                expected.push(expect(i as u32 + 1, f, t));
            }
            add_noise_floor(&mut sig, 3e-4, 5);
            let ev = run(&sig, &expected, VerifierConfig::default());
            let bad: Vec<String> = ev
                .iter()
                .filter(|e| e.kind == EvidenceKind::Decision && e.presence != Presence::Present)
                .map(|e| format!("{}:{:?}", e.note_id, e.presence))
                .collect();
            println!("spacing {spacing:.2}s decay {decay:>4} dB/s -> rejected: {}", if bad.is_empty() { "none".to_string() } else { bad.join(" ") });
        }
    }
}

/// Scale pass rate over many excitation seeds with `render_part` (realistic decay).
#[allow(dead_code)]
pub fn scale_seeds() {
    let steps = [0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21, 23, 24];
    for spacing in [0.60f64, 0.45, 0.30] {
        let (mut runs_clean, mut notes_bad, mut notes_total) = (0, 0, 0);
        let n_seeds = 30;
        for seed in 0..n_seeds {
            let plucks: Vec<PluckAt> = steps.iter().enumerate().map(|(i, s)| PluckAt {
                frame: (T0_S + ms_to_frames(spacing * 1000.0 * i as f64)) as usize,
                f0_hz: midi_to_hz(48.0 + *s as f64), amp: 0.6 }).collect();
            let sig = render_part(&plucks, 44100 * 12, 900.0, 5000 + seed);
            let expected: Vec<_> = plucks.iter().enumerate().map(|(i, p)| expect(i as u32 + 1, p.f0_hz, p.frame as u64)).collect();
            let ev = run(&sig, &expected, VerifierConfig::default());
            let bad = ev.iter().filter(|e| e.kind == EvidenceKind::Decision && e.presence != Presence::Present).count();
            notes_bad += bad; notes_total += expected.len();
            if bad == 0 { runs_clean += 1; }
        }
        println!("realistic decay, spacing {spacing:.2}s: {runs_clean}/{n_seeds} scales fully Present; {notes_bad}/{notes_total} notes rejected");
    }
}

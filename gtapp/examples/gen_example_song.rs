//! Generates the example song (plan step S-008, A-005: an original procedural
//! composition) into a directory:
//!
//!   cargo run --release -p gtapp --example gen_example_song -- assets/songs/example
//!
//! Writes `chart.json`, `backing.wav` (synthesized bass + drums, mono 16-bit)
//! and `guitar.wav` (the guitar part rendered with the Karplus–Strong fixture
//! generator, aligned to song time). Fully deterministic. The committed files
//! are the output of this program.

use gtaudio::wav::encode_pcm16;
use gtfixtures::*;
use std::path::PathBuf;

const BPM: f64 = 90.0;
const LEAD_S: f64 = 2.0; // song time of beat 0
const TAIL_S: f64 = 2.5;
const OPEN_MIDI: [i32; 6] = [40, 45, 50, 55, 59, 64];

/// (beat, midi note, length in beats). 8 bars of 4/4 in A minor. Note spacing
/// is kept ≥ 0.33 s (see HANDOFF.md, R-108: faster ringing passages are a
/// known weak spot of the verifier).
#[rustfmt::skip]
const MELODY: &[(f64, i32, f64)] = &[
    // bars 1–2: low riff (E2–E3)
    (0.0, 45, 1.0), (1.0, 45, 1.0), (2.0, 48, 1.0), (3.0, 50, 1.0),
    (4.0, 52, 1.0), (5.0, 50, 1.0), (6.0, 48, 1.0), (7.0, 45, 1.0),
    // bars 3–4: climbing from the open low E
    (8.0, 40, 1.0), (9.0, 43, 1.0), (10.0, 45, 1.0), (11.0, 47, 1.0),
    (12.0, 48, 1.0), (13.0, 47, 1.0), (14.0, 45, 1.0), (15.0, 43, 1.0),
    // bars 5–6: melody in the middle register
    (16.0, 57, 1.0), (17.0, 60, 1.0), (18.0, 64, 1.0), (19.0, 69, 1.0),
    (20.0, 67, 1.0), (21.0, 64, 1.0), (22.0, 62, 0.5), (22.5, 60, 0.5), (23.0, 60, 1.0),
    // bars 7–8: up to E5 and home
    (24.0, 69, 1.0), (25.0, 72, 1.0), (26.0, 76, 1.0), (27.0, 72, 1.0),
    (28.0, 69, 1.0), (29.0, 64, 1.0), (30.0, 57, 1.0), (31.0, 45, 2.0),
];

fn position(midi: i32) -> (u8, u8) {
    // Highest string that can play the note within the first 12 frets.
    for s in (0..6).rev() {
        let fret = midi - OPEN_MIDI[s];
        if (0..=12).contains(&fret) {
            return (s as u8, fret as u8);
        }
    }
    panic!("note {midi} is outside E2–E5");
}

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).expect("usage: gen_example_song <output_dir>"));
    std::fs::create_dir_all(&out).unwrap();
    let beat_s = 60.0 / BPM;
    let end_beat = MELODY.iter().map(|(b, _, l)| b + l).fold(0.0, f64::max);
    let total_s = LEAD_S + end_beat * beat_s + TAIL_S;
    let total = (total_s * FS) as usize;

    // ---- chart + guitar part
    let mut notes = Vec::new();
    let mut plucks = Vec::new();
    for (i, &(beat, midi, len)) in MELODY.iter().enumerate() {
        let (string, fret) = position(midi);
        let start_s = LEAD_S + beat * beat_s;
        notes.push(serde_json::json!({
            "id": i as u32 + 1,
            "start_s": (start_s * 1e6).round() / 1e6,
            "sustain_s": ((len * beat_s - 0.05) * 1e6).round() / 1e6,
            "string": string,
            "fret": fret,
        }));
        plucks.push(PluckAt { frame: (start_s * FS).round() as usize, f0_hz: midi_to_hz(midi as f64), amp: 0.55 });
    }
    let guitar = render_part(&plucks, total, 1800.0, 20260925);

    // ---- backing: bass on the beat, kick on 1 & 3, hats on eighths
    let mut backing = vec![0.0f32; total];
    let roots = [45, 41, 48, 43, 45, 41, 48, 40]; // one per bar (A F C G A F C E)
    let mut rng = Rng::new(77);
    let bars = (end_beat / 4.0).ceil() as usize;
    for bar in 0..bars {
        for beat in 0..4 {
            let t = LEAD_S + (bar * 4 + beat) as f64 * beat_s;
            let f = midi_to_hz(roots[bar % roots.len()] as f64 - 12.0);
            let start = (t * FS) as usize;
            let len = (beat_s * 0.9 * FS) as usize;
            for n in 0..len.min(total.saturating_sub(start)) {
                let tt = n as f64 / FS;
                let env = (-tt * 5.0).exp() * (1.0 - (-tt * 400.0).exp());
                let ph = 2.0 * std::f64::consts::PI * f * tt;
                backing[start + n] += (0.30 * env * (ph.sin() + 0.35 * (2.0 * ph).sin())) as f32;
            }
            if beat % 2 == 0 {
                for n in 0..(0.18 * FS) as usize {
                    if start + n >= total {
                        break;
                    }
                    let tt = n as f64 / FS;
                    let phase = 2.0 * std::f64::consts::PI * (50.0 * tt + 70.0 * (1.0 - (-tt * 30.0).exp()) / 30.0);
                    backing[start + n] += (0.35 * (-tt * 22.0).exp() * phase.sin()) as f32;
                }
            }
            for half in 0..2 {
                let hs = ((t + half as f64 * 0.5 * beat_s) * FS) as usize;
                let mut prev = 0.0f64;
                for n in 0..(0.03 * FS) as usize {
                    if hs + n >= total {
                        break;
                    }
                    let x = rng.bipolar();
                    let hp = x - prev; // crude high-pass
                    prev = x;
                    backing[hs + n] += (0.06 * (-(n as f64) / FS * 120.0).exp() * hp) as f32;
                }
            }
        }
    }
    let peak = backing.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-6);
    backing.iter_mut().for_each(|s| *s *= 0.6 / peak);

    // ---- write
    let chart = serde_json::json!({
        "title": "Quiet Arithmetic",
        "assets": { "backing_track": "backing.wav", "guitar_reference": "guitar.wav" },
        "notes": notes,
    });
    std::fs::write(out.join("chart.json"), serde_json::to_string_pretty(&chart).unwrap() + "\n").unwrap();
    std::fs::write(out.join("backing.wav"), encode_pcm16(FS as u32, 1, &backing)).unwrap();
    std::fs::write(out.join("guitar.wav"), encode_pcm16(FS as u32, 1, &guitar)).unwrap();
    println!("wrote {} notes, {:.1} s, to {}", MELODY.len(), total_s, out.display());
}

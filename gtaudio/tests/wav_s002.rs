//! WAV reader/writer tests: round trips and hostile input (never panic).

use gtaudio::wav::*;

fn tone(n: usize) -> Vec<f32> {
    (0..n).map(|i| 0.8 * (i as f32 * 0.05).sin()).collect()
}

#[test]
fn pcm16_round_trip_mono_and_stereo() {
    let mono = tone(1000);
    let w = parse(&encode_pcm16(44100, 1, &mono)).unwrap();
    assert_eq!((w.sample_rate, w.channels, w.frames()), (44100, 1, 1000));
    for (a, b) in mono.iter().zip(&w.samples) {
        assert!((a - b).abs() <= 1.0 / 32767.0 + 1e-6);
    }

    let stereo: Vec<f32> = mono.iter().flat_map(|&s| [s, -s]).collect();
    let w = parse(&encode_pcm16(48000, 2, &stereo)).unwrap();
    assert_eq!((w.sample_rate, w.channels, w.frames()), (48000, 2, 1000));
    let m = w.to_mono();
    assert!(m.iter().all(|s| s.abs() < 1e-4), "L and −L average to zero");
}

#[test]
fn clamps_out_of_range_samples_when_encoding() {
    let w = parse(&encode_pcm16(44100, 1, &[2.0, -2.0, 0.0])).unwrap();
    assert!((w.samples[0] - 1.0).abs() < 1e-3 && (w.samples[1] + 1.0).abs() < 1e-3 && w.samples[2] == 0.0);
}

fn build(fmt_tag: u16, bits: u16, data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&((36 + data.len()) as u32).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&fmt_tag.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&44100u32.to_le_bytes());
    v.extend_from_slice(&(44100 * (bits as u32 / 8)).to_le_bytes());
    v.extend_from_slice(&(bits / 8).to_le_bytes());
    v.extend_from_slice(&bits.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&(data.len() as u32).to_le_bytes());
    v.extend_from_slice(data);
    v
}

#[test]
fn reads_float32_24bit_and_8bit() {
    let f: Vec<u8> = [0.5f32, -0.25].iter().flat_map(|s| s.to_le_bytes()).collect();
    assert_eq!(parse(&build(3, 32, &f)).unwrap().samples, vec![0.5, -0.25]);

    // 24-bit: +0.5 = 0x400000, -0.5 = 0xC00000.
    let w24 = [0x00, 0x00, 0x40, 0x00, 0x00, 0xC0];
    let s = parse(&build(1, 24, &w24)).unwrap().samples;
    assert!((s[0] - 0.5).abs() < 1e-6 && (s[1] + 0.5).abs() < 1e-6, "{s:?}");

    let s = parse(&build(1, 8, &[128, 255, 0])).unwrap().samples;
    assert!(s[0].abs() < 1e-6 && s[1] > 0.99 && s[2] < -0.99);
}

#[test]
fn rejects_bad_headers_and_formats() {
    assert_eq!(parse(b"").unwrap_err(), WavError::NotRiffWave);
    assert_eq!(parse(b"RIFF\0\0\0\0WAVX").unwrap_err(), WavError::NotRiffWave);
    let mut no_data = build(1, 16, &[]);
    no_data.truncate(36);
    assert_eq!(parse(&no_data).unwrap_err(), WavError::MissingChunk("data"));
    assert!(matches!(parse(&build(2, 4, &[0; 8])), Err(WavError::Unsupported(_))), "ADPCM-ish tag");
    assert!(matches!(parse(&build(1, 12, &[0; 8])), Err(WavError::Unsupported(_))));
}

/// Hostile input must produce an error or a value, never a panic: every
/// truncation of a valid file, and a corpus of corrupted headers.
#[test]
fn malformed_input_never_panics() {
    let valid = encode_pcm16(44100, 2, &tone(64));
    for cut in 0..valid.len() {
        let _ = parse(&valid[..cut]);
    }
    // Flip each header byte to every "interesting" value.
    for i in 0..44 {
        for v in [0u8, 1, 0x7F, 0x80, 0xFF] {
            let mut bad = valid.clone();
            bad[i] = v;
            let _ = parse(&bad);
        }
    }
    // Chunk sizes claiming more than the file has / overflowing.
    let mut huge = valid.clone();
    huge[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
    let w = parse(&huge).expect("a data chunk that overruns the file is tolerated");
    assert_eq!(w.frames(), 32);
    let mut huge_fmt = valid.clone();
    huge_fmt[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(parse(&huge_fmt).is_err());
    // Zero channels / zero rate.
    let mut zero = valid.clone();
    zero[22] = 0;
    zero[23] = 0;
    assert!(matches!(parse(&zero), Err(WavError::Unsupported(_))));
}

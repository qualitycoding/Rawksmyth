//! RT core + file backend: duplex semantics (C-011), determinism, and
//! injected device loss (T-011, CI part).

use gtaudio::backend::{AudioBackend, BackendError, FileBackend};
use gtaudio::rt::{RtConfig, RtCore};

fn backing(frames: usize) -> Vec<f32> {
    (0..frames).flat_map(|i| [(i as f32 * 0.001).sin() * 0.5, (i as f32 * 0.002).cos() * 0.5]).collect()
}

fn input_signal(frames: usize) -> Vec<f32> {
    (0..frames).map(|i| ((i % 1000) as f32 - 500.0) / 1000.0).collect()
}

#[test]
fn backing_plays_from_s0_input_reaches_the_ring_and_clock_advances() {
    let s0 = 3000u64;
    let bk = backing(10_000);
    let input = input_signal(20_000);
    let (mut core, mut h) = RtCore::new(
        RtConfig { channels: 2, s0, input_ring_samples: 1 << 16 },
        bk.clone(),
        vec![],
    );
    let mut fb = FileBackend::new(input.clone(), 44100.0).block_size(256).channels(2);
    let mut got_input = Vec::new();
    let mut scratch = vec![0.0f32; 256];
    let frames = fb
        .run(&mut |inp, out, ns| {
            core.callback(inp, out, ns);
            let k = h.input.pop_samples(&mut scratch);
            got_input.extend_from_slice(&scratch[..k]);
        })
        .unwrap();
    assert_eq!(frames, 20_000);
    assert_eq!(core.frame(), 20_000);
    assert_eq!(got_input, input, "input delivered bit-exactly and in order");

    let out = fb.captured_output();
    assert_eq!(out.len(), 20_000 * 2, "equal input/output frame counts");
    assert!(out[..(s0 as usize) * 2].iter().all(|&s| s == 0.0), "silence before s0");
    let b0 = s0 as usize * 2;
    assert_eq!(&out[b0..b0 + 20_000], &bk[..20_000], "backing starts exactly at s0");
    assert!(out[(s0 as usize + 10_000) * 2..].iter().all(|&s| s == 0.0), "silence after the track");
    let c = h.clock.read().unwrap();
    assert_eq!(c.frames_written, 20_000);
}

#[test]
fn sfx_is_mixed_on_trigger_only() {
    let (mut core, mut h) = RtCore::new(
        RtConfig { channels: 2, s0: 0, input_ring_samples: 4096 },
        vec![0.0; 2 * 4096],
        vec![vec![0.25; 300]],
    );
    let mut fb = FileBackend::new(vec![0.0; 2048], 44100.0).block_size(256);
    let mut blocks = 0;
    fb.run(&mut |inp, out, ns| {
        if blocks == 2 {
            h.commands.push_words(&[0]);
        }
        core.callback(inp, out, ns);
        blocks += 1;
    })
    .unwrap();
    let out = fb.captured_output();
    // Trigger pushed before block 2's callback → audible from block 2's start.
    let first_loud = out.iter().position(|&s| s != 0.0).unwrap();
    assert_eq!(first_loud, 2 * 256 * 2);
    let loud = out.iter().filter(|&&s| s != 0.0).count();
    assert_eq!(loud, 300 * 2, "clip of 300 frames on both channels, once");
}

#[test]
fn file_backend_is_deterministic_including_jitter() {
    let run_once = || {
        let (mut core, _h) = RtCore::new(RtConfig { channels: 2, s0: 100, input_ring_samples: 1 << 16 }, backing(5000), vec![]);
        let mut fb = FileBackend::new(input_signal(5000), 44100.0).jitter(2_000_000.0, 77);
        let mut stamps = Vec::new();
        fb.run(&mut |i, o, ns| {
            stamps.push(ns);
            core.callback(i, o, ns)
        })
        .unwrap();
        (stamps, fb.captured_output().to_vec())
    };
    let (a, out_a) = run_once();
    let (b, out_b) = run_once();
    assert_eq!(a, b);
    assert_eq!(out_a, out_b);
    // Jitter is bounded to ±2 ms around the ideal timestamps.
    for (k, ns) in a.iter().enumerate() {
        let ideal = ((k + 1) * 256).min(5000) as f64 / 44100.0 * 1e9;
        assert!((*ns as f64 - ideal).abs() <= 2_000_001.0, "callback {k}");
    }
}

#[test]
fn tail_frames_keep_the_stream_running_after_the_input_ends() {
    let mut fb = FileBackend::new(vec![0.5; 1000], 44100.0).block_size(300).tail_frames(700);
    let mut seen = Vec::new();
    let frames = fb.run(&mut |i, _o, _| seen.extend_from_slice(i)).unwrap();
    assert_eq!(frames, 1700);
    assert_eq!(seen.len(), 1700);
    assert!(seen[..1000].iter().all(|&s| s == 0.5) && seen[1000..].iter().all(|&s| s == 0.0));
}

/// T-011 (CI part): a device that disappears mid-stream is reported as an
/// error at a well-defined frame; frames before it were fully delivered; the
/// RT core is left consistent (no panic).
#[test]
fn t011_injected_device_loss_is_an_error_not_a_panic() {
    let (mut core, mut h) = RtCore::new(RtConfig { channels: 2, s0: 0, input_ring_samples: 1 << 16 }, backing(50_000), vec![]);
    let mut fb = FileBackend::new(input_signal(50_000), 44100.0).block_size(256).fail_at_frame(10_000);
    let mut delivered = 0usize;
    let mut scratch = vec![0.0f32; 256];
    let r = fb.run(&mut |i, o, ns| {
        core.callback(i, o, ns);
        delivered += h.input.pop_samples(&mut scratch);
    });
    match r {
        Err(BackendError::DeviceLost { at_frame }) => {
            assert!((10_000..10_000 + 256).contains(&at_frame), "lost at {at_frame}");
            assert_eq!(delivered as u64, at_frame, "every frame before the loss was delivered");
            assert_eq!(core.frame(), at_frame);
        }
        other => panic!("expected DeviceLost, got {other:?}"),
    }
    assert!(format!("{}", BackendError::DeviceLost { at_frame: 5 }).contains("lost"));
}

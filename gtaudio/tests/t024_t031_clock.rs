//! T-024 (render clock under callback jitter) and T-031 (two devices whose
//! clocks differ by 50 ppm), on the file backend's simulated timestamps.

use gtaudio::backend::simulate_two_devices;
use gtcore::clock::{render_song_time, ClockModel, ClockSample, RenderClockFit};
use gtcore::drift::DriftEstimator;

/// Worst render-clock error (ms) over 20 jitter seeds when the game thread
/// fits the last `window` callbacks; also asserts monotonicity throughout.
fn worst_render_error_ms(window: usize) -> f64 {
    let fs = 44100.0;
    let clock = ClockModel { fs, s0: 0, l_in: 0.0, l_out: 0.0, delta_a: 0.0 };
    let block = 256u64;
    let mut worst_ms = 0.0f64;
    for seed in 1..=20u64 {
        let (samples, _) = simulate_two_devices(30.0, fs, fs, block, 2_000_000.0, seed);
        let mut prev: Option<f64> = None;
        let mut now_ns = 0i64;
        let mut last_song_t = f64::MIN;
        // The game thread renders every 16.7 ms; it sees callbacks whose
        // (jittered) timestamps are ≤ now.
        while now_ns < 29_000_000_000 {
            now_ns += 16_700_000;
            let visible: Vec<ClockSample> =
                samples.iter().copied().filter(|s| s.monotonic_ns <= now_ns).collect();
            if visible.len() < window {
                continue;
            }
            let fit = RenderClockFit::fit(&visible[visible.len() - window..]).unwrap();
            let (song_t, frames) = render_song_time(&fit, now_ns, 0.0, 0.0, &clock, prev);
            prev = Some(frames);
            assert!(song_t >= last_song_t, "render clock went backwards (seed {seed})");
            last_song_t = song_t;
            let true_frames = now_ns as f64 * 1e-9 * fs;
            let err_ms = (frames - true_frames).abs() / fs * 1e3;
            worst_ms = worst_ms.max(err_ms);
        }
    }
    worst_ms
}

/// T-024: with ±2 ms callback-timestamp jitter, the render clock is monotonic
/// and within 1 ms of true device time.
///
/// Finding (recorded in HANDOFF.md): plan §2B.1d specifies a fit over the
/// last 32 callbacks. Measured here, that window has a worst-case error of
/// ≈1.7 ms under ±2 ms uniform jitter — it cannot meet the ≤1 ms requirement
/// (theory: prediction rms ≈ 0.42 ms, worst over thousands of renders ≈ 3.5σ).
/// The window is therefore `RENDER_FIT_WINDOW` (128 callbacks ≈ 0.74 s at 256
/// frames), which does.
#[test]
fn t024_render_clock_under_2ms_jitter_is_monotonic_and_within_1ms() {
    for w in [32usize, 64, 128, 256] {
        eprintln!("T-024 window {w:>3}: worst error {:.3} ms", worst_render_error_ms(w));
    }
    let worst = worst_render_error_ms(gtcore::drift::RENDER_FIT_WINDOW);
    assert!(worst <= 1.0, "render clock error {worst:.3} ms > 1 ms with window {}", gtcore::drift::RENDER_FIT_WINDOW);
}

/// T-031: input device at 44102.2 Hz, output at 44100 Hz (50 ppm). Over 300 s
/// the drift estimator keeps the input→song-time mapping within 2 ms, and a
/// warning is produced. A naive mapping that ignores drift is >10 ms off,
/// which shows the test can fail.
#[test]
fn t031_fifty_ppm_drift_is_tracked_within_2ms_over_300s() {
    let fs = 44100.0;
    let in_hz = 44102.2;
    let clock = ClockModel { fs, s0: 44100, l_in: 0.010, l_out: 0.015, delta_a: 0.0 };
    let (in_samples, out_samples) = simulate_two_devices(300.0, in_hz, fs, 256, 1_000_000.0, 4242);

    let mut est = DriftEstimator::default();
    let (mut ii, mut oi) = (0usize, 0usize);
    let mut worst_ms = 0.0f64;
    let mut worst_naive_ms = 0.0f64;
    let mut warned_at = None;
    // Advance wall time in 1 s steps; evaluate the mapping of the most
    // recent input frame each step (after 20 s of warm-up).
    for sec in 1..=299u64 {
        let t_ns = sec as i64 * 1_000_000_000;
        while ii < in_samples.len() && in_samples[ii].monotonic_ns <= t_ns {
            est.push_input(in_samples[ii]);
            ii += 1;
        }
        while oi < out_samples.len() && out_samples[oi].monotonic_ns <= t_ns {
            est.push_output(out_samples[oi]);
            oi += 1;
        }
        if warned_at.is_none() && est.warning().is_some() {
            warned_at = Some(sec);
        }
        if sec < 20 {
            continue;
        }
        // The input frame captured at true wall time t.
        let i = (sec as f64 * in_hz) as u64;
        // Ground truth: the output device has played t·fs frames by then.
        let truth = (sec as f64 * fs - clock.s0 as f64) / fs - clock.l_rt();
        let est_t = est.song_time(&clock, i).unwrap();
        worst_ms = worst_ms.max((est_t - truth).abs() * 1e3);
        worst_naive_ms = worst_naive_ms.max((clock.song_time(i) - truth).abs() * 1e3);
    }
    eprintln!("T-031: worst corrected error {worst_ms:.3} ms; naive (no drift correction) {worst_naive_ms:.1} ms; warned at {warned_at:?} s");
    assert!(worst_ms <= 2.0, "corrected timing error {worst_ms:.3} ms > 2 ms");
    assert!(worst_naive_ms > 10.0, "sanity: without correction the error should exceed 10 ms, got {worst_naive_ms:.2}");
    let w = est.warning().expect("a warning must be produced for 50 ppm drift");
    assert!(w.contains("+50") || w.contains("+49") || w.contains("+51"), "warning should state the measured drift: {w}");
    assert!(warned_at.unwrap() <= 60, "warning came too late: {warned_at:?}");
}

/// Two devices that share a clock (same true rate) must not warn.
#[test]
fn no_warning_when_both_devices_share_a_clock() {
    let (i, o) = simulate_two_devices(120.0, 44100.0, 44100.0, 256, 1_000_000.0, 7);
    let mut est = DriftEstimator::default();
    i.into_iter().for_each(|s| est.push_input(s));
    o.into_iter().for_each(|s| est.push_output(s));
    assert!(est.warning().is_none(), "{:?}", est.warning());
    assert!(est.fit().unwrap().drift_ppm().abs() < 10.0);
}

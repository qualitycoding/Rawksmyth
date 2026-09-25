# Spike 1 — cubeb duplex (partial: no valid round-trip latency yet)

Code: `research/spikes/spike-cubeb-duplex` (standalone, not in the main
workspace; needs cmake and a C++ toolchain).

```
cargo run --release --manifest-path research/spikes/spike-cubeb-duplex/Cargo.toml -- list
cargo run --release --manifest-path research/spikes/spike-cubeb-duplex/Cargo.toml -- duplex \
    [--rate 44100] [--latency 256] [--in N] [--out N] [--gain 0.3] [--clicks 12] [--silent] \
    [--only-in|--only-out] [--device-changed-cb] [--json FILE]
```

It drives the production `gtaudio::rt::RtCore` callback from a real cubeb
stream, plays a chirp train, records the input, and finds each chirp by
normalized cross-correlation (with a peak-to-sidelobe test and a −80 dBFS
level floor, so it cannot report a latency from noise).

## What was run, and on what

One run on the developer's laptop (Windows 10, cubeb 0.38 / WASAPI shared
mode, Conexant ISST codec, internal microphone and speakers). **This is not
the plan's reference setup**: no audio interface, no electrical loopback cable,
no guitar. The spike was run on Windows only.

## Results that are valid

| Claim | Result |
|---|---|
| **C-011** equal input/output frame counts in a duplex callback | **Held on this setup**: 0 of 1183 callbacks (and 0 of 1181 in a later run) had input length ≠ output length; no zero-length input callbacks. These runs used two different endpoints (`--in` Internal Microphone, `--out` Speakers) of the same codec, so this also shows cubeb/WASAPI accepts a duplex stream across distinct endpoints (C-014, partly). Same physical codec and clock, WASAPI shared only — not evidence for CoreAudio/PulseAudio, exclusive mode, or truly separate devices with independent clocks. |
| Callback size | 441 frames (10.0 ms) at a requested 44100 Hz / 256 frames; one 440 and one 972 at start-up. The plan's **256-frame request was not honoured**. `ctx.min_latency()` for 44.1 kHz stereo f32 said 441. |
| Callback timing | interval mean 10.001 ms, sd 0.31–0.34 ms, p99 10.7–10.8 ms, max 11.0–12.6 ms, over ~1180 callbacks (12 s). |
| RT core under a real device | `RtCore` ran 1183 callbacks with 0 input-ring overruns and its frame counter equal to the frames delivered. |
| Native device format | 48 000 Hz only, stereo. The plan pins fs = 44 100 Hz; WASAPI shared therefore **resamples** 44.1 → 48 kHz inside cubeb/the driver. Untested consequence: added latency (see below) and whether resampling affects timestamps. |
| Driver-reported latency | `stream.latency()` (output) 1499–1735 frames (34–39 ms), `stream.input_latency()` 506–562 frames (11–13 ms) — **the values changed from run to run** (total 2005–2299 frames, 45–52 ms). Reported latency is not a constant. |
| Enumeration | Works: input devices include a *disabled* "Stereo Mix" and an unplugged external mic; default output was "Headphones", not the speakers. |

## Findings that affect the real backend

1. **COM must be initialized.** cubeb's WASAPI backend aborts the process
   (`cubeb_wasapi.cpp: hr != CO_E_NOTINITIALIZED`) if the calling thread has not
   called `CoInitializeEx`; cubeb-rs does not. The real `CubebBackend` must do it
   on every thread that touches cubeb.
2. **`device_changed_cb` breaks stream creation on WASAPI.** Registering it via
   cubeb-rs's `StreamBuilder::device_changed_cb` makes `init` fail with
   `NotSupported` (single-direction streams too). Without it, streams open. So
   device-change/loss detection cannot rely on that callback here; use the
   `State::Error` state callback and/or endpoint notifications. This matters for
   T-011 on hardware.
3. **cubeb-rs duplex uses one frame type for both directions.** Mono-in /
   stereo-out is not expressible; the spike captures stereo and uses channel 0.
   On an interface, guitar-on-input-1 is channel 0, which works, but the mapping
   must be explicit.
4. **The 256-frame request is a request.** Expect 10 ms (441-frame) callbacks
   from WASAPI shared. The plan's latency budget (§2B.1c) assumes 5.8 ms input
   buffering; it does not affect scoring accuracy (frame-index based) but does
   affect the informational feedback-latency target (≤ 100 ms).

## What could NOT be measured

**Round-trip latency L_rt (measured vs reported, i.e. R-104) — no valid result.**
The internal microphone delivered digital silence (overall RMS −158 dBFS) even
though the OS microphone-privacy setting is "Allow". Likely causes: the mic is
muted in Windows sound settings, or the Conexant driver's echo cancellation /
noise suppression removes the playback from the mic path. I did not change any
audio settings on the machine.

An earlier version of the analysis printed "12 of 12 clicks detected, 211.79 ms"
from that silent input. **That number is spurious** (normalized correlation is
scale-invariant and matched the chirp to numerical residue at −158 dBFS). The
analysis now requires ≥ −80 dBFS and reports "no valid clicks" instead. An
earlier 5 ms noise-burst stimulus also failed (1 of 12 at correlation 0.40 and
250 ms) and was likewise not a measurement.

Still needed for spike 1 proper (needs the plan's hardware):
- C-014 for real: input and output devices with *independent clocks* (e.g. USB interface in + built-in out), where drift (R-105) appears.
- L_rt on the reference interface with a loopback cable: `duplex --in N --out M`
  with the cable connected; check `measured − reported` per platform (R-104).
- WASAPI exclusive mode and the 44.1 kHz-native case (D-005, T-019).
- macOS (CoreAudio) and Linux (PulseAudio/PipeWire).
- A 10-minute run for the T-021 hardware overrun criterion (these runs were 12 s).

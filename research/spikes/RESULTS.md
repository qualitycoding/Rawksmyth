# Spike results (synthetic fixtures only)

**Read this first.** These numbers come from the Karplus–Strong fixture
generator (`tests/fixtures/gen`) running through the CI-tier code. They are
*not* real-guitar measurements: plan Phase 1 (R2–R6 research, spikes 1 and 4,
real-guitar validation) was never run, and G-001 has not happened. KS plucks
are cleaner than a real guitar: instantaneous noise-burst attack, perfectly
harmonic partials, no pick noise, no fret buzz, no pickup coloration, no
sympathetic strings. Treat every "passes" below as "meets the specification on
idealized input", and expect real-guitar false-reject rates to be worse.

Reproduce with `cargo run --release -p gtcore --example <name> [-- mode]`.

## Fixture generator ground truth

Independent DFT peak search over 0.4 s of sustain: fundamental within ±2 cents
of the requested f0 for MIDI 40–79 (`tuning_selftest`). The loop's all-pass is
tuned for the phase delay *at f0*, not at DC.

## Spike 3 — onset detector (`--example spike_onset`)

512/128 log-magnitude spectral flux, γ = 1000, max-filter 1 bin on the previous
frame (SuperFlux), 3-hop lookahead, threshold `4 + 2·mean(previous 12 hops)`,
40 ms refractory, first-difference envelope backtracking.

| Run | Plucks | Missed | Spurious | \|error\| median / p95 / max |
|---|---|---|---|---|
| Seeds 1–10 (thresholds tuned here) | 500 | 0 | 0 | 0.00 / 0.02 / 0.14 ms |
| Seeds 100–139 (held out) | 2000 | 1 | 0 | 0.00 / 0.02 / 0.23 ms |

Design findings during calibration (each was a real failure before the fix):

- Plain spectral flux gave a background flux of 10–19 units from *beating
  between closely spaced partials of ringing low notes* (partial spacing <
  one 86 Hz bin), which masked quiet onsets (misses) or, at lower thresholds,
  produced spurious onsets (up to 757 in 500 plucks). Max-filtering the
  previous frame fixed it.
- Backtracking on the raw signal was 4–24 ms early when a low note was
  ringing; using the first difference (high-pass) fixed it.
- The fixture initially truncated plucks abruptly, which the detector
  correctly reported as onsets; fixed in the fixture (fade-out).
- The near-zero timing error is a property of the idealized attack. It says
  nothing yet about C-005 on a real guitar.

## Spike 2 — verifier (`--example spike_verifier [-- rates|letring|sweep|scale|scaleseeds]`)

Defaults from the plan (θ = θ_k = 0.2, confirm 3 hops, G_attack 5 ms, W =
clamp(2τ0, 512, 1024)) were **not tuned**; they worked as specified.

- Correct notes, MIDI 40–79 × 3 seeds (120 per variant): 120/120 Present for
  clean, quiet (amp 0.1), H1 −20 dB, H1 −30 dB, +25 c/50 ms glide, +40 c/80 ms
  glide, fast decay. Worst sustain error: 0.19 c clean, 2.06 c (+25 c glide),
  8.05 c (+40 c/80 ms glide — near the 10-cent limit).
- Wrong notes (12 intervals from −12 to +24 semitones, ~130 trials): 0 false
  Present *after* widening f_max (see DEVIATIONS in HANDOFF.md). With the plan's
  f_max = E5·2^(1/12), octave-up errors above E5 (e.g. expected A4, played A5)
  were falsely accepted 8 times in 23 trials.
- Decision latency (verifier + onset reporting, 128-frame blocks): worst 2163
  frames = 49.0 ms for E2–G#3 (limit 55), 1297 frames = 29.4 ms for A3–E5
  (limit 35).

### R-108 let-ring (this is the weak spot)

Present rate for one previous note ringing `x` dB below the new pluck (decay
8 dB/s, previous pluck 450 ms earlier; rows = interval of the previous note):

```
interval / ring dB:     -1     -3     -4     -5     -6     -8    -10    -14
prev -12 st          2/4    4/4    4/4    4/4    4/4    4/4    4/4    4/4
prev  -7 st          4/5    4/5    5/5    5/5    5/5    5/5    5/5    5/5
prev  -5 st          4/6    6/6    6/6    6/6    6/6    6/6    6/6    6/6
prev  -2 .. +2 st    6/6 in every column
prev  +5 st          5/6    6/6    ...
prev  +7 st          4/5    5/5    ...
prev +12 st          4/4    4/4    ...
```

A single ringing note is tolerated down to about −3 dB. Several ringing notes
add up. C-major scale C3–C5 (15 notes), 30 excitation seeds each, decay =
`guitar_decay_db_per_s` (12 dB/s at E2 rising to 34 dB/s at E5 — an
**assumption**, no source):

| Note spacing | Scales with every note Present | Notes rejected |
|---|---|---|
| 0.60 s | 30/30 | 0/450 |
| 0.45 s | 30/30 | 0/450 |
| 0.30 s | 23/30 | 7/450 (1.6 %) |

With a fixed slow 8 dB/s decay and 0.30 s spacing, 7 of 15 notes were rejected;
at 0.45 s a *single seed* rejected one note (a = 0.32). So the outcome is
sensitive to string decay rate and to excitation phase. T-034 (−6 dB) passes,
so D-012 is not triggered by its own rule, but I would not rely on that: the
G-001 real-guitar let-ring test decides. The example song therefore keeps
≥ 0.33 s between notes.

## S-002/S-003 clock findings

- **T-024:** worst render-clock error under ±2 ms uniform callback jitter over
  20 seeds, fitting the last *N* callbacks: N=32 → 1.738 ms, 64 → 1.117 ms,
  **128 → 0.776 ms**, 256 → 0.524 ms. The plan's 32 cannot meet "≤ 1 ms";
  `gtcore::drift::RENDER_FIT_WINDOW` = 128.
- **T-031:** 44102.2 Hz input vs 44100 Hz output (50 ppm), 300 s, ±1 ms jitter:
  worst corrected timing error 0.058 ms; the uncorrected mapping reaches
  14.9 ms. Warning raised within 1 s of data. No warning for two devices on the
  same clock.

## Spike 1 — cubeb duplex

Partial; see `research/spikes/spike-cubeb-duplex/RESULTS.md`. In short: on a
Windows laptop (WASAPI shared) C-011 held (0 mismatched callbacks in ~1180),
callbacks were 441 frames not 256, and two real backend issues were found (COM
must be initialised; registering a device-changed callback fails with
`NotSupported`). **No valid round-trip latency was obtained**: the laptop
microphone returned digital silence and no reference interface / loopback
cable was available.

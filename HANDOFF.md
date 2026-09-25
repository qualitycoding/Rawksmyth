# HANDOFF

## Purpose

Build a Rocksmith-style guitar learning game (plan/PLAN.md, Revision 2). This document is the entry point for anyone — human or agent — picking this up.

## Reading order

1. This file.
2. `research/spikes/RESULTS.md` — measured numbers, and what they do and do not show.
3. `plan/PLAN.md` §2B.1b–e (verifier, latency budget, clock model, threads) — needed to understand the code.
4. `plan/ASSUMPTIONS.md` — confirmed defaults A-001–A-014.
5. `plan/PLAN.md` in full (DECISIONS, GATES, ENVIRONMENT sections are inline, not yet split into separate files).

## Status: substantial hardware-free implementation, still NOT READY

Phase 1 (research rounds, real-hardware spikes) was never run. **Everything that passes here passes on synthetic Karplus–Strong fixtures**, which are cleaner than a real guitar. Real-guitar behaviour (R-101, R-108, C-005), real driver latency (spike 1, T-015, T-019), and the Godot bridge (spike 4, T-030) are unverified. Gates G-001 and G-002 have not happened.

### What exists and is tested (`cargo test --workspace`, ~90 tests, no hardware)

| Step | Where | Tests |
|---|---|---|
| S-001 workspace | `gtcore`, `gtaudio`, `gtapp`, `tests/fixtures/gen` (`gtfixtures`) | — |
| S-003 clock (pure math) | `gtcore/src/clock.rs` (frozen), `gtcore/src/drift.rs` | T-023, T-024, T-031 |
| S-004 onset detector | `gtcore/src/dsp/onset.rs` | T-004, T-005 |
| S-005 note verifier | `gtcore/src/dsp/verifier.rs` | T-001–T-003, T-017, T-018, T-025–T-027, T-032–T-034 |
| S-006 evidence pipeline | `gtcore/src/pipeline.rs` | T-006, T-020 |
| S-007 chart parser | `gtcore/src/chart.rs` (frozen) | T-012, T-013, T-029 |
| S-008 example song | `assets/songs/example/` (generator: `gtapp/examples/gen_example_song.rs`) | S-008 load test |
| S-009 scoring | `gtcore/src/scoring.rs` | T-008, T-009, T-028 |
| S-011 highway (layout half) | `gtcore/src/highway.rs` | T-007 |
| S-012 session logger | `gtcore/src/session.rs` | atomic write, D-013 statistic |
| S-002 audio engine (hardware-free half) | `gtaudio`: WAV, SPSC ring, seqlock, RT callback core, file backend | T-011 (CI), T-021 (CI) |
| S-013 headless core | `gtapp`: `Song`, `Engine`, `run_session`, `gt-headless` | T-010, T-011 (app level), SC-4, D-013 |

Try it: `cargo run --release -p gtapp --bin gt-headless -- assets/songs/example --log session.json`.

### What is deliberately NOT implemented

| Component | Step | Why |
|---|---|---|
| cubeb duplex backend | S-002 | Spike 1 tool built and partly run (C-011 held on WASAPI shared). Backend still unwritten: needs COM init, must not register `device_changed_cb` on WASAPI, stereo-frame duplex, and L_rt verification on a real interface. Implements `gtaudio::backend::AudioBackend`. |
| `gtbridge` (gdext) and `godot-project/` | S-010, S-011 | No Godot/display here. `gtcore::highway` already produces the `NotePose`s the scene should draw; `gtcore::drift::RENDER_FIT_WINDOW` is the render-clock fit size to use. |
| Live three-thread wiring, settings UI, macOS mic plist | S-013 | Need hardware/UI. `gtapp::run_session` shows the intended single-thread-equivalent flow. |
| Performance hardening | S-014 | Needs hardware (T-015, T-016, T-019, T-021 HW part). |
| G-001 evidence, real-guitar validation | — | Needs a guitar and interface. |
| Spike 1 proper (L_rt measured vs reported) | — | Tool exists (`research/spikes/spike-cubeb-duplex`, standalone); run only partially on a laptop with a silent mic. Needs the reference interface + loopback cable. See its RESULTS.md. |
| D-012 harmonic-comb verifier | — | Not triggered by the plan's rule (T-034 passes); see R-108 below. |

### Deviations from the plan (all measured, all in code comments)

1. **Verifier sub-lag range** (`VerifierConfig::f_max_hz`): E6·2^(1/12), not E5·2^(1/12) (§2B.1b). With E5, an octave-up error above E5 (expected A4, played A5) has no sub-lag to test and was falsely accepted (8 of 23 trials). Latency unaffected.
2. **Render-clock fit window = 128 callbacks, not 32** (§2B.1d). Under ±2 ms jitter the 32-callback fit's worst error is 1.74 ms, so it cannot meet T-024's ≤ 1 ms; 128 gives 0.78 ms. Same class of finding as RV-1: a plan number that did not survive checking.
3. **Onset detector**: SuperFlux max-filter (±1 bin) on the previous frame, first-difference envelope backtracking, γ = 1000, threshold `4 + 2·mean`. The plain flux in §1.4 fails on ringing low notes (RESULTS.md).
4. **Fixtures are generated, not committed** (§2B.2 allowed either); location `tests/fixtures/gen` is a workspace crate `gtfixtures` with a seeded RNG, no external crates.
5. **`decided_at`** in `NoteEvidence` is the verifier's window end; `Pipeline` raises it to `max(that, frame the onset was reported)` so `L_onset` is included in T-020.
6. **No `rtrb` / `assert_no_alloc`**: the SPSC ring is a small safe-Rust atomic implementation; T-021 uses a counting global allocator. C-015 is therefore moot, not verified.
7. **`ChartNote` window** is ±50 ms around the expected onset (`ExpectedNote::[open, close)`, D-008).
8. `clock.rs` and `chart.rs` are untouched; new clock code is in `drift.rs`.

### Known limitations and risks (read before trusting a result)

- **R-108 let-ring is the weak spot.** A single ringing note is tolerated to about −3 dB; several ringing notes add up. With an assumed guitar-like decay, a C-major scale passes at 0.45 s spacing (0/450 rejected) but rejects 1.6 % at 0.30 s; with a slow fixed 8 dB/s decay it rejects far more. T-034 passes, so D-012 is *not* triggered by its own rule, but the G-001 real-guitar recording decides. The example song keeps ≥ 0.33 s between notes.
- **R-104**: D-013's offset assist learns only from hit notes inside the ±50 ms window. A latency misreport bigger than the window gives all Misses and no calibration data (pinned by `known_limitation_latency_error_beyond_the_window_gives_no_offset_assist_data`). A wide-window calibration mode would fix it.
- `guitar_decay_db_per_s` (fixture decay by pitch) is my assumption, not a measurement.
- The plan's "note 8 dB/s decay" default in `gtfixtures::Pluck` is arbitrary and on the slow side.
- Attribution inside the verifier is greedy per call; a better-matching onset arriving later cannot displace an earlier assignment.
- Legato notes are unsupported (A-014); the parser rejects them.
- Tests T-004…T-034 added in this pass were written *alongside* the implementation and tuned against it, not frozen before it. Rule 9 protects them from now on; they did not protect against tuning. I checked that they can fail by mutating θ, θ_k, the ±50-cent gate and f_max (each fails the intended tests).
- Toolchain: local Rust is 1.83.0 (matches the plan). `clippy` is not installed here and was not run; code is not rustfmt-normalized.

### Freeze

`tests/FROZEN_MANIFEST.sha256` now also covers the fixture generator, all test files, `tests/common`, and the example song assets. Verify with `sha256sum -c tests/FROZEN_MANIFEST.sha256`. Existing hashes for `chart.rs`, `clock.rs`, `.gitattributes` are unchanged.

### How to run

```
cargo test --workspace                          # everything, no hardware
sha256sum -c tests/FROZEN_MANIFEST.sha256        # freeze verification
cargo run --release -p gtcore --example spike_verifier -- rates   # spikes (see RESULTS.md)
cargo run --release -p gtapp --example gen_example_song -- assets/songs/example
```

### Suggested next steps

1. Hardware: `gtaudio` cubeb backend (spike 1), then T-015/T-019.
2. Real-guitar recording of the example song and scale passages through `gt-headless`-style playback of a WAV (extend `run_session` input from a file) → G-001 evidence; decide D-012.
3. Godot bridge (S-010/S-011) using `gtcore::highway` and `gtapp::Engine`; T-030.
4. Wide-window calibration mode for R-104.
5. Plan review points 7 and 8 (still open), Phase 1 research, cold-read gate.

## Halt protocol

If any frozen test fails, halt and write `BLOCKED.md` describing which test, the failure, and whether it indicates a bug or a needed plan revision.

## Integrity Rule (Rule 9)

Implementing agents cannot modify, skip, or weaken frozen tests. Restated verbatim from protocol §Rule 9 as required by plan §3.5.

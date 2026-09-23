# Execution Plan: Rocksmith Clone (planning-protocol-v3.1)

## 1. Overview & Success Criteria
- **Goal:** Build a native, real-time Rocksmith-style guitar learning game that listens to a real electric guitar through an audio interface, detects pitch and onsets with low latency, renders a 3D note highway at $\ge 60\text{ FPS}$, and scores note accuracy and timing.
- **Success Criteria:**
  - **SC-1:** Monophonic pitch detection of guitar notes $E_2\text{--}E_5$ ($82.4\text{--}659.3\text{ Hz}$) with $\le 50\text{ ms}$ algorithmic latency and $\le 10\text{ cents}$ error on sustained notes, with zero low-octave errors on $E_2$.
  - **SC-2:** Onset detection within $20\text{ ms}$ of ground truth on reference electric guitar recordings.
  - **SC-3:** 3D note highway renders and scrolls in sample-accurate synchronization with audio at $\ge 60\text{ FPS}$ at $1080\text{p}$.
  - **SC-4:** Scoring engine produces accurate per-note judgments (Perfect, Great, Good, Miss) according to perceptual windows.
  - **SC-5:** Full end-to-end playable session on an example song with session log export.
  - **SC-6:** Deterministic test suite passes in headless CI with zero network access.

---

## 2. Step Breakdown

| Step | Title | Assigned Tier | Dependencies | Key Outputs | Done When |
|---|---|---|---|---|---|
| **S-001** | Workspace Bootstrap & Domain Types | Sonnet | None | Rust workspace, domain structs (`AudioFrame`, `PitchResult`, `OnsetResult`, `ResolvedNoteEvent`), mock traits | `cargo check` & `cargo test` pass stubs |
| **S-002** | Duplex Audio I/O Layer | Opus | S-001 | `src/audio/` with `cubeb` duplex stream, `cpal` fallback, `MockAudioBackend`, and SPSC ring buffer (`rtrb`) | `T-011` passes on mock backend; loopback streams without drops |
| **S-003** | Multi-Rate Pitch Detector (Hybrid MPM/YIN) | Opus | S-001 | `src/dsp/pitch.rs` with 4× IIR decimation filter for sub-200 Hz register and McLeod Pitch Method | `T-001`, `T-002`, `T-003`, `T-017`, `T-018` pass |
| **S-004** | Onset Detector & Pre-Conditioning Filter | Sonnet | S-001 | `src/dsp/onset.rs` with 2nd-order 3.5 kHz low-pass filter, adaptive spectral flux, and refractory gate | `T-004`, `T-005` pass on click and silence fixtures |
| **S-005** | Temporal Note Tracker (Attack/Pitch Reconciler) | Opus | S-003, S-004 | `src/dsp/temporal_tracker.rs` with 15–35 ms consensus window and confidence voting | `T-006` passes on reference FLAC DI guitar recording |
| **S-006** | Chart Format & Parser | Sonnet | S-001 | `src/chart/` with JSON parser, schema validation, and path traversal guards | `T-012`, `T-013`, `T-020` pass |
| **S-007** | Godot 4.3 GDExtension Slave Renderer | Opus | S-001 | `godot/` 3D highway scene, `rocksmith-gdext` crate consuming SPSC visual queue | `T-007` passes in headless Godot runner |
| **S-008** | Duplex Audio-Clock Synchronization | Opus | S-002, S-007 | `src/sync/` mapping hardware output sample counter directly to highway scroll position with dual offset sliders | `T-016` renders $\ge 60\text{ FPS}$ with zero visual stutter |
| **S-009** | Scoring & Judgment Engine | Sonnet | S-005, S-008 | `src/scoring/` implementing Rocksmith timing windows (±15ms, ±30ms, ±50ms) and continuous streak scoring | `T-008`, `T-009`, `T-022` pass |
| **S-010** | Session Logger & Statistics | Haiku | S-009 | `src/session/` writing atomic JSON reports with per-note timings and pitch deviations | Session JSON valid and verified against schema |
| **S-011** | Reference Song & Audio Fixture Generation | Sonnet | S-006 | `assets/songs/example/` with chart JSON, backing track, and ground-truth DI track | Song loads and parses cleanly |
| **S-012** | End-to-End Application Integration | Opus | S-009, S-010, S-011 | `src/app/` coordinating startup, song selection, playback, scoring, and teardown | `T-010` passes end-to-end playthrough |
| **S-013** | Performance Hardening & Latency Tuning | Opus | S-012 | Zero-allocation audio callbacks, SIMD vectorization in FFT/decimation | `T-015a` passes ($\le 2\text{ ms}$ processing per hop) |
| **S-014** | CI & Release Freeze Verification | Sonnet | S-013 | `.github/workflows/ci.yml`, Cargo manifest checks, security audit | `T-014` (`cargo audit`) clean; all frozen tests pass |

---

## 3. Human Gate Plan

### Gate G-001: DSP & Latency Validation
- **Trigger:** Immediately after S-005 (Audio I/O + Multi-Rate Pitch + Temporal Tracker).
- **Evidence Bundle:**
  - Synthetic sine/saw accuracy report across $E_2\text{--}E_5$.
  - Low-$E_2$ sub-octave error verification report.
  - Throughput benchmark results (`T-015a` processing time per 2048-sample hop).
  - Manual hardware roundtrip latency report (`T-015b`) on physical machine.
- **Questions for Human:**
  1. Does detected pitch accuracy on low-$E_2$ satisfy playability standards ($\le 10\text{ cents}$)?
  2. Is observed processing latency acceptable ($\le 50\text{ ms}$)?
- **Responses:** `proceed` / `proceed-with-rescope` / `stop`.

### Gate G-002: Gameplay & Note Highway Evaluation
- **Trigger:** Immediately after S-012 (End-to-End Playthrough).
- **Evidence Bundle:**
  - 60 FPS frame-timing trace during example song playback.
  - Session scoring log demonstrating Perfect/Great/Good/Miss classification.
  - Audio and visual calibration offset responsiveness.
- **Questions for Human:**
  1. Is the note highway feel and visual synchronization smooth and responsive?
  2. Are scoring windows fair on physical guitar play?
- **Responses:** `proceed` / `proceed-with-rescope` / `stop`.

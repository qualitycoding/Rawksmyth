# Rocksmith Clone — Execution Plan (planning-protocol-v3.1) — Revision 2

## Revision note

This revision addresses review points 1–6 and reruns the Phase 4 pre-mortem with chart-informed detection as the baseline. Review points 7 (hardware-only tests vs. SC-6 "deterministic CI") and 8 (research claims asserted rather than executed; citation and licence checks) are **not** resolved here. The status header in Phase 5 reflects that.

| ID | Review point | Change |
|---|---|---|
| RV-1 | Latency arithmetic | C-003 corrected: analysis latency is bounded by the full buffer length N, not the hop. The 4096/2048 YINFFT default is withdrawn. Per-note decision latency is derived in §2B.1b and the full budget is decomposed in §2B.1c. |
| RV-2 | Low-E handling | R-002's premise corrected: 4096 frames ≈ 7.7 periods of E2 (2048 ≈ 3.8), and ~2 periods suffice. Dual-resolution D-006 retired. Octave/harmonic confusion is handled by harmonic sub-lag rejection in the verifier (§2B.1b). |
| RV-3 | Chart-informed detection | Blind pitch estimation is removed from the scoring path. A `NoteVerifier` tests the charted pitch. `Scorer` now receives the `ChartNote`. Blind detection (tuner mode) is deferred. |
| RV-4 | SC-1 split | SC-1a = pitch accuracy; SC-1b = decision latency (deterministic, in frames). SC-2 tightened to timestamp accuracy. Scoring uses input-stream frame indices, so device latency affects feedback delay but not judgment accuracy once L_rt is calibrated. |
| RV-5 | Rust–Godot seam | Engine-agnostic Rust core + godot-rust (gdext) bridge. One cubeb duplex stream owns guitar input and backing-track output; Godot runs with its Dummy audio driver. Explicit clock model (§2B.1d) and thread model (§2B.1e). |
| RV-6 | Traceability | Rebuilt against the new step list (§2E). R-005 now has a test (T-029). Stale "T-020 = parser depth" reference removed. |

---

## Phase 0: Environment Diagnostic, Intake & Ambiguity Resolution

### 0.1 Environment Diagnostic [All]

| Item | Status | Detail |
|---|---|---|
| Subagent spawning | Assumed available | Model tier hierarchy per protocol §0.2 |
| GitHub access | Assumed — plan halts if absent | Target repo TBD at intake; `gh auth status` verified by implementer |
| Sandbox capability | Assumed available | Code execution, package install, network needed for Phase 1 spikes |
| Software tooling | Pinned in 1.4 after R4 | Rust 1.83+; cubeb-rs; godot-rust (gdext); Godot 4.3; rtrb; assert_no_alloc |
| Audio hardware | Required for G-001 and hardware-tier tests | Reference interface with loopback cable; electric guitar |
| Publication tooling | N/A | publication profile inactive |
| Implementer credentials | Not required by planner | No deployment keys; local-only build |

### 0.2 Model Tier Hierarchy [All]

Standard protocol hierarchy. Software-profile-only plan with `software.deploys = false`.

### 0.3 Intake & Definition of Ready

#### 0.3.1 Common Intake [All]

| Field | Value |
|---|---|
| Goal | Build a Rocksmith-style guitar learning game that listens to a real electric guitar through an audio interface, verifies charted notes in real time, renders a 3D note highway, and scores the player's performance — executable and verifiable without human questions (except at gates). |
| In Scope | Real-time audio capture → onset detection → chart-informed note verification → timestamp-based scoring → note highway rendering → session logging. Desktop (Windows/macOS/Linux). One example song. |
| Out of Scope | CDLC import; bass/drums/vocals; multiplayer; amp modeling; mobile/web; distribution. **New:** blind pitch detection / tuner mode; chords; string identification; legato techniques (hammer-on/pull-off/slide) in charts. |
| Success Criteria | **SC-1a (pitch accuracy):** for a charted note E2–E5 that is played, the verifier's cents error on the sustained portion (onset+60 ms to onset+160 ms) is within ±10 cents of ground truth on reference fixtures. **SC-1b (decision latency):** worst-case time from true onset to the verifier's decision, measured in frames on reference fixtures, is ≤55 ms for charted notes E2–G#3 and ≤35 ms for A3–E5. **SC-2 (timestamp accuracy):** onset timestamps, as input-stream frame indices, are within ±10 ms of ground truth at p95 (median ≤5 ms). **SC-3:** the highway renders at ≥60 FPS at 1080p, driven by the audio clock with ≤1 ms render-clock error under callback jitter. **SC-4:** per-note judgment (Perfect/Great/Good/Miss) follows the timing windows, computed from timestamps, independent of device latency after calibration. **SC-5:** a session can be played end-to-end on the committed example song. **SC-6:** all CI-tier tests pass deterministically with no network. |
| Informational target | Feedback latency (pluck → judgment visible) ≤100 ms on reference hardware; reported at G-001, not pass/fail. Does not affect scoring accuracy. |
| Constraints | One Rust-owned cubeb duplex stream for input, backing track and SFX; Godot audio disabled (Dummy driver). Sample rate 44.1 kHz; input buffer request 256 frames. Verification by chart-informed constrained NSDF (§2B.1b). Rendering Godot 4.3 via gdext; core crates have no engine dependency. No cloud dependencies. |
| Proposed Profiles | software (primary). computational, math, publication not selected (unchanged rationale). |
| Mode Flags | `software.deploys = false`; `math.exploration = false` (N/A). |

#### 0.3.2 Software Intake [software]

| Field | Value |
|---|---|
| Target users | Guitarists learning songs; developers extending the clone. |
| Supported platforms | Windows 10+, macOS 12+, Ubuntu 22.04+ (PulseAudio or PipeWire-pulse). |
| Runtime environment | Native desktop. Cargo workspace: `gtcore` (DSP, chart, clock, scoring, highway layout — pure Rust, no I/O), `gtaudio` (AudioBackend: cubeb duplex + file backend), `gtbridge` (gdext cdylib), `godot-project/`. |
| Deployment target | N/A (`software.deploys = false`). |
| Performance targets | SC-1b decision latency; feedback latency ≤100 ms (informational); ≥60 FPS at 1080p; CPU ≤30% on a 4-core 3 GHz reference machine. |
| Threat model | Local-only. Risks: real-time violations in the audio callback (allocation, locks, I/O) causing glitches; malformed chart files causing panics or stack overflow; unbounded queues. No network attack surface. |
| Data handling | Session logs stored locally as JSON; no PII. |
| Release channel | N/A for first milestone. |
| Maintenance | Implementer documents extension points (verifier swap, renderer swap); no SLA. |

#### 0.3.3–0.3.5 Mathematical / Computational / Publication Intake

N/A. Profiles inactive.

#### 0.3.6 Ambiguity Resolution [All]

| ID | Ambiguity | Proposed Default | Consequence |
|---|---|---|---|
| A-001 | Rendering engine | Godot 4.3 via gdext, with engine-agnostic core crates | Fast 3D highway; if the bridge fails, D-003 swaps only the renderer. |
| A-002 | Audio library | cubeb (cubeb-rs), single duplex stream | One device clock for input and output (C-011). cpal fallback per D-001. |
| A-003 | Detection approach (**revised**) | Chart-informed constrained NSDF verifier on the scoring path; no blind estimator in milestone 1 | Short windows, bounded latency, octave/harmonic confusion handled by construction. Tuner mode deferred. |
| A-004 | Polyphony | Monophonic only | Unchanged. Let-ring mixtures handled by D-012 fork. |
| A-005 | Example song | Original procedural composition | Unchanged. |
| A-006 | Guitar input | Dry signal only | Unchanged. |
| A-007 | Scoring model | Rocksmith-style: pitch gate + timing grade | Grade from onset timing; pitch must pass the identity band. |
| A-008 | Latency calibration (**amended**) | Manual audio offset δ_a and video offset δ_v, plus offset assist (D-013) | Two offsets are standard in rhythm games; assist addresses misreported driver latency (R-104). |
| A-009 | Repository | `qualitycoding/Rawksmyth` (new, empty) | Confirmed by human. |
| A-010 | License | AGPL-3.0 | Unchanged here (licence review is open review point 8). The Essentia port and aubio dependency are removed in this revision. |
| A-011 (**new**) | Input/output device | Same device for input and output | Shared frame clock, no drift. Distinct devices supported with a warning via D-010. |
| A-012 (**new**) | Pitch identity band | ±50 cents, configurable | A slightly out-of-tune guitar still scores; cents error is logged separately. |
| A-013 (**new**) | String identity | Not verified — any string producing the charted pitch counts | Timbre-based string ID is out of scope. |
| A-014 (**new**) | Legato notes | Excluded from milestone-1 charts | Notes without a pick onset are judged Miss (D-011). |

**Resolution (2026-09-23):** the human confirmed all proposed defaults A-001 through A-014 at intake. They are recorded as confirmed in `plan/ASSUMPTIONS.md`.

---

## Phase 1: Storage Isolation, Iterative Research & Checkpointing

### 1.1 Generation Branch [All]

Branch: `gen-<UTC-timestamp>-rocksmith-clone`

```
HANDOFF.md                     [All]
plan/PROFILE.md                [All]
plan/PLAN.md                   [All]
plan/ASSUMPTIONS.md            [All]
plan/DECISIONS.md              [All]
plan/GATES.md                  [All]
plan/ENVIRONMENT.md            [All]
plan/TRACEABILITY.md           [All]
plan/OPERATIONS.md             N/A (software.deploys = false)
research/QUESTIONS.md          [All]
research/claims.json           [All]
research/SOURCES.md            [All]
research/NOVELTY.md            N/A
research/rounds/round-N.md     [All]
research/spikes/               [All]  (spike list revised, §1.3 R6)
math/*                         N/A
tests/                         [All]
tests/fixtures/gen/            [All]  (fixture generator + seeds, §2B.2)
tests/FROZEN_MANIFEST.sha256   [All]
figures/SPEC.md                N/A
manuscript/*                   N/A
premortem/round-N.md           [All]
premortem/RISK_REGISTER.md     [All]
.checkpoints/state.json        [All]
```

### 1.2 Checkpointing [All]

Schema per protocol. `profiles: ["software"]`, `mode_flags: {"software.deploys": false}`; `not_applicable` lists all math, computational, publication sections.

### 1.3 Iterative Research Loop [All]

**R1 — Decompose (Opus).** Question tree:

- *Audio engine:* Does a cubeb duplex callback give input and output equal frame counts (shared frame index)? How accurate are cubeb's reported input/output latencies vs. loopback measurement on WASAPI shared/exclusive, CoreAudio, PulseAudio/PipeWire? What happens with distinct input and output devices?
- *DSP:* Does a normalized difference function evaluated only at candidate lags reach ±10 cents on plucked strings E2–E5 with W = clamp(2τ0, 512, 1024)? How robust is harmonic sub-lag rejection to weak fundamentals, attack glide, and let-ring mixtures? What onset localization does 512/128 spectral flux with envelope backtracking achieve?
- *Integration:* Can gdext on Godot 4.3 host an extension that owns OS threads, with Godot's audio driver set to Dummy? What render-clock jitter results from fitting callback timestamps?
- *Game logic:* Are timing windows ±15/±30/±50 ms appropriate? How should onsets be attributed when same-pitch notes are close together?

**R2 — Breadth Pass (Haiku/Sonnet, parallel).**

| C-### | Claim | Kind | Load-Bearing | Expected Evidence |
|---|---|---|---|---|
| C-001 | cubeb supports full-duplex streams on WASAPI, CoreAudio (AudioUnit) and PulseAudio, usable from cubeb-rs. | software | Yes | Tier 1: cubeb repo/docs; spike 1 |
| C-002 (**replaced**) | A normalized difference function (McLeod & Wyvill 2005 NSDF normalization) evaluated only at candidate lags gives ≤10-cent accuracy on sustained plucked-string tones E2–E5 with W = clamp(2τ0, 512, 1024). | numerical | Yes | Tier 1: McLeod & Wyvill 2005 (method); spike 2 (accuracy) |
| C-003 (**corrected**) | Analysis latency is bounded below by the buffer length N(f0)/fs, not the hop. N(E2) = 1591 frames = 36.1 ms; worst-case decision latency follows the §2B.1b formula (49.8 ms at E2, 26.9 ms at E5). | numerical | Yes | Derivation (§2B.1b); must be verified by spike 2 |
| C-004 (**demoted**) | Godot AudioServer timing APIs are accurate. | software | No — Godot audio is off the timing path | Tier 1: Godot docs |
| C-005 (**revised**) | Spectral-flux onset detection (512-frame, 128-hop) with envelope backtracking localizes plucked-string onsets to median ≤5 ms, p95 ≤10 ms. | numerical | Yes | Tier 1 source to be identified in R4 (the Rev 1 citation is unverified); spike 3 |
| C-006 | *Retired* — no YINFFT port. | — | — | — |
| C-007 | Open-source Rocksmith alternatives are viable. | software | No | Tier 3 |
| C-008 | *Retired* — aubio no longer used. | — | — | — |
| C-009 | Manual offset calibration is standard in rhythm games. | software | No | Tier 2 |
| C-010 | fee[dB]ack uses swappable note detection. | software | No | Tier 3 |
| C-011 (**new**) | In a cubeb duplex callback, input and output buffers have equal frame counts, so one running frame index serves both; cubeb exposes output and input latency queries. | software | Yes | Tier 1: `cubeb.h` docs; spike 1 |
| C-012 (**new**) | Godot 4.3 can run with its Dummy audio driver, leaving the audio device to the Rust engine. | software | Yes | Tier 1: Godot docs; spike 4 |
| C-013 (**new**) | godot-rust (gdext) supports the Godot 4.3 API, and a GDExtension may own background OS threads that never call Godot APIs. | software | Yes | Tier 1: gdext docs; spike 4 |
| C-014 (**new**) | cubeb's behaviour when input and output devices differ in one duplex stream (resampling, drift handling). | software | Yes (D-010) | Tier 1: cubeb source; spike 1 |
| C-015 (**new**) | rtrb provides a wait-free SPSC ring buffer and assert_no_alloc detects allocation within a scope; both are usable in the audio callback. | software | Yes | Tier 1: crate docs |

**R2b — Prior-art check.** Unchanged in scope (C-007, C-010). Not load-bearing.

**R3 — Synthesis (Opus).**

- C-003 is a derivation; it must end *verified* by spike 2 measuring decided_at − true onset in frames.
- The verifier's octave/harmonic handling is a design claim covered by C-002 plus fixtures T-025, T-026, T-032; real-guitar behaviour is checked only at G-001.
- Godot's audio subsystem is no longer on the timing path; the Rev 1 "Godot audio latency" question is replaced by C-012/C-013.

**R4 — Depth Pass (Sonnet/Opus).** Verify from Tier 1 sources: cubeb duplex semantics and latency API (C-001, C-011, C-014); NSDF definition and normalization (C-002); Godot Dummy audio driver (C-012); gdext Godot 4.3 support and threading rules (C-013); rtrb/assert_no_alloc (C-015); a Tier 1 onset-localization source (C-005).

**R5 — Adversarial Pass (Opus, fresh context).** Search for: drivers that misreport latency; WASAPI exclusive-mode failures on Windows 11; cubeb duplex with distinct devices; gdext thread-safety pitfalls; difference-function behaviour on mixtures (let-ring) and weak fundamentals; macOS microphone permission requirements (NSMicrophoneUsageDescription); Linux headers (libpulse-dev).

**R6 — Empirical Verification (Sonnet).** Spikes in `research/spikes/`:

1. `spike-cubeb-duplex/` — open a duplex stream on the reference interface; confirm equal input/output frame counts (C-011); loopback-measure L_rt vs. cubeb-reported latencies; record callback sizes and timing jitter; repeat with distinct devices (C-014).
2. `spike-verifier/` — constrained-NSDF verifier on generated fixtures (clean, weak-fundamental, attack-glide, harmonic confusions, let-ring). Report cents accuracy, false accept/reject rates, and decision latency in frames. Calibrates initial θ, θ_k.
3. `spike-onset/` — 512/128 spectral flux + envelope backtracking on a 50-pluck fixture set; report error distribution.
4. `spike-gdext-bridge/` — Godot 4.3 project + gdext extension; Dummy audio driver; a Rust thread publishes a synthetic clock; sample render song time each frame; report monotonicity and jitter.

**Termination criteria.** Minimum 3 rounds, maximum 6; stop on saturation. Every load-bearing claim ends *corroborated* or *verified*. C-003 must be verified by spike 2; C-011 by spike 1; C-012 and C-013 by spike 4. Unresolved claims go to `premortem/RISK_REGISTER.md`.

### 1.4 Environment Pinning [All]

`plan/ENVIRONMENT.md` pins:

```
Rust 1.83.0
cubeb-rs            version pinned after C-001/C-011 verified
godot-rust (gdext)  version pinned after C-013 verified; targets Godot 4.3 API
Godot 4.3-stable
rtrb, assert_no_alloc   versions pinned after C-015 verified
serde, serde_json   pinned (chart parsing)
wgpu / Bevy         only if D-003 triggers

audio:   duplex, same device (A-011); input buffer request 256 frames; fs = 44100 Hz
onset:   frame 512, hop 128, log-magnitude spectral flux, envelope backtracking
verifier: hop 128; W = clamp(2·τ0, 512, 1024); refine W_r = 2048;
          G_attack = 5 ms; confirm = 3 consecutive hops;
          θ = θ_k = 0.2 initial (calibrated by spike 2, recorded in DECISIONS.md)

Removed from Rev 1: aubio, YINFFT port, OMP_NUM_THREADS
```

All setup commands executed and verified in sandbox.

---

## Phase 2: Specification & Freeze

### 2A Mathematical Specification [math]

N/A.

### 2B Software & Numerical Tests [software]

#### 2B.1a Interfaces First [software]

Defined in `plan/DECISIONS.md`; stubs committed with `unimplemented!()`. All timestamps are frames, not milliseconds.

```rust
/// Running frame index since the duplex stream started.
/// Input and output share this index (C-011, A-011).
pub type Frame = u64;

pub struct ChartNote {
    pub id: u32,
    pub start_s: f64,        // song time
    pub sustain_s: f64,
    pub string: u8,          // display only; not verified (A-013)
    pub fret: u8,
    pub target_hz: f32,      // derived from tuning + string + fret
    pub legato: bool,        // must be false in milestone-1 charts (A-014)
}

/// A chart note mapped into input frames by the ClockModel.
pub struct ExpectedNote { pub note: ChartNote, pub open: Frame, pub close: Frame }

pub struct Onset { pub frame: Frame, pub strength: f32 }

pub enum Presence { Present, Absent, HarmonicUp(u8) /* k-th harmonic of target */ }
pub enum EvidenceKind { Decision, Refinement }

pub struct NoteEvidence {
    pub note_id: u32,
    pub kind: EvidenceKind,
    pub presence: Presence,
    pub onset: Option<Frame>,
    pub decided_at: Frame,        // SC-1b is measured from this
    pub cents_error: Option<f32>, // Refinement carries the SC-1a value
    pub aperiodicity: f32,        // a(τ*), lower = more periodic
}

pub trait OnsetDetector {
    fn process(&mut self, block: &[f32], first: Frame, out: &mut Vec<Onset>);
}

pub trait NoteVerifier {
    fn process(&mut self, block: &[f32], first: Frame,
               expected: &[ExpectedNote], onsets: &[Onset],
               out: &mut Vec<NoteEvidence>);
}

pub struct JudgedNote {
    pub note_id: u32, pub judgment: Judgment,
    pub timing_error_ms: Option<f32>, pub cents_error: Option<f32>,
}
pub enum Judgment { Perfect, Great, Good, Miss }

pub trait Scorer {
    fn on_evidence(&mut self, ev: &NoteEvidence, chart: &ChartNote,
                   clock: &ClockModel) -> Option<JudgedNote>;
    /// Miss for every note whose window closed without Present evidence.
    fn expire(&mut self, now: Frame, clock: &ClockModel) -> Vec<JudgedNote>;
}

/// Pure layout in gtcore; the Godot scene only draws NotePose values.
pub trait HighwayLayout {
    fn visible_notes(&self, song_time_s: f64, out: &mut Vec<NotePose>);
}

/// cubeb duplex backend, or file backend (deterministic, faster than real time).
pub trait AudioBackend { /* start, stop, latencies, frame counters */ }
```

Output parameters (`out: &mut Vec<_>`) let the DSP thread reuse buffers without per-hop allocation.

#### 2B.1b Note Verifier Specification (chart-informed constrained NSDF)

For each `ExpectedNote` whose window overlaps the current analysis buffer:

1. **Target lag.** τ0 = fs / target_hz. Search band B(τ) = [τ·2^(−1/12), τ·2^(1/12)] (±100 cents, so a note 60 cents off still shows a clear minimum that is then rejected by the ±50-cent gate).
2. **Normalized difference** at each candidate lag, over integration window W = clamp(2τ0, 512, 1024):
   `d(τ) = Σ_{j=0}^{W−1} (x_j − x_{j+τ})²`, `m(τ) = Σ_{j=0}^{W−1} (x_j² + x_{j+τ}²)`, `a(τ) = d(τ)/m(τ)` ∈ [0, 2].
   a = 0 is perfectly periodic, a ≈ 1 is uncorrelated. Unlike YIN's cumulative-mean normalization, this normalization is local to each lag, so only the candidate lags need evaluating (McLeod & Wyvill 2005). Cost at E2: ~62 lags × 1024 ≈ 63k MAC per hop.
3. **Estimate.** τ* = argmin a over B(τ0), refined by parabolic interpolation; f̂ = fs/τ*; cents c = 1200·log2(f̂/target_hz).
4. **Harmonic sub-lag rejection.** A note at k× the target frequency is also periodic at τ0, so a periodic minimum at τ0 alone cannot distinguish E2 from E3 or B3. For each k = 2…K, K = ⌊f_max/target_hz⌋ with f_max = E5·2^(1/12), compute a_k = min a over B(τ0/k). If a_k ≤ θ_k, the signal is periodic at the shorter lag → `HarmonicUp(k)`. A lower note played instead (e.g. E2 when E3 is expected) is not periodic at τ0 while its odd harmonics carry energy, so step 5's threshold rejects it. The residual ambiguity (a lower note with essentially no odd-harmonic energy) is physically indistinguishable from the higher note and is accepted as a limitation.
5. **Decision.** `Present` iff a(τ*) ≤ θ, |c| ≤ 50 (A-012), and no `HarmonicUp`, on 3 consecutive hops whose windows start ≥ G_attack = 5 ms after the associated onset. Evidence is emitted at `decided_at`.
6. **Refinement (SC-1a).** After a Present decision, evaluate with W_r = 2048 on hops whose windows start in [onset+60 ms, onset+160 ms] (clipped to sustain end); the median c is emitted as a `Refinement` event.

**Buffer and latency.** N(f0) = W + ⌈2^(1/12)·τ0⌉ (sub-lag checks use shorter lags and do not extend N). With hop H = 128:

L_verify(f0) = G_attack + N(f0)/fs + 2H/fs (confirmation) + H/fs (hop quantization)
L_onset = 512/fs + 3H/fs ≈ 20.3 ms (onset frame + peak-picking lookahead)
L_decide = max(L_verify, L_onset)

| Note | f0 (Hz) | τ0 | W | N | N/fs (ms) | L_decide (ms) | SC-1b limit |
|---|---|---|---|---|---|---|---|
| E2 | 82.41 | 535.1 | 1024 | 1591 | 36.1 | 49.8 | 55 |
| A2 | 110.00 | 400.9 | 802 | 1227 | 27.8 | 41.5 | 55 |
| D3 | 146.83 | 300.3 | 601 | 920 | 20.9 | 34.6 | 55 |
| G3 | 196.00 | 225.0 | 512 | 751 | 17.0 | 30.7 | 55 |
| A3 | 220.00 | 200.5 | 512 | 725 | 16.4 | 30.1 | 35 |
| E5 | 659.26 | 66.9 | 512 | 583 | 13.2 | 26.9 | 35 |

These are pipeline latencies in the sample domain, measured deterministically on fixtures (T-020). They determine how quickly feedback can appear; they do not determine judgment accuracy, which comes from onset timestamps (§2B.1d).

#### 2B.1c Latency Budget

| Component | Value | Affects judgment accuracy? |
|---|---|---|
| Input driver/converter latency L_in | Measured (T-015, T-019) | Only through L_rt calibration |
| Input buffer (256 frames) | 5.8 ms | No — frame index is exact |
| Onset + verifier decision | 26.9–49.8 ms (§2B.1b) | No — judgments use onset frames |
| DSP thread wake/scheduling | ≤1 hop + OS jitter (~3–5 ms) | No |
| Game-thread poll | ≤16.7 ms at 60 FPS | No |
| Display/compositor | Device-dependent; covered by δ_v | No |
| Output latency L_out | Measured | Only through L_rt calibration |
| **Round-trip calibration error** | Systematic bias | **Yes — the only latency that affects scoring** |
| Feedback total (pluck → visible) | Target ≤100 ms, informational | No |

#### 2B.1d Clock Model

- One cubeb duplex stream on one device (A-011). Callback k receives input frames and fills the same number of output frames; one running `Frame` counter indexes both (C-011).
- L_in and L_out are read from cubeb at stream start. L_rt = L_in + L_out + δ_a, where δ_a is the signed user audio offset.
- The backing track is fully decoded into memory at load and starts at output frame s0 (lead-in ≥ 1 s).
- Output frame j is heard L_out after its callback; input frame i was captured L_in before its callback. A pluck the player aligned with what they heard therefore has
  **song_time(i) = (i − s0)/fs − L_rt.**
- **Render clock.** Each callback publishes (monotonic_ns, frames_written) through a seqlock. The game thread fits frames = α + β·t by least squares over the last 32 callbacks and renders at
  **song_time_render = (frames(now + δ_v − L_out) − s0)/fs**,
  clamped to be monotonic. δ_v is the signed user video offset.
- **Distinct devices (D-010).** Separate counters per direction; the drift ratio comes from the two clock fits; input frames are mapped to output time through that ratio; the UI shows a warning.

#### 2B.1e Thread Model

- **RT thread (cubeb callback):** copies input into an rtrb SPSC ring; writes output from preloaded PCM plus the SFX mixer; publishes a clock sample. No allocation, locks, syscalls or logging — enforced in debug builds with assert_no_alloc (T-021).
- **DSP thread:** pulls the ring in 128-frame hops; runs onset detector, verifier, attribution and scorer; pushes `NoteEvidence`/`JudgedNote` into an SPSC event queue.
- **Game thread (Godot main, via gdext):** each frame reads the render clock, drains the event queue, draws `HighwayLayout` output. Never touches the RT thread.
- **File backend:** drives the same DSP pipeline from a WAV file faster than real time for deterministic tests (T-006, T-010, T-020, T-031).
- **Attribution:** each onset is consumed by at most one chart note, greedily by smallest |timing error| among notes whose window contains it. A Present decision without an attributed onset on a non-legato note is a Miss (D-011); this also prevents a still-ringing previous note from satisfying a repeated same-pitch note.

#### 2B.2 Provenance Contract

computational profile inactive. Lightweight provenance for fixtures: commit hash + environment hash + generator seed. Fixtures are generated by `tests/fixtures/gen/` using fractional-delay Karplus–Strong synthesis; ground-truth f0 accounts for the loop filter's phase delay, and ground-truth onset is the excitation start frame. Variants: clean, H1 attenuated 20/30 dB, +25-cent attack glide, harmonic confusions, let-ring mixtures, 50-pluck timing set, C-major scale, example song guitar part.

#### 2B.3 Generate Tests (T-###) [software]

Env: **CI** = deterministic, no hardware; **HW** = reference hardware or external tool; **Human** = human judgment at a gate. (How HW/Human tests relate to SC-6 is open review point 7.)

| T-### | Category | Req | Env | Description |
|---|---|---|---|---|
| T-001 | Unit | SC-1a | CI | Expected A4; 440 Hz sine → Present; \|cents\| ≤5. |
| T-002 | Unit | SC-1a | CI | Expected E2; KS pluck 82.41 Hz → Present; sustain \|cents\| ≤10. |
| T-003 | Unit | SC-1a | CI | Expected E5; KS pluck 659.26 Hz → Present; sustain \|cents\| ≤10. |
| T-004 | Unit | SC-2 | CI | Onset for KS pluck at frame 44100 within ±441 frames; over the 50-pluck set, median ≤5 ms, p95 ≤10 ms. |
| T-005 | Unit | SC-2 | CI | No onset on silence or on a decaying sustain. |
| T-006 | Integration | SC-5 | CI | File backend: C-major scale fixture vs. its chart → every note Present with correct note_id; timing error p95 ≤10 ms. |
| T-007 | Integration | SC-3 | CI | HighwayLayout returns correct lane/depth for given chart and song time. |
| T-008 | Integration | SC-4 | CI | Scorer boundaries: Present with onset error ≤15 ms → Perfect; ≤30 → Great; ≤50 → Good. |
| T-009 | Integration | SC-4 | CI | Miss when: onset error >50 ms; Absent; HarmonicUp; window closes with no evidence; Present without attributed onset on a non-legato note. |
| T-010 | Operational | SC-5 | CI | Headless app core on file backend loads example song, plays through, writes session log matching expected judgments. |
| T-011 | Operational | — | CI (+HW) | Injected device loss in backend → no panic, session saved, user notified. Repeated on hardware at G-002. |
| T-012 | Operational | — | CI | Malformed chart → error, no panic. |
| T-013 | Security | — | CI | Chart parser rejects path traversal in asset references. |
| T-014 | Security | SC-6 | CI | `cargo audit` reports no Critical/High. |
| T-015 | Performance | SC-1b (report) | HW | Loopback: measured L_rt vs. cubeb-reported; feedback latency reported against ≤100 ms target. |
| T-016 | Performance | SC-3 | HW | ≥60 FPS at 1080p during playback. |
| T-017 | Boundary | — | CI | Silence → no Present evidence. |
| T-018 | Boundary | — | CI | White noise → Absent for all expected notes. |
| T-019 | Performance | — | HW | Windows WASAPI shared vs. exclusive: L_rt measured and reported. |
| T-020 | Performance | SC-1b | CI | decided_at − true onset ≤55 ms (E2–G#3) and ≤35 ms (A3–E5) on the fixture set, in frames. |
| T-021 | Operational | — | CI (+HW) | RT callback allocates nothing (assert_no_alloc, debug); on hardware, no input overrun over a 10-minute run under synthetic render load. |
| T-022 | Acceptance | SC-4 | Human | Reference human playthrough: timing windows judged fair (G-002 evidence). |
| T-023 | Unit | SC-2 | CI | ClockModel: song_time(i) = (i − s0)/fs − L_rt for synthetic latencies and offsets (property test). |
| T-024 | Unit | SC-3 | CI | Render clock under ±2 ms callback jitter: monotonic, error ≤1 ms. |
| T-025 | Unit | SC-1a | CI | Expected E2, played E3 → HarmonicUp(2); expected E2, played B3 → HarmonicUp(3). |
| T-026 | Unit | SC-1a | CI | Expected E3, played E2 → Absent. |
| T-027 | Unit | SC-1a | CI | Expected A4, played A#4 → Absent; played A4 +40 cents → Present, cents +40 ±5. |
| T-028 | Integration | SC-4 | CI | Two same-pitch chart notes 80 ms apart: two plucks → each judged once; one pluck → one hit + one Miss. |
| T-029 | Security | — | CI | Chart with nesting depth >32 or size >5 MB rejected without stack overflow (R-005). |
| T-030 | Integration | SC-3 | HW/Tool | Headless Godot 4.3 loads gdext bridge with Dummy audio driver; song_time monotonic; receives a JudgedNote event. |
| T-031 | Integration | SC-4 | CI | File backend with input at 44102.2 Hz vs. output 44100 Hz (50 ppm): drift estimator keeps timing error ≤2 ms over 300 s; warning logged. |
| T-032 | Unit | SC-1a | CI | Expected E2 and A2; KS with H1 attenuated 20 dB and 30 dB → Present, not HarmonicUp (R-101). |
| T-033 | Unit | SC-1a | CI | KS with +25-cent attack glide decaying over 50 ms → correct decision; sustain \|cents\| ≤10 (R-102). |
| T-034 | Unit | SC-1a | CI | Expected A2 while a prior E2 rings at −6 dB → Present (R-108). If it fails, D-012 applies. |

#### 2B.4 Unknown-Outcome Tests

N/A.

#### 2B.5 Red Verification [software]

Run the suite against stubs. Every test must fail cleanly (assertion or `unimplemented!()`), never from syntax errors or missing fixtures. Fixtures are generated and committed before this step.

### 2C, 2D Figure / Manuscript Specification

N/A.

### 2E Traceability & Freeze [All]

| SC | Tests | Steps |
|---|---|---|
| SC-1a | T-001, T-002, T-003, T-025, T-026, T-027, T-032, T-033, T-034 | S-005 |
| SC-1b | T-020 (pass/fail); T-015 (report) | S-004, S-005, S-006 |
| SC-2 | T-004, T-005, T-023 | S-003, S-004 |
| SC-3 | T-007, T-016, T-024, T-030 | S-003, S-010, S-011, S-014 |
| SC-4 | T-008, T-009, T-022, T-028, T-031 | S-003, S-009 |
| SC-5 | T-006, T-010 | S-006, S-008, S-012, S-013 |
| SC-6 | T-014 + all CI-tier tests | S-015 |

Non-SC tests: T-011 → S-002, S-013; T-012, T-013, T-029 → S-007; T-017, T-018 → S-005; T-019, T-021 → S-002, S-014.

**Freeze.** SHA-256 hashes of all test files and fixtures written to `tests/FROZEN_MANIFEST.sha256` with a `FROZEN — DO NOT MODIFY` header; CI runs `sha256sum -c`.

**Immutability Rule.** Implementing agents cannot modify, skip, or weaken frozen tests.

---

## Phase 3: Plan Construction

### 3.1 Steps [All]

| Step | Title | Tier | Depends On | Outputs | Done When |
|---|---|---|---|---|---|
| S-001 | Workspace bootstrap | Sonnet | — | Cargo workspace (`gtcore`, `gtaudio`, `gtbridge`), `godot-project/`, CI config, interface stubs | `cargo build` succeeds; red suite fails cleanly |
| S-002 | Audio engine | Opus | S-001 | `gtaudio`: AudioBackend, cubeb duplex backend, file backend, rtrb rings, preloaded PCM playback, SFX mixer | T-011, T-021 (CI part) pass; spike 1 numbers reproduced |
| S-003 | Clock model | Opus | S-002 | `gtcore::clock` | T-023, T-024, T-031 pass |
| S-004 | Onset detector | Sonnet | S-001 | `gtcore::dsp::onset` | T-004, T-005 pass |
| S-005 | Note verifier | Opus | S-001 | `gtcore::dsp::verifier` | T-001–T-003, T-017, T-018, T-025–T-027, T-032, T-033 pass; T-034 pass or D-012 invoked |
| S-006 | Evidence pipeline | Sonnet | S-003, S-004, S-005 | `gtcore::pipeline` (onset + verifier + attribution, file-backend driver) | T-006, T-020 pass |
| S-007 | Chart format & parser | Sonnet | S-001 | `gtcore::chart` + JSON schema | T-012, T-013, T-029 pass |
| S-008 | Example song | Sonnet | S-007 | `assets/songs/example/` (chart, backing PCM, rendered guitar part) | Loads via S-007 parser |
| S-009 | Scoring engine | Sonnet | S-006, S-007 | `gtcore::scoring` | T-008, T-009, T-028 pass |
| S-010 | Godot bridge | Opus | S-002, S-003 | `gtbridge` (gdext): `GuitarEngine` node, event-queue drain, Dummy audio driver config | T-030 passes |
| S-011 | Note highway | Opus | S-007, S-010 | `gtcore::highway` + `godot-project/highway/` | T-007 passes; T-016 on HW |
| S-012 | Session logger | Haiku | S-009 | `gtcore::session` (atomic temp-file + rename) | Log written; used by T-010 |
| S-013 | End-to-end integration | Opus | S-008, S-009, S-011, S-012 | App wiring; settings (δ_a, δ_v, low-latency toggle, offset assist); macOS mic-permission plist entry | T-010 passes |
| S-014 | Performance hardening | Opus | S-013 | Optimized code | T-015, T-016, T-019, T-021 (HW) pass or report |
| S-015 | CI & release prep | Sonnet | S-014 | `.github/workflows/ci.yml` | T-014 passes; all CI-tier frozen tests green |

S-004, S-005 and S-007 depend only on S-001 and can run in parallel with the audio engine.

### 3.2 Decision Rules [All]

**Engineering forks:**

- **D-001** If cubeb cannot open a low-latency duplex stream on a platform, use cpal there and log the substitution. cpal opens separate input and output streams, so that platform uses the D-010 drift path.
- **D-002 (replaced)** Verifier tuning: if T-020 fails, lower the W floor from 512 to 384 and confirmation from 3 to 2 hops, then re-run all SC-1a tests. If T-032 fails, lower θ_k for k = 2 only and re-run T-025. If SC-1a and SC-1b cannot both be met, escalate to G-001 with a trade-off table. SC-1a is never traded for SC-1b.
- **D-003 (revised)** If the gdext bridge cannot pass T-030 (compatibility or threading), replace Godot with a Bevy/wgpu renderer over the same `gtcore`/`gtaudio` crates. Only S-010 and S-011 are re-planned.
- **D-004** If onset detection fires on note decay, raise the spectral-flux threshold adaptively from recent RMS.
- **D-005** WASAPI: default shared mode; a "low-latency mode" toggle requests exclusive mode; on failure, fall back to shared and continue (absorbs Rev 1 D-009).
- **D-006** *Retired* (dual-resolution analysis).
- **D-007 (strengthened)** RT callback rules and three-thread model per §2B.1e.
- **D-008** Timing windows Perfect ±15 / Great ±30 / Good ±50 ms on onset timing error in song time; pitch gate is the ±50-cent identity band (A-012).
- **D-010** Distinct input/output devices: two counters, drift ratio from the clock fits, UI warning (T-031).
- **D-011** Present evidence without an attributed onset → Miss unless `legato` (excluded in milestone 1). Later milestone: legato timestamp = pitch-lock frame minus a calibrated lag.
- **D-012** If T-034 fails, or the G-001 real-guitar let-ring false-reject rate exceeds 5%, add a harmonic-comb salience verifier (spectral energy at k·f0, k = 1…8, against the local spectral floor) and accept a note if either verifier reports Present and neither reports HarmonicUp. Re-run SC-1a and SC-1b tests. This is also the base for chords (A-004 follow-up).
- **D-013** Offset assist: the session log records the median signed timing error over Present notes (n ≥ 30); settings offer to apply it to δ_a on user confirmation.

**Latency pivot (replaced):** if SC-1b still fails after D-002, keep timestamp-based scoring unchanged, accept the higher decision latency, and document the feedback-latency impact in `DEVIATIONS.md`.

**Default rule:** per protocol §3.2.

### 3.3 Human Gates [All]

| Gate | Trigger | Evidence Bundle | Questions | Responses | Plan Branch |
|---|---|---|---|---|---|
| G-001 | After S-006, with S-002/S-003 run on reference hardware | Spike outputs; T-020 latency report; SC-1a accuracy report incl. T-032–T-034; L_rt measured vs. reported (T-015, T-019); a 5-minute real-guitar recording (incl. let-ring passages) run through the pipeline against a hand-annotated chart | Is SC-1b met? On real guitar, are ≥95% of correctly played notes Present and ≤2% of wrong notes falsely Present? Does D-012 apply? | proceed / proceed-with-rescope: \<text\> / stop | proceed → remaining steps; rescope → adjust D-002/D-012 |
| G-002 | After S-013 | Session log; FPS measurements; scoring samples; T-022 playthrough; calibration flow (δ_a, δ_v, offset assist) | Is gameplay acceptable? Are windows fair? | proceed / proceed-with-rescope / stop | proceed → S-014, S-015 |

No other gates — no deployment, release, or external action.

### 3.4 Execution Resiliency [All]

All steps idempotent (`cargo build`, `cargo test` re-runnable). Long-running hardware tests checkpoint to `tests/results/`. Session logs written atomically (temp file + rename).

### 3.5 Handoff Document [All]

`HANDOFF.md` contains: purpose; active profiles (software); reading order PROFILE.md → PLAN.md (§2B.1b–e first: verifier, latency budget, clock model, threads) → DECISIONS.md → GATES.md → ENVIRONMENT.md; environment setup (rustup 1.83, cargo-audit, Godot 4.3, gdext per ENVIRONMENT.md); how to run the frozen suite (`cargo test`, CI tier) and hardware tier; freeze verification (`sha256sum -c tests/FROZEN_MANIFEST.sha256`); step list S-001–S-015; gates G-001, G-002; halt protocol (any frozen test fails → halt, write `BLOCKED.md`); Integrity Rule (Rule 9) verbatim.

### 3.6 Cold-Read Gate [All]

Unchanged procedure; must be re-run for this revision. (1) Fresh Sonnet agent lists every question, undefined term, missing input or judgment call in HANDOFF.md and PLAN.md. (2) Haiku agent verifies all paths, commands and IDs exist or are created by an earlier step. (3) Amend until zero items; maximum 5 passes.

---

## Phase 4: Recursive Pre-Mortem Analysis [All] — Rerun on Revision 2 Baseline

**Baseline:** chart-informed constrained-NSDF verifier with harmonic sub-lag rejection; frame-index timestamps and L_rt calibration; one cubeb duplex stream; Godot via gdext with Dummy audio driver; three-thread model.

### Round 1 (Opus, fresh context)

*Simulation: six months after delivery, the system has failed.*

**Carried-over risks, re-assessed:**

| R-### | Failure Mode | Rev 1 | Rev 2 | Reason |
|---|---|---|---|---|
| R-001 | WASAPI shared-mode latency high | Critical | Medium | Now affects feedback delay only; judgment accuracy is preserved by L_rt calibration (D-005, D-013). |
| R-002 | Low-E octave errors from short window | High | *Retired* | Premise false (window covered ~7.7 periods); dual-resolution removed; octave handling is in the verifier. |
| R-003 | Godot stalls cause input overruns | High | Low | RT and DSP threads are independent of Godot's main thread (§2B.1e); T-021. |
| R-004 | macOS microphone permission missing | Medium | Medium | Plist entry added in S-013; checked at G-002. |
| R-005 | Chart parser stack overflow | Medium | Low | Depth/size limits, T-029. |
| R-006 | Timing windows feel unfair | High | Medium | T-022 at G-002; D-013 removes systematic bias as a confound. |

**New risks from the Revision 2 baseline:**

| R-### | Failure Mode | Lens | Severity | Likelihood | Root Cause | Mitigation | Traces To |
|---|---|---|---|---|---|---|---|
| R-101 | Correct low-string notes rejected as HarmonicUp(2) | Technical | High | Medium | Sub-lag rejection relies on odd-harmonic energy; pickups and pick position can weaken the fundamental. | T-032 fixtures; θ_k calibrated in spike 2; real recording at G-001; D-002. | S-005 |
| R-102 | Attack pitch glide causes wrong early decisions | Technical | Medium | Medium | Plucked strings start sharp; decision may land on the transient. | G_attack guard, 3-hop confirmation, accuracy measured on sustain; T-033. | S-005 |
| R-103 | Godot opens the audio device, blocking exclusive mode or adding a second client | Operational | Medium | Medium | Godot's default audio driver. | Dummy driver (C-012); all SFX through the Rust mixer; T-030 asserts driver. | S-010 |
| R-104 | Drivers misreport latency → systematic timing bias ("always Great, late") | Technical | High | High | cubeb-reported L_in/L_out trusted as ground truth. | δ_a slider plus offset assist (D-013); T-015 compares reported vs. loopback. | S-003, S-013 |
| R-105 | Separate input/output devices drift (50 ppm × 300 s = 15 ms) | Technical | Medium | Medium | No shared clock across devices. | A-011 default; D-010 estimator; T-031. | S-003 |
| R-106 | Legato notes lack pick onsets → Miss | Technical | Low | Low (milestone 1) | Onset-gated scoring. | Excluded by A-014; D-011 defines the later path. Becomes High when user charts arrive. | S-006 |
| R-107 | Repeated same-pitch notes double-counted or merged | Technical | Medium | Medium | Onset attribution ambiguity. | Greedy one-onset-per-note attribution; T-028. | S-006, S-009 |
| R-108 | Let-ring mixtures cause false rejects in normal playing | Technical | High | High | The difference function assumes one periodic source; a ringing previous note breaks periodicity at τ0. | T-034 frozen at baseline; D-012 harmonic-comb fork; let-ring passages in the G-001 recording. | S-005 |
| R-109 | gdext / Godot 4.3 incompatibility blocks the bridge | Operational | Medium | Low | External binding maturity. | Engine-agnostic core; D-003. | S-010, S-011 |

**Round 1 result:** 0 Critical, 3 High (R-101, R-104, R-108). Mitigations add D-010–D-013 and T-031–T-034 (already incorporated in Phases 2–3). Re-freeze and re-run the 3.6 Cold-Read Gate.

### Round 2

*Re-simulation with mitigations in place.*

- R-101 → Medium (fixture-covered; real-guitar confirmation at G-001).
- R-104 → Medium (bias is visible and correctable in one step).
- R-108 → Medium (fork defined; outcome depends on T-034 and G-001 evidence).

New risks introduced by the mitigations:

| R-### | Failure Mode | Severity | Likelihood | Mitigation |
|---|---|---|---|---|
| R-110 | D-012 comb verifier has different latency/accuracy and breaks SC-1b | Medium | Low | If D-012 triggers, T-020 and all SC-1a tests re-run before G-001 closes. |
| R-111 | Offset assist absorbs a player's genuine rushing/dragging | Low | Medium | Applied only on user confirmation; sample size shown. |

**Round 2 result:** 0 Critical, 0 High.

### Round 3

Confirmation pass; no new items. Stop.

### Final Risk Register (provisional)

Post-mitigation severities for R-101, R-104, R-105 and R-108 depend on spike and G-001 evidence and are provisional until then.

| Severity | Count | Risks |
|---|---|---|
| Critical | 0 | — |
| High | 0 | — |
| Medium | 8 | R-001, R-004, R-006, R-101, R-102, R-104, R-105, R-108 |
| Low | 8 | R-003, R-005, R-103, R-106, R-107, R-109, R-110, R-111 |
| Retired | 1 | R-002 |

---

## Phase 5: GitHub Push & Workflow Termination [All]

### 5.1 Final Consistency Check (Haiku)

Freeze manifest hashes verified; every applicable file exists; no N/A artifact referenced; every traceability row complete; `plan/GATES.md` defines G-001 and G-002; `state.json` shows all applicable phases complete; `plan/OPERATIONS.md` absent (correct).

### 5.2 Status Header

**NOT READY — review points 7 and 8 open.**

Active profiles: software (only).

Counts:

- Claims: 13 active (C-006, C-008 retired); 0 verified — R2–R6 and spikes not yet executed.
- Tests: 34 (29 CI-tier, 4 HW/tool-tier, 1 human).
- Steps: 15 (S-001–S-015).
- Gates: 2 (G-001, G-002).
- Risks: 0 Critical, 0 High, 8 Medium, 8 Low (provisional), 1 retired.

Open before READY:

- **Point 7:** decide how HW-tier (T-015, T-016, T-019, T-030) and Human-tier (T-022) tests relate to SC-6; T-014 needs an advisory database, which conflicts with "no network" unless vendored.
- **Point 8:** execute R2–R6 and record evidence; verify all citations (C-005 source; A-010 precedent); licence review (Essentia port and aubio are now removed, which simplifies it).

### 5.3 Commit and Push

Commit to `gen-<timestamp>-rocksmith-clone`; push to origin. On failure, retry 3× with backoff; then `git bundle create rocksmith-clone.bundle --all` and report the path.

### 5.4 STOP INSTRUCTION

```
Final commit hash: <computed>
Branch URL: <remote>/gen-<timestamp>-rocksmith-clone
Active profiles: software
Plan status: NOT READY (review points 7, 8 open)
Research summary: not yet executed; 13 active claims; C-003 corrected and awaiting spike verification.
Residual Medium risks: WASAPI feedback latency; macOS permission; timing-window fairness;
  weak-fundamental false rejects; attack glide; misreported driver latency;
  cross-device drift; let-ring mixtures.
Residual Low risks: render stalls; parser limits; Godot device contention; legato (milestone 1);
  same-pitch attribution; gdext compatibility; comb-verifier fork; offset-assist bias.
Execution has halted. No plan step has been executed. No deployment, release, or external action has been taken.
```

# Engineer Handoff Guide (HANDOFF.md)

## 1. Project Purpose & Scope
This repository houses the planning and architectural specification for the **Rocksmith Clone (planning-protocol-v3.1)**: a native, low-latency electric guitar rhythm game and learning engine built with a Rust audio/DSP core and Godot 4.3 slave 3D renderer.

- **Active Profile:** `software` only.
- **Deploys:** `false` (native local desktop build target: Windows, macOS, Linux).
- **Current State:** Plan fully revised, vetted, and frozen. Ready for Step Execution (S-001 through S-014).

---

## 2. Document Reading Order
Before executing any step, the implementing engineer or subagent must read the plan documents in the following strict order:
1. `plan/PROFILE.md` — Active profiles and boundary constraints.
2. `plan/PLAN.md` — Step-by-step roadmap, done criteria, and gate definitions.
3. `plan/DECISIONS.md` — Architectural contracts, interfaces, and core algorithms.
4. `plan/ASSUMPTIONS.md` — Ambiguity resolutions and defaults.
5. `plan/ENVIRONMENT.md` — Pinned compiler, library versions, buffer sizes, and thread budgets.
6. `plan/GATES.md` — Evidence requirements for Gate G-001 and Gate G-002.
7. `plan/TRACEABILITY.md` — Mapping of Success Criteria to steps and test IDs.
8. `premortem/RISK_REGISTER.md` — Failure modes and mitigation strategies.

---

## 3. Core Architectural Contracts (Non-Negotiable)

1. **Temporal Note Tracker (Transient/Pitch Reconciliation):**
   - Plucked strings exhibit chaotic, broadband inharmonic behavior for the first $15\text{ ms}$. 
   - Never evaluate pitch at the exact instant of onset detection. 
   - Use `TemporalNoteTracker` with a $15\text{--}35\text{ ms}$ consensus window to produce `ResolvedNoteEvent`.
2. **Multi-Rate Low-Register Pitch Detection:**
   - Downsample by $4\times$ (to $11.025\text{ kHz}$) using an IIR half-band decimation filter for frequencies below $200\text{ Hz}$. 
   - A 1024-sample window at $11.025\text{ kHz}$ captures $92.8\text{ ms}$ of signal ($\approx 7.6$ full periods of $E_2$), eliminating octave jumps without exceeding the $50\text{ ms}$ latency budget.
3. **Duplex Audio Clocking:**
   - Backing track playback and guitar DI capture MUST run inside the same full-duplex Cubeb stream callback. 
   - Never run separate audio streams for input and output.
4. **Thread Isolation & Zero-Allocation Callback:**
   - Audio callback runs on an OS real-time thread. 
   - Absolutely no memory allocations (`Vec::new`, `Box`, etc.), mutex locking, file I/O, or FFI calls to Godot within the audio callback.
   - All communication to the main/render thread must pass through a lock-free SPSC ring buffer (`rtrb`).
5. **Headless CI Testing:**
   - `T-015a` is a deterministic DSP throughput test using synthetic buffers in CI. 
   - `T-015b` is a manual physical loopback test conducted only during Gate G-001.

---

## 4. Execution Step List & Gate Protocol

- **Batch 1 (S-001 through S-005):**
  - S-001: Workspace Bootstrap & Domain Types
  - S-002: Duplex Audio I/O (`cubeb`, `cpal`, `MockAudioBackend`, `rtrb`)
  - S-003: Multi-rate Pitch Detector (Hybrid MPM/YIN)
  - S-004: Onset Detector & Pre-Conditioning Filter
  - S-005: Temporal Note Tracker
- **Gate G-001 (Human Review):**
  - Collect pitch accuracy and latency evidence.
  - Halt and present evidence bundle to human. Await `proceed`.
- **Batch 2 (S-006 through S-012):**
  - S-006: Chart Format & Parser
  - S-007: Godot 4.3 GDExtension Slave Renderer
  - S-008: Audio-Clock Synchronization & Dual Offset Sliders
  - S-009: Scoring & Judgment Engine
  - S-010: Session Logger & Statistics
  - S-011: Reference Song & Audio Fixtures
  - S-012: End-to-End Playthrough Integration
- **Gate G-002 (Human Review):**
  - Present 60 FPS highway trace and scoring logs. Await `proceed`.
- **Batch 3 (S-013 through S-014):**
  - S-013: SIMD Hardening & Latency Tuning
  - S-014: CI Verification & Frozen Manifest Audit

---

## 5. Integrity Rule (Rule 9)
> *The implementing agent must never weaken, delete, or skip frozen tests. Any test failure must be resolved by fixing the application code, never by relaxing test tolerances or assertions.*

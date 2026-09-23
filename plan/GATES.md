# Quality & Verification Gates (plan/GATES.md)

## Gate G-001: DSP Accuracy, Latency & Pipeline Validation

- **Trigger:** Immediate completion of Step S-005 (Duplex I/O, Multi-rate Pitch, Onset, and Temporal Note Tracker).
- **Evidence Bundle Required:**
  1. `reports/pitch_accuracy.json`: Synthetic sine and sawtooth frequency accuracy across $E_2\text{--}E_5$ ($82.4\text{--}659.3\text{ Hz}$).
  2. `reports/low_e_octave_stability.json`: Verification that $E_2$ downsampling produces 0% octave jumping/sub-harmonic misdetections.
  3. `reports/dsp_throughput_t015a.json`: Deterministic benchmark demonstrating $\le 2.0\text{ ms}$ processing time per 2048-sample hop on standard CPU.
  4. `reports/temporal_reconciliation.json`: Demonstration that guitar pick transients ($0\text{--}15\text{ ms}$) are cleanly rejected in favor of the consensus fundamental ($15\text{--}35\text{ ms}$).
  5. `reports/hardware_latency_t015b.md`: Loopback latency report on reference testing workstation.
- **Required Review Questions:**
  1. *Is monophonic pitch detection accuracy within $\le 10\text{ cents}$ across all standard guitar strings?*
  2. *Does the multi-rate downsampling architecture successfully eliminate low-E octave errors while preserving the $<50\text{ ms}$ overall latency budget?*
- **Allowed Human Responses & Branching:**
  - `proceed`: Proceed directly to Step S-006 (Chart parser) through S-014.
  - `proceed-with-rescope: <instructions>`: Adjust parameters (e.g. increase consensus window or tweak decimation filter) via Decision D-002/D-006 and re-verify before S-006.
  - `stop`: Halt execution immediately and log blocking issues.

---

## Gate G-002: End-to-End Gameplay & Highway Synchronization

- **Trigger:** Immediate completion of Step S-012 (End-to-End Playthrough & Godot GDExtension Integration).
- **Evidence Bundle Required:**
  1. `reports/fps_frame_timing.json`: Frame timing trace during complete playthrough of example song demonstrating zero frame drops below 60 FPS.
  2. `reports/session_score_sample.json`: Exported session score log evaluating timing accuracy (Perfect, Great, Good, Miss) on a human or automated playback.
  3. `reports/calibration_verification.md`: Verification that manual audio and visual offset sliders adjust the hit line and audio playhead predictably without introducing audio stutter.
- **Required Review Questions:**
  1. *Does the 3D note highway scroll smoothly with sample-accurate alignment to the backing track?*
  2. *Do the scoring timing windows feel natural and fair during actual guitar play?*
- **Allowed Human Responses & Branching:**
  - `proceed`: Proceed directly to Step S-013 (Hardening) and S-014 (CI release freeze).
  - `proceed-with-rescope: <instructions>`: Adjust timing window thresholds in `plan/DECISIONS.md` (Decision D-008).
  - `stop`: Halt execution immediately.

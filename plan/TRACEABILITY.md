# Traceability Matrix (plan/TRACEABILITY.md)

| Requirement / Criterion | Description | Implementing Steps | Verifying Tests |
|---|---|---|---|
| **SC-1** | Real-time monophonic pitch detection of guitar notes $E_2\text{--}E_5$ with $\le 50\text{ ms}$ latency and $\le 10\text{ cent}$ error | S-003, S-005, S-013 | `T-001` (A4 440Hz), `T-002` (E2 82.41Hz low octave check), `T-003` (E5 659.26Hz), `T-015a` (DSP throughput), `T-017` (silence rejection), `T-018` (noise rejection), `T-020` (E2 decimation test) |
| **SC-2** | Onset detection within $20\text{ ms}$ of ground truth on reference electric guitar recording | S-004, S-005 | `T-004` (click localization $\le 20\text{ ms}$), `T-005` (silence stability), `T-006` (real DI guitar track onset sequence) |
| **SC-3** | 3D note highway renders and scrolls in sync with audio at $\ge 60\text{ FPS}$ | S-007, S-008 | `T-007` (highway note coordinates), `T-016` (60 FPS render timing trace) |
| **SC-4** | Scoring engine produces per-note judgment (Perfect/Great/Good/Miss) according to timing windows | S-009 | `T-008` (Perfect $\le 15\text{ ms}$), `T-009` (Miss $> 50\text{ ms}$), `T-022` (reference human timing playback) |
| **SC-5** | End-to-end playable session on example song with session log output | S-010, S-011, S-012 | `T-010` (full song playback & session JSON schema verification) |
| **SC-6** | Deterministic test execution in CI with zero network access | S-001, S-014 | `T-011` (MockAudioBackend device resilience), `T-012` (malformed chart handling), `T-013` (chart path traversal guard), `T-014` (`cargo audit` vulnerability check) |
| **Hardware Gate** | Physical audio interface latency on reference hardware ($\le 50\text{ ms}$ roundtrip) | S-002, S-013 | `T-015b` (Hardware loopback test, executed manually at Gate G-001) |
| **OS Driver Gate** | Windows WASAPI Exclusive Mode low-latency fallback | S-002 | `T-019` (WASAPI driver mode switch and offset calibration) |
| **Audio Thread** | Real-time thread isolation under heavy GPU load | S-002, S-007 | `T-021` (Zero underruns during simulated 100% GPU frame spike) |

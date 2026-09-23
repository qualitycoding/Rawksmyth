# Assumptions

Status: **confirmed by human at intake, 2026-09-23.** All proposed defaults adopted. Full rationale and consequences in `plan/PLAN.md` §0.3.6.

| ID | Topic | Confirmed Decision |
|---|---|---|
| A-001 | Rendering engine | Godot 4.3 via godot-rust (gdext); core crates engine-agnostic (D-003 fallback: Bevy/wgpu) |
| A-002 | Audio library | cubeb (cubeb-rs), single duplex stream; cpal fallback per D-001 |
| A-003 | Detection approach | Chart-informed constrained NSDF verifier on the scoring path; no blind estimator in milestone 1 |
| A-004 | Polyphony | Monophonic only; let-ring handled by D-012 fork |
| A-005 | Example song | Original procedural composition |
| A-006 | Guitar input | Dry signal only |
| A-007 | Scoring model | Rocksmith-style: pitch gate + timing grade |
| A-008 | Latency calibration | Manual audio offset δ_a and video offset δ_v, plus offset assist (D-013) |
| A-009 | Repository | `qualitycoding/Rawksmyth` |
| A-010 | License | Apache-2.0 (changed from AGPL-3.0 default on 2026-09-23; `LICENSE` at repo root; dependency compatibility check open — review point 8) |
| A-011 | Input/output device | Same device for input and output; distinct devices supported with warning (D-010) |
| A-012 | Pitch identity band | ±50 cents, configurable |
| A-013 | String identity | Not verified; any string producing the charted pitch counts |
| A-014 | Legato notes | Excluded from milestone-1 charts; notes without a pick onset judged Miss (D-011) |

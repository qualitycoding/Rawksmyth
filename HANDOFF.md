# HANDOFF

## Purpose

Build a Rocksmith-style guitar learning game (plan/PLAN.md, Revision 2). This document is the entry point for anyone — human or agent — picking this up.

## Reading order

1. This file.
2. `plan/PLAN.md` §2B.1b–e (verifier, latency budget, clock model, threads) — needed to understand `gtcore` before reading its code.
3. `plan/ASSUMPTIONS.md` — confirmed defaults A-001–A-014.
4. `plan/PLAN.md` in full (DECISIONS, GATES, ENVIRONMENT sections are inline, not yet split into separate files).

## Status: partial implementation, NOT READY

This is **not** a completed plan execution. Phase 1 (research rounds, empirical spikes) was never run — no claim in `research/` is verified or corroborated by anything other than citation-free derivation. Phase 4's pre-mortem is the desk-based rerun from the planning conversation, not re-validated against real code or hardware. Treat every number below as provisional until G-001.

### What exists and is tested

`gtcore` — the engine-agnostic, hardware-independent core crate:

- **`gtcore/src/chart.rs`** (step S-007): chart format, parser, hardening. Implements T-012 (malformed input never panics), T-013 (asset path traversal rejected), T-029 (oversized / deeply-nested input rejected without stack overflow — the depth scan is iterative, not recursive), and A-014 (legato notes rejected at parse time, since milestone-1 charts must not contain them).
- **`gtcore/src/clock.rs`** (step S-003, partial): the `ClockModel::song_time` formula and the `RenderClockFit` least-squares fit from plan §2B.1d. Implements T-023 (song_time formula, checked across a parameter grid) and the *pure-math portion only* of T-024 (fit recovers the correct slope under synthetic jitter).

All 15 tests pass under `cargo test`. The freeze manifest (`tests/FROZEN_MANIFEST.sha256`) covers `chart.rs` and `clock.rs`; verify with `sha256sum -c tests/FROZEN_MANIFEST.sha256`. Per the Immutability Rule (protocol Rule 9), these files' existing tests must not be modified, skipped, or weakened — only added to, with a re-freeze.

### What is deliberately NOT implemented, and why

This was built in a sandboxed container with **no audio hardware, no display server, and no route to install Godot or a GUI toolkit.** Rather than fake these, they are left as stubs or omitted:

| Component | Step | Why deferred |
|---|---|---|
| `dsp::onset` (onset detector) | S-004 | Needs the spike-2/3 fixture work (KS-synthesis plucks) to calibrate against; not yet ported from the plan's pseudocode to tested Rust. |
| `dsp::verifier` (constrained-NSDF, harmonic sub-lag rejection) | S-005 | Same — this is the highest-risk component (R-101, R-108) and should not be written without the fixture generator (§2B.2) and spike 2 results to validate against. |
| `pipeline`, `scoring`, `highway` | S-006, S-009, S-011 | Depend on S-004/S-005. |
| `gtaudio` (cubeb duplex backend) | S-002 | Needs a real audio interface for loopback measurement (spike 1) and cannot be meaningfully tested without one. A **file backend** (deterministic WAV-driven, per §2B.1e) is the next reasonable step and does not need hardware — not yet written. |
| `gtbridge` (gdext / Godot bridge) | S-010 | No Godot binary or display server available in this environment. |
| Fixture generator (`tests/fixtures/gen/`) | §2B.2 | Not yet written. Needed before S-004/S-005 can be tested at all. |

### Toolchain deviation

`plan/PLAN.md` §1.4 pins Rust 1.83.0. This environment could only install **Rust 1.75.0** via `apt` (no route to `rustup.rs`, not on the network allowlist). The code as written does not use any 1.75→1.83 feature gap that I'm aware of, but this has not been checked mechanically. **Pin and verify 1.83.0 before any real CI run or G-001 evidence bundle** — the `.github/workflows/ci.yml` added here correctly targets 1.83.0 via rustup (GitHub Actions runners have unrestricted network), so this only affects local sandbox development, not CI.

### How to run what exists

```
cargo test --workspace                          # 15 tests, no hardware needed
sha256sum -c tests/FROZEN_MANIFEST.sha256        # freeze verification
```

### Suggested next steps, in dependency order

1. `tests/fixtures/gen/` — Karplus-Strong fixture generator (§2B.2), CI-tier, no hardware needed.
2. `dsp::onset` + T-004/T-005 against the fixture generator.
3. `dsp::verifier` + T-001–T-003, T-017, T-018, T-025–T-027, T-032, T-033 against fixtures.
4. `pipeline` (file-backend driver) + T-006, T-020.
5. `gtaudio` file backend (no hardware) enabling T-010, T-011 (CI part).
6. Only once hardware is available: `gtaudio` cubeb backend, spike 1, spike 2 real-guitar validation, G-001.

## Halt protocol

If any frozen test fails, halt and write `BLOCKED.md` describing which test, the failure, and whether it indicates a bug or a needed plan revision.

## Integrity Rule (Rule 9)

Implementing agents cannot modify, skip, or weaken frozen tests. Restated verbatim from protocol §Rule 9 as required by plan §3.5.

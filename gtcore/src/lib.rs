//! gtcore: engine-agnostic, hardware-independent core of the Rocksmith
//! clone (plan/PLAN.md). No dependency on cubeb, a real audio device, or
//! Godot — everything here is CI-tier testable (plan §2B.3).
//!
//! Implemented so far (see HANDOFF.md for what is deferred and why):
//! - `chart`: chart format + parser (S-007) — T-012, T-013, T-029, A-014.
//! - `clock`: clock model + render-clock fit math (S-003, partial) — T-023,
//!   and the pure-math portion of T-024.
//!
//! NOT implemented in this pass (require real audio hardware, a live
//! cubeb stream, or a Godot/gdext build — see plan Phase 1 spikes and
//! gate G-001): `dsp::onset`, `dsp::verifier`, `pipeline`, `scoring`,
//! `highway`, and all of `gtaudio` / `gtbridge`.

pub mod chart;
pub mod clock;

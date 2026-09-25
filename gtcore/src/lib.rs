//! gtcore: engine-agnostic, hardware-independent core of the Rocksmith
//! clone (plan/PLAN.md). No dependency on cubeb, a real audio device, or
//! Godot — everything here is CI-tier testable (plan §2B.3).
//!
//! - `chart`: chart format + parser (S-007).
//! - `clock`, `drift`: clock model, render-clock fit, two-device drift (S-003).
//! - `dsp`: onset detector (S-004) and constrained-NSDF note verifier (S-005).
//! - `pipeline`: onset + verifier + attribution, block driven (S-006).
//! - `scoring`: timing-window judgments (S-009).
//! - `highway`: note-highway layout (S-011, engine-agnostic half).
//! - `session`: session log, atomic write (S-012).
//!
//! Not here (need hardware / Godot; see HANDOFF.md): the cubeb backend, the
//! gdext bridge and the Godot scene. `gtaudio` and `gtapp` hold the rest.

pub mod chart;
pub mod clock;
pub mod dsp;
pub mod drift;
pub mod highway;
pub mod pipeline;
pub mod scoring;
pub mod session;

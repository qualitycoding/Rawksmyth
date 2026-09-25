//! gtaudio: audio engine pieces that need no audio hardware (plan step S-002).
//!
//! - `wav`: minimal RIFF/WAVE reader and writer (no panics on malformed input).
//! - `ring`: wait-free SPSC ring buffer (safe Rust, atomics only).
//! - `seqlock`: single-writer seqlock publishing `(monotonic_ns, frames)`
//!   clock samples from the RT callback to the game thread (§2B.1d).
//! - `rt`: the RT callback core — copies input into a ring, writes output
//!   from preloaded PCM plus an SFX mixer, publishes a clock sample; it never
//!   allocates, locks, or does I/O (§2B.1e, D-007, T-021).
//! - `backend`: the `AudioBackend` trait and the deterministic `FileBackend`
//!   (faster than real time, with jitter and device-loss injection), plus a
//!   two-device clock simulator for D-010 / T-031.
//!
//! NOT here: the cubeb duplex backend (needs a real interface to verify).

pub mod backend;
pub mod ring;
pub mod rt;
pub mod seqlock;
pub mod wav;

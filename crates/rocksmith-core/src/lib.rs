//! Rocksmith Core Engine Library (Rust 1.83+)
//!
//! Provides ultra-low latency audio capture, multi-rate McLeod Pitch Method (MPM),
//! adaptive spectral flux onset detection, temporal consensus note tracking,
//! sample-accurate clock synchronization, chart parsing, and real-time scoring.

pub mod audio;
pub mod chart;
pub mod domain;
pub mod dsp;
pub mod scoring;
pub mod session;
pub mod sync;

pub use audio::{AudioBackend, DuplexAudioConfig, MockAudioBackend};
pub use chart::{ChartParser, SongChart, SongMetadata};
pub use domain::*;
pub use dsp::{
    ButterworthLowPass, Decimator4x, MultiRatePitchDetector, SpectralFluxOnsetDetector,
    TemporalConsensusTracker,
};
pub use scoring::RocksmithScorer;
pub use session::{ScoredNoteRecord, SessionLog, SessionLogger};
pub use sync::ClockSynchronizer;

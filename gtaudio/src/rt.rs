//! The real-time callback core (plan §2B.1e, D-007).
//!
//! `RtCore::callback` is what runs inside the audio device callback (cubeb
//! duplex, or the file backend in tests). Per call it: pushes the input block
//! into an SPSC ring for the DSP thread; writes the output block from
//! preloaded PCM starting at output frame `s0`; mixes triggered SFX voices;
//! and publishes `(monotonic_ns, frames_written)` through the clock seqlock.
//!
//! **RT rules**: no allocation, locks, syscalls or logging inside `callback`.
//! Everything it touches is sized at construction. `tests/t021_rt_alloc.rs`
//! checks the allocation rule with a counting global allocator.

use crate::ring::{ring, Consumer, Producer};
use crate::seqlock::{clock_channel, ClockReader, ClockWriter};
use gtcore::clock::{ClockSample, Frame};

pub const MAX_VOICES: usize = 8;

#[derive(Clone, Copy)]
struct Voice {
    sfx: usize,
    pos: usize,
    active: bool,
}

pub struct RtConfig {
    /// Interleaved output channels (input is always one channel: the guitar).
    pub channels: usize,
    /// Output frame at which the backing track's first frame plays (§2B.1d).
    pub s0: Frame,
    /// Input ring capacity in samples (must comfortably exceed the DSP
    /// thread's worst-case stall; 1 s = 44100).
    pub input_ring_samples: usize,
}

/// Handles for the non-RT side.
pub struct RtHandles {
    /// DSP thread reads guitar input here.
    pub input: Consumer,
    /// Game thread reads the latest clock sample here.
    pub clock: ClockReader,
    /// Game thread triggers SFX by pushing the SFX index as one word.
    pub commands: Producer,
}

pub struct RtCore {
    channels: usize,
    backing: Vec<f32>,
    backing_frames: u64,
    s0: Frame,
    frame: Frame,
    input: Producer,
    clock: ClockWriter,
    commands: Consumer,
    sfx_bank: Vec<Vec<f32>>,
    voices: [Voice; MAX_VOICES],
}

impl RtCore {
    /// `backing`: interleaved with `cfg.channels` channels, fully decoded.
    /// `sfx_bank`: mono one-shots, triggered by index.
    pub fn new(cfg: RtConfig, backing: Vec<f32>, sfx_bank: Vec<Vec<f32>>) -> (RtCore, RtHandles) {
        assert!(cfg.channels >= 1);
        let (input_tx, input_rx) = ring(cfg.input_ring_samples);
        let (cmd_tx, cmd_rx) = ring(64);
        let (clock_w, clock_r) = clock_channel();
        let backing_frames = (backing.len() / cfg.channels) as u64;
        (
            RtCore {
                channels: cfg.channels,
                backing,
                backing_frames,
                s0: cfg.s0,
                frame: 0,
                input: input_tx,
                clock: clock_w,
                commands: cmd_rx,
                sfx_bank,
                voices: [Voice { sfx: 0, pos: 0, active: false }; MAX_VOICES],
            },
            RtHandles { input: input_rx, clock: clock_r, commands: cmd_tx },
        )
    }

    /// Frames processed so far (the shared input/output frame index, C-011).
    pub fn frame(&self) -> Frame {
        self.frame
    }

    /// Input words dropped because the DSP thread lagged (T-021, HW part:
    /// must stay 0 over a 10-minute run).
    pub fn input_overruns(&self) -> u64 {
        self.input.overruns()
    }

    /// One duplex callback. `input` is mono; `output` is interleaved with the
    /// configured channel count and must hold `input.len()` frames.
    pub fn callback(&mut self, input: &[f32], output: &mut [f32], now_ns: i64) {
        let n = input.len();
        let ch = self.channels;
        debug_assert_eq!(output.len(), n * ch, "duplex callbacks have equal frame counts (C-011)");

        self.input.push_samples(input);

        // SFX triggers.
        let mut words = [0u32; 8];
        loop {
            let k = self.commands.pop_words(&mut words);
            for &w in &words[..k] {
                let idx = w as usize;
                if idx < self.sfx_bank.len() {
                    if let Some(v) = self.voices.iter_mut().find(|v| !v.active) {
                        *v = Voice { sfx: idx, pos: 0, active: true };
                    }
                }
            }
            if k < words.len() {
                break;
            }
        }

        // Backing track (silence before s0 and after the end).
        for f in 0..n {
            let pos = self.frame + f as u64;
            let out = &mut output[f * ch..(f + 1) * ch];
            if pos >= self.s0 && pos - self.s0 < self.backing_frames {
                let b = (pos - self.s0) as usize * ch;
                out.copy_from_slice(&self.backing[b..b + ch]);
            } else {
                out.fill(0.0);
            }
        }

        // SFX voices.
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let clip = &self.sfx_bank[v.sfx];
            for f in 0..n {
                if v.pos >= clip.len() {
                    v.active = false;
                    break;
                }
                let s = clip[v.pos];
                v.pos += 1;
                for o in &mut output[f * ch..(f + 1) * ch] {
                    *o += s;
                }
            }
            if v.pos >= clip.len() {
                v.active = false;
            }
        }
        for o in output.iter_mut() {
            *o = o.clamp(-1.0, 1.0);
        }

        self.frame += n as u64;
        self.clock.publish(ClockSample { monotonic_ns: now_ns, frames_written: self.frame });
    }
}

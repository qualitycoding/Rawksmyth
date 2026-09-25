//! Single-writer seqlock publishing the RT callback's clock sample
//! `(monotonic_ns, frames_written)` to the game thread (§2B.1d). The writer
//! never blocks; a reader retries if it overlapped a write.

use gtcore::clock::ClockSample;
use std::sync::atomic::{fence, AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

struct Inner {
    seq: AtomicU64,
    ns: AtomicI64,
    frames: AtomicU64,
}

/// The one writer (RT thread).
pub struct ClockWriter(Arc<Inner>);
/// Any number of readers.
#[derive(Clone)]
pub struct ClockReader(Arc<Inner>);

pub fn clock_channel() -> (ClockWriter, ClockReader) {
    let inner = Arc::new(Inner { seq: AtomicU64::new(0), ns: AtomicI64::new(0), frames: AtomicU64::new(0) });
    (ClockWriter(inner.clone()), ClockReader(inner))
}

impl ClockWriter {
    pub fn publish(&mut self, s: ClockSample) {
        let i = &*self.0;
        let seq = i.seq.load(Ordering::Relaxed);
        i.seq.store(seq.wrapping_add(1), Ordering::Relaxed); // odd: write in progress
        fence(Ordering::Release);
        i.ns.store(s.monotonic_ns, Ordering::Relaxed);
        i.frames.store(s.frames_written, Ordering::Relaxed);
        i.seq.store(seq.wrapping_add(2), Ordering::Release); // even: stable
    }
}

impl ClockReader {
    /// Latest consistent sample, or `None` if nothing was published yet (or
    /// the writer kept overlapping — practically never).
    pub fn read(&self) -> Option<ClockSample> {
        let i = &*self.0;
        for _ in 0..64 {
            let s1 = i.seq.load(Ordering::Acquire);
            if s1 == 0 {
                return None;
            }
            if s1 & 1 == 1 {
                std::hint::spin_loop();
                continue;
            }
            let ns = i.ns.load(Ordering::Relaxed);
            let frames = i.frames.load(Ordering::Relaxed);
            fence(Ordering::Acquire);
            if i.seq.load(Ordering::Relaxed) == s1 {
                return Some(ClockSample { monotonic_ns: ns, frames_written: frames });
            }
        }
        None
    }
}

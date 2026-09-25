//! Wait-free single-producer/single-consumer ring buffer of 32-bit words.
//!
//! Safe Rust only: slots are `AtomicU32` (f32 samples are stored as bits), the
//! head/tail indices synchronize with release/acquire. Neither side ever
//! allocates, locks or blocks after construction, which is what the RT
//! callback needs (§2B.1e, D-007). When the consumer falls behind, the
//! producer drops the excess and counts it (`overruns`) — the RT thread must
//! never wait for the DSP thread.

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

struct Inner {
    buf: Box<[AtomicU32]>,
    mask: usize,
    /// Next slot to read (monotonic, wraps via mask on access).
    head: AtomicUsize,
    /// Next slot to write.
    tail: AtomicUsize,
    dropped: AtomicU64,
}

pub struct Producer(Arc<Inner>);
pub struct Consumer(Arc<Inner>);

/// Create a ring holding at least `capacity` words (rounded up to a power of
/// two).
pub fn ring(capacity: usize) -> (Producer, Consumer) {
    let cap = capacity.max(2).next_power_of_two();
    let inner = Arc::new(Inner {
        buf: (0..cap).map(|_| AtomicU32::new(0)).collect(),
        mask: cap - 1,
        head: AtomicUsize::new(0),
        tail: AtomicUsize::new(0),
        dropped: AtomicU64::new(0),
    });
    (Producer(inner.clone()), Consumer(inner))
}

impl Producer {
    pub fn capacity(&self) -> usize {
        self.0.mask + 1
    }

    /// Free space in words.
    pub fn free(&self) -> usize {
        let tail = self.0.tail.load(Ordering::Relaxed);
        let head = self.0.head.load(Ordering::Acquire);
        self.capacity() - tail.wrapping_sub(head)
    }

    /// Push words; returns how many were stored. Words that do not fit are
    /// dropped and added to the overrun count.
    pub fn push_words(&mut self, words: &[u32]) -> usize {
        let tail = self.0.tail.load(Ordering::Relaxed);
        let head = self.0.head.load(Ordering::Acquire);
        let free = self.capacity() - tail.wrapping_sub(head);
        let n = words.len().min(free);
        for (i, w) in words[..n].iter().enumerate() {
            self.0.buf[tail.wrapping_add(i) & self.0.mask].store(*w, Ordering::Relaxed);
        }
        self.0.tail.store(tail.wrapping_add(n), Ordering::Release);
        if n < words.len() {
            self.0.dropped.fetch_add((words.len() - n) as u64, Ordering::Relaxed);
        }
        n
    }

    /// Push samples (as bits); same overrun policy as `push_words`.
    pub fn push_samples(&mut self, samples: &[f32]) -> usize {
        let tail = self.0.tail.load(Ordering::Relaxed);
        let head = self.0.head.load(Ordering::Acquire);
        let free = self.capacity() - tail.wrapping_sub(head);
        let n = samples.len().min(free);
        for (i, s) in samples[..n].iter().enumerate() {
            self.0.buf[tail.wrapping_add(i) & self.0.mask].store(s.to_bits(), Ordering::Relaxed);
        }
        self.0.tail.store(tail.wrapping_add(n), Ordering::Release);
        if n < samples.len() {
            self.0.dropped.fetch_add((samples.len() - n) as u64, Ordering::Relaxed);
        }
        n
    }

    /// Total words dropped because the consumer was too slow.
    pub fn overruns(&self) -> u64 {
        self.0.dropped.load(Ordering::Relaxed)
    }
}

impl Consumer {
    /// Words currently readable.
    pub fn available(&self) -> usize {
        let head = self.0.head.load(Ordering::Relaxed);
        let tail = self.0.tail.load(Ordering::Acquire);
        tail.wrapping_sub(head)
    }

    /// Pop up to `out.len()` samples; returns how many were read.
    pub fn pop_samples(&mut self, out: &mut [f32]) -> usize {
        let head = self.0.head.load(Ordering::Relaxed);
        let tail = self.0.tail.load(Ordering::Acquire);
        let n = out.len().min(tail.wrapping_sub(head));
        for (i, o) in out[..n].iter_mut().enumerate() {
            *o = f32::from_bits(self.0.buf[head.wrapping_add(i) & self.0.mask].load(Ordering::Relaxed));
        }
        self.0.head.store(head.wrapping_add(n), Ordering::Release);
        n
    }

    pub fn pop_words(&mut self, out: &mut [u32]) -> usize {
        let head = self.0.head.load(Ordering::Relaxed);
        let tail = self.0.tail.load(Ordering::Acquire);
        let n = out.len().min(tail.wrapping_sub(head));
        for (i, o) in out[..n].iter_mut().enumerate() {
            *o = self.0.buf[head.wrapping_add(i) & self.0.mask].load(Ordering::Relaxed);
        }
        self.0.head.store(head.wrapping_add(n), Ordering::Release);
        n
    }

    pub fn overruns(&self) -> u64 {
        self.0.dropped.load(Ordering::Relaxed)
    }
}

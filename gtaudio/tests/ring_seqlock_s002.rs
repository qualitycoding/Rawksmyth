//! Lock-free primitives: SPSC ring and clock seqlock, single- and two-thread.

use gtaudio::ring::ring;
use gtaudio::seqlock::clock_channel;
use gtcore::clock::ClockSample;

#[test]
fn ring_preserves_order_across_wraparound() {
    let (mut p, mut c) = ring(8);
    assert_eq!(p.capacity(), 8);
    let mut next_in = 0u32;
    let mut next_out = 0u32;
    for round in 0..200 {
        let n = 1 + round % 7;
        let words: Vec<u32> = (0..n as u32).map(|i| next_in + i).collect();
        assert_eq!(p.push_words(&words), n);
        next_in += n as u32;
        let mut out = vec![0u32; 5];
        loop {
            let k = c.pop_words(&mut out);
            for w in &out[..k] {
                assert_eq!(*w, next_out);
                next_out += 1;
            }
            if k < out.len() {
                break;
            }
        }
    }
    assert_eq!(next_in, next_out);
    assert_eq!(p.overruns(), 0);
}

#[test]
fn ring_drops_and_counts_when_full_never_blocks() {
    let (mut p, mut c) = ring(4);
    assert_eq!(p.push_samples(&[1.0, 2.0, 3.0]), 3);
    assert_eq!(p.push_samples(&[4.0, 5.0, 6.0]), 1, "only one slot left");
    assert_eq!(p.overruns(), 2);
    let mut out = [0.0f32; 8];
    assert_eq!(c.pop_samples(&mut out), 4);
    assert_eq!(&out[..4], &[1.0, 2.0, 3.0, 4.0]);
    assert_eq!(c.overruns(), 2);
    assert_eq!(p.free(), 4);
}

#[test]
fn ring_f32_bits_round_trip_exactly() {
    let (mut p, mut c) = ring(16);
    let vals = [0.0f32, -0.0, 1.5, f32::MIN_POSITIVE, f32::MAX, -123.456, f32::INFINITY];
    p.push_samples(&vals);
    let mut out = [9.0f32; 7];
    assert_eq!(c.pop_samples(&mut out), 7);
    for (a, b) in vals.iter().zip(&out) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
}

/// Producer and consumer on different threads: every word arrives, in order,
/// exactly once (the consumer keeps up here, so nothing is dropped).
#[test]
fn ring_two_threads_in_order() {
    const N: u32 = 500_000;
    let (mut p, mut c) = ring(1024);
    let producer = std::thread::spawn(move || {
        let mut i = 0u32;
        while i < N {
            let end = (i + 100).min(N);
            let chunk: Vec<u32> = (i..end).collect();
            let mut sent = 0;
            while sent < chunk.len() {
                sent += p.push_words(&chunk[sent..]);
                // Full: wait for the consumer (a test-only luxury — the real
                // producer drops instead). Undo the drop accounting is not
                // needed; we only assert on received data.
            }
            i = end;
        }
    });
    let mut expect = 0u32;
    let mut buf = [0u32; 256];
    while expect < N {
        let k = c.pop_words(&mut buf);
        for w in &buf[..k] {
            assert_eq!(*w, expect);
            expect += 1;
        }
        if k == 0 {
            std::thread::yield_now();
        }
    }
    producer.join().unwrap();
}

#[test]
fn seqlock_reads_none_before_first_publish_then_latest() {
    let (mut w, r) = clock_channel();
    assert!(r.read().is_none());
    w.publish(ClockSample { monotonic_ns: 10, frames_written: 256 });
    w.publish(ClockSample { monotonic_ns: 20, frames_written: 512 });
    let s = r.read().unwrap();
    assert_eq!((s.monotonic_ns, s.frames_written), (20, 512));
}

/// A reader racing a writer never observes a torn pair: the invariant
/// frames == ns/1000 · 128 holds for every sample it reads.
#[test]
fn seqlock_never_returns_a_torn_sample() {
    let (mut w, r) = clock_channel();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop2 = stop.clone();
    let writer = std::thread::spawn(move || {
        let mut n = 1i64;
        while !stop2.load(std::sync::atomic::Ordering::Relaxed) {
            w.publish(ClockSample { monotonic_ns: n * 1000, frames_written: n as u64 * 128 });
            n += 1;
        }
        n
    });
    let (mut reads, mut last_n) = (0u64, 0i64);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(300);
    while std::time::Instant::now() < deadline {
        if let Some(s) = r.read() {
            assert_eq!(s.frames_written as i64, s.monotonic_ns / 1000 * 128, "torn read: {s:?}");
            assert!(s.monotonic_ns / 1000 >= last_n, "went backwards");
            last_n = s.monotonic_ns / 1000;
            reads += 1;
        }
    }
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let written = writer.join().unwrap();
    assert!(reads > 1000 && written > 1000, "test did not exercise the race: {reads} reads, {written} writes");
}

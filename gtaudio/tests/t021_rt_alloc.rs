//! T-021 (CI part): the RT callback allocates nothing. A counting global
//! allocator, armed per thread around each `callback` call, must see zero
//! allocations. (The hardware part — no input overrun over a 10-minute run —
//! is HW tier.)

use gtaudio::rt::{RtConfig, RtCore};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAlloc;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if ARMED.with(|a| a.get()) {
            ALLOCS.fetch_add(1, Ordering::SeqCst);
        }
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if ARMED.with(|a| a.get()) {
            ALLOCS.fetch_add(1, Ordering::SeqCst);
        }
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if ARMED.with(|a| a.get()) {
            ALLOCS.fetch_add(1, Ordering::SeqCst);
        }
        System.realloc(p, l, n)
    }
}

#[global_allocator]
static A: CountingAlloc = CountingAlloc;

fn count_allocs<R>(f: impl FnOnce() -> R) -> (R, usize) {
    let before = ALLOCS.load(Ordering::SeqCst);
    ARMED.with(|a| a.set(true));
    let r = f();
    ARMED.with(|a| a.set(false));
    (r, ALLOCS.load(Ordering::SeqCst) - before)
}

#[test]
fn the_counting_allocator_actually_counts() {
    let (_v, n) = count_allocs(|| vec![0u8; 1024]);
    assert!(n >= 1, "allocator hook is not wired up; T-021 would pass vacuously");
}

#[test]
fn t021_rt_callback_allocates_nothing() {
    let backing: Vec<f32> = (0..2 * 20_000).map(|i| ((i / 2) as f32 * 0.01).sin() * 0.3).collect();
    let sfx = vec![vec![0.2f32; 3000], vec![-0.2f32; 500]];
    let (mut core, mut handles) = RtCore::new(RtConfig { channels: 2, s0: 4_000, input_ring_samples: 1 << 16 }, backing, sfx);

    let input = vec![0.1f32; 256];
    let mut output = vec![0.0f32; 512];
    let mut drain = vec![0.0f32; 256];

    // Warm up outside the measured region (first-use costs are not RT).
    core.callback(&input, &mut output, 0);

    let mut total_allocs = 0;
    for k in 1..10_000i64 {
        if k % 50 == 0 {
            handles.commands.push_words(&[(k as u32 / 50) % 3]); // includes an out-of-range id
        }
        let ((), n) = count_allocs(|| core.callback(&input, &mut output, k * 5_800_000));
        total_allocs += n;
        // Keep the ring from filling: the DSP thread's job.
        handles.input.pop_samples(&mut drain);
        if k % 100 == 0 {
            assert!(handles.clock.read().is_some());
        }
    }
    assert_eq!(total_allocs, 0, "RT callback performed {total_allocs} allocations/deallocations");
    assert_eq!(core.input_overruns(), 0);
}

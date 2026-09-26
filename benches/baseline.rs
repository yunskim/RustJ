use rustj::{
    Engine, Value,
    kernels::{Op, atomic},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
    time::Instant,
};

// Instrumentation only. Production library forbids unsafe code. Each operation
// forwards the identical pointer/layout to System; counters never allocate.
struct Meter;
static TRACK: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
fn allocated(n: usize) {
    let live = LIVE.fetch_add(n, Relaxed) + n;
    if TRACK.load(Relaxed) {
        ALLOCS.fetch_add(1, Relaxed);
        BYTES.fetch_add(n, Relaxed);
        PEAK.fetch_max(live, Relaxed);
    }
}
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarding the caller's valid allocation layout unchanged.
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            allocated(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Relaxed);
        // SAFETY: this allocator allocated ptr with this layout via System.
        unsafe {
            System.dealloc(ptr, layout);
        }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: forwarding a matching System allocation and valid size.
        let p = unsafe { System.realloc(ptr, layout, size) };
        if !p.is_null() {
            LIVE.fetch_sub(layout.size(), Relaxed);
            allocated(size);
        }
        p
    }
}
#[global_allocator]
static METER: Meter = Meter;

fn memory(f: impl FnOnce()) -> (usize, usize, usize) {
    ALLOCS.store(0, Relaxed);
    BYTES.store(0, Relaxed);
    let baseline = LIVE.load(Relaxed);
    PEAK.store(baseline, Relaxed);
    TRACK.store(true, Relaxed);
    f();
    TRACK.store(false, Relaxed);
    (
        ALLOCS.load(Relaxed),
        BYTES.load(Relaxed),
        PEAK.load(Relaxed).saturating_sub(baseline),
    )
}

fn main() {
    for n in [16usize, 100_000, 1_000_000] {
        let iterations = if n < 100 { 10_000 } else { 20 };
        let mut e = Engine::new();
        e.eval(&format!("a =: i. {n}")).unwrap();
        for _ in 0..5 {
            black_box(e.eval("r =: a + 2").unwrap());
        }
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(e.eval("r =: a + 2").unwrap());
        }
        let micros = start.elapsed().as_secs_f64() * 1e6 / iterations as f64;
        let (allocs, bytes, peak) = memory(|| {
            black_box(e.eval("r =: a + 2").unwrap());
        });
        println!(
            "{{\"workload\":\"bound_add\",\"n\":{n},\"iterations\":{iterations},\"us\":{micros:.3},\"allocations\":{allocs},\"allocated_bytes\":{bytes},\"peak_additional_bytes\":{peak}}}"
        );
        let mut v = Value::ints(vec![n], (0..n as i64).collect()).unwrap();
        let start = Instant::now();
        for _ in 0..iterations {
            v = atomic(Op::Add, v, Value::scalar(2)).unwrap();
            black_box(&v);
        }
        let micros = start.elapsed().as_secs_f64() * 1e6 / iterations as f64;
        let mut result = None;
        let (allocs, bytes, peak) = memory(|| {
            result = Some(atomic(Op::Add, v, Value::scalar(2)).unwrap());
        });
        black_box(result);
        println!(
            "{{\"workload\":\"owned_add\",\"n\":{n},\"iterations\":{iterations},\"us\":{micros:.3},\"allocations\":{allocs},\"allocated_bytes\":{bytes},\"peak_additional_bytes\":{peak}}}"
        );
    }
}

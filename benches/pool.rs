use rustj::Engine;
use std::{hint::black_box, time::Instant};
fn main() {
    for n in [4096, 1_000_000, 4_000_000] {
        let mut engines = [
            Engine::with_output_cache_limit(0),
            Engine::with_output_cache_limit(64 * 1024 * 1024),
        ];
        for e in &mut engines {
            e.eval(&format!("a=:i.{n}")).unwrap();
            for _ in 0..4 {
                e.eval("r=:a+2").unwrap();
            }
        }
        let iterations = if n == 4096 { 10000 } else { 100 };
        let mut times = [Vec::new(), Vec::new()];
        for round in 0..8 {
            for i in if round % 2 == 0 { [0, 1] } else { [1, 0] } {
                let start = Instant::now();
                for _ in 0..iterations {
                    black_box(engines[i].eval("r=:a+2").unwrap());
                }
                times[i].push(start.elapsed().as_secs_f64() * 1e6 / iterations as f64);
            }
        }
        for i in 0..2 {
            times[i].sort_by(f64::total_cmp);
            println!(
                "n={n} cache={} median_us={:.3} retained_bytes={} hits={} rounds_us={:?}",
                i == 1,
                (times[i][3] + times[i][4]) / 2.,
                engines[i].output_cache_stats().0,
                engines[i].output_cache_stats().1,
                times[i]
            );
        }
    }
}

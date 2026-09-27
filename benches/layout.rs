//! Uninstrumented public-API workloads, compiled identically against M1 and M2.
use rustj::{
    Engine, Value,
    kernels::{Op, atomic},
};
use std::{hint::black_box, time::Instant};
fn measure(name: &str, n: usize, iterations: usize, mut f: impl FnMut()) {
    for _ in 0..4 {
        f();
    }
    let mut rounds = Vec::new();
    for _ in 0..7 {
        let start = Instant::now();
        for _ in 0..iterations {
            f();
        }
        rounds.push(start.elapsed().as_secs_f64() * 1e6 / iterations as f64);
    }
    rounds.sort_by(f64::total_cmp);
    println!(
        "{{\"workload\":\"{name}\",\"n\":{n},\"us\":{},\"min_us\":{},\"max_us\":{}}}",
        rounds[3], rounds[0], rounds[6]
    );
}
fn main() {
    measure("scalar_add", 1, 10000, || {
        black_box(atomic(Op::Add, Value::scalar(2), Value::scalar(3)).unwrap());
    });
    for n in [16, 4096, 1000000] {
        let mut v = Some(Value::ints(vec![n], vec![2; n]).unwrap());
        measure("owned_add", n, if n < 100 { 10000 } else { 30 }, || {
            v = Some(atomic(Op::Add, v.take().unwrap(), Value::scalar(2)).unwrap());
            black_box(&v);
        });
    }
    for (name, expr) in [("rank_tally", "r =: #\"1 a"), ("rank_sum", "r =: +/\"1 a")] {
        for rows in [16, 4096] {
            let mut e = Engine::new();
            e.eval(&format!("a =: i. {rows} 8")).unwrap();
            measure(name, rows, if rows < 100 { 1000 } else { 30 }, || {
                black_box(e.eval(expr).unwrap());
            });
        }
    }
}

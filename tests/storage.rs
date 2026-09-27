use rustj::{
    Data, Value,
    kernels::{self, Op},
    storage::{CpuStorage, CpuView},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
};

struct Meter;
thread_local! { static ALLOCS: Cell<Option<usize>> = const { Cell::new(None) }; }
fn record() {
    let _ = ALLOCS.try_with(|n| {
        if let Some(v) = n.get() {
            n.set(Some(v + 1));
        }
    });
}
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        record();
        // SAFETY: forward the requested layout to the matching allocator.
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // SAFETY: all allocations are forwarded unchanged to System.
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        record();
        // SAFETY: the caller supplies a live System allocation and its layout.
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static METER: Meter = Meter;
fn allocations<T>(f: impl FnOnce() -> T) -> (T, usize) {
    ALLOCS.with(|n| n.set(Some(0)));
    let out = f();
    let count = ALLOCS.with(|n| n.replace(None).unwrap());
    (out, count)
}

#[test]
fn ownership_and_allocation_contracts() {
    let (sum, n) =
        allocations(|| kernels::atomic(Op::Add, Value::scalar(2), Value::scalar(3)).unwrap());
    assert_eq!(n, 0);
    assert_eq!(sum.int_at(0).unwrap(), 5);

    let a = Value::ints([1000], (0..1000).collect()).unwrap();
    let Data::Int(v) = a.data() else { panic!() };
    let ptr = v.as_ptr();
    let (a, n) = allocations(|| kernels::atomic(Op::Add, a, Value::scalar(2)).unwrap());
    assert_eq!(n, 0, "unique add must not allocate");
    let Data::Int(v) = a.data() else { panic!() };
    assert_eq!(ptr, v.as_ptr());
    let a = a.into_shared();
    let (alias, n) = allocations(|| a.clone());
    assert_eq!(n, 0);
    let Data::Int(v) = alias.data() else { panic!() };
    assert_eq!(ptr, v.as_ptr(), "shared clone must not copy payload");
    let (changed, n) = allocations(|| kernels::atomic(Op::Add, alias, Value::scalar(2)).unwrap());
    assert_eq!(n, 1, "shared add should allocate only the result payload");
    assert_eq!(a.int_at(0).unwrap(), 2);
    assert_eq!(changed.int_at(0).unwrap(), 4);

    let a = Value::ints([2, 3], (0..6).collect()).unwrap();
    let (cell, n) = allocations(|| a.view().cell(1, 1).unwrap());
    assert_eq!(n, 0);
    assert_eq!(cell.shape(), &[3]);
    let CpuView::Int(v) = cell.data() else {
        panic!()
    };
    let Data::Int(base) = a.data() else { panic!() };
    assert_eq!(v.as_ptr(), base[3..].as_ptr());
    assert!(a.view().cell(1, 2).is_err());
    assert!(a.view().cell(3, 0).is_err());
    let own = cell.to_owned().unwrap();
    drop(a);
    assert_eq!(own.display(), "3 4 5");

    let a = Value::ints([2, 0], vec![]).unwrap();
    assert!(a.view().cell(1, 1).unwrap().is_empty());
    assert!(a.view().cell(1, 2).is_err());
    let a = Value::ints([1, 1, 1, 1, 2], vec![7, 8]).unwrap();
    assert_eq!(a.clone().shape(), &[1, 1, 1, 1, 2]);

    // Rank # only reads cell metadata. More frames must not allocate per cell.
    let mut counts = vec![];
    for rows in [2, 1000] {
        let a = Value::ints([rows, 3], vec![7; rows * 3]).unwrap();
        let (v, n) = allocations(|| kernels::ranked("#", false, 1, a).unwrap());
        assert_eq!(v.shape(), &[rows]);
        assert_eq!(v.int_at(rows - 1).unwrap(), 3);
        assert_eq!(n, 1, "rank tally should allocate only its final payload");
        counts.push(n);
        black_box(v);
    }
    assert_eq!(counts[0], counts[1]);
    let v = CpuStorage::new(vec![1, 2]).into_shared();
    let alias = v.clone();
    assert!(alias.try_into_vec().is_err());
    assert_eq!(v.try_into_vec().unwrap(), vec![1, 2]);
}

#[test]
fn reduction_reuses_accumulator_without_changing_promotion() {
    for rows in [2, 100] {
        let a = Value::ints([rows, 128], vec![3; rows * 128]).unwrap();
        let (out, n) = allocations(|| kernels::reduce("+", a).unwrap());
        assert_eq!(n, 1, "only one accumulator buffer, not one per row");
        assert_eq!(out.int_at(127).unwrap(), (rows * 3) as i64);
    }
    for op in ["+", "-"] {
        for lane in [0, 31, 63, 64, 128] {
            let mut data = vec![2; 3 * 129];
            data[129 + lane] = i64::MAX;
            data[2 * 129 + lane] = if op == "+" { 1 } else { -1 };
            let a = Value::ints([3, 129], data).unwrap();
            let out = kernels::reduce(op, a).unwrap();
            assert_eq!(out.type_code(), 8);
            let middle = if op == "+" {
                i64::MAX as f64 + 1.0
            } else {
                i64::MAX as f64 - (-1.0)
            };
            let expected = if op == "+" {
                2.0 + middle
            } else {
                2.0 - middle
            };
            assert_eq!(out.float_at(lane).unwrap(), expected);
            assert_eq!(
                out.float_at((lane + 1) % 129).unwrap(),
                if op == "+" { 6.0 } else { 2.0 }
            );
        }
    }
}

#[test]
fn streaming_rank_assembly_promotes_earlier_cells_and_preserves_errors() {
    let mut e = rustj::Engine::new();
    let result = e
        .eval("+/\"1 (3 2 $ 2 3 9223372036854775807 1 4 5)")
        .unwrap()
        .unwrap();
    assert_eq!(result.type_code(), 8);
    assert_eq!(result.float_at(0).unwrap(), 5.0);
    assert_eq!(result.float_at(2).unwrap(), 9.0);
    assert_eq!(
        e.eval("i.\"0 (2 3 _9223372036854775808)").unwrap_err(),
        rustj::Error::Limit
    );
    assert!(matches!(
        e.eval("i.\"0 (2 3)"),
        Err(rustj::Error::Unsupported(_))
    ));
    let v = e.eval(",\"1 (2 0 $ 1)").unwrap().unwrap();
    assert_eq!(v.shape(), &[2, 0]);
}

#[test]
fn view_integer_conversion_matches_owned_values() {
    for x in [
        0.0,
        -0.0,
        3.0,
        -3.0,
        3.5,
        f64::NAN,
        f64::INFINITY,
        i64::MIN as f64,
        -(i64::MIN as f64),
    ] {
        let v = Value::new([], Data::Float(CpuStorage::Inline(x))).unwrap();
        assert_eq!(v.int_at(0), v.view().int_at(0));
        assert_eq!(v.view().int_at(1), Err(rustj::Error::Index));
    }
}

#[test]
fn borrowed_reduction_preserves_whole_cell_promotion_and_order() {
    let a = Value::ints([2, 2], vec![i64::MAX, 9, 1, 2]).unwrap();
    let v = kernels::reduce("+", a).unwrap();
    assert_eq!(v.type_code(), 8);
    assert_eq!(v.float_at(0).unwrap(), i64::MAX as f64 + 1.0);
    assert_eq!(v.float_at(1).unwrap(), 11.0);
    let a = Value::ints([2, 3], vec![10, 3, 1, 20, 5, 2]).unwrap();
    assert_eq!(kernels::ranked("-", true, 1, a).unwrap().display(), "8 17");
    assert!(kernels::reduce("?", Value::scalar(3)).is_err());
    let a = Value::new([2], Data::Bool(CpuStorage::new(vec![1, 1]))).unwrap();
    assert_eq!(kernels::reduce("*", a).unwrap().type_code(), 1);
}

#[test]
fn output_cache_saves_allocation_and_preserves_aliases() {
    use rustj::Engine;
    let mut cached = Engine::with_output_cache_limit(32768);
    let mut plain = Engine::with_output_cache_limit(0);
    for e in [&mut cached, &mut plain] {
        for s in ["a=:i.4096", "r=:a+2", "r=:a+2", "r=:a+2"] {
            e.eval(s).unwrap();
        }
    }
    let (_, pooled) = allocations(|| cached.eval("r=:a+2").unwrap());
    let (_, fresh) = allocations(|| plain.eval("r=:a+2").unwrap());
    assert_eq!(pooled + 1, fresh);
    assert!(cached.output_cache_stats().1 > 0);
    assert_eq!(cached.output_cache_stats().0, 32768);
    cached.eval("alias=:r").unwrap();
    let held = cached.eval("r").unwrap().unwrap();
    cached.eval("r=:a+3").unwrap();
    cached.eval("r=:a+4").unwrap();
    assert_eq!(held.int_at(0).unwrap(), 2);
    assert_eq!(
        cached.eval("alias").unwrap().unwrap().int_at(4095).unwrap(),
        4097
    );
    assert!(cached.eval("r=:a+2 3").is_err());
    assert_eq!(cached.eval("r").unwrap().unwrap().int_at(0).unwrap(), 4);
    cached.clear_output_cache();
    assert_eq!(cached.output_cache_stats().0, 0);
}

#[test]
fn output_cache_bounds_shapes_and_overflow() {
    use rustj::Engine;
    let mut e = Engine::with_output_cache_limit(4096);
    for n in [128, 129, 256, 512, 1024, 4096] {
        e.eval(&format!("a=:i.{n}")).unwrap();
        for _ in 0..4 {
            e.eval("r=:a+2").unwrap();
            assert!(e.output_cache_stats().0 <= 4096);
            let v = e.eval("r").unwrap().unwrap();
            assert_eq!(v.int_at(n - 1).unwrap(), n as i64 + 1);
        }
    }
    e.clear_output_cache();
    for s in ["a=:i.129", "r=:a+2", "r=:a+2"] {
        e.eval(s).unwrap();
    }
    let before = e.output_cache_stats().1;
    e.eval("r=:a+9223372036854775807").unwrap();
    assert!(e.output_cache_stats().1 > before);
    let result = e.eval("r").unwrap().unwrap();
    assert!(matches!(result.view().data(), CpuView::Float(_)));
    for i in 0..129 {
        assert_eq!(result.float_at(i).unwrap(), i as f64 + i64::MAX as f64);
    }
    let mut disabled = Engine::with_output_cache_limit(0);
    for s in ["a=:i.4096", "r=:a+2", "r=:a+2", "r=:a+2"] {
        disabled.eval(s).unwrap();
    }
    assert_eq!(disabled.output_cache_stats(), (0, 0));
}

use rustj::{
    Data, Value,
    kernels::{Op, atomic},
};

fn expected(op: Op, a: i64, b: i64) -> Option<i64> {
    match op {
        Op::Add => a.checked_add(b),
        Op::Sub => a.checked_sub(b),
        Op::Mul => a.checked_mul(b),
        _ => unreachable!(),
    }
}
fn real(op: Op, a: i64, b: i64) -> f64 {
    match op {
        Op::Add => a as f64 + b as f64,
        Op::Sub => a as f64 - b as f64,
        Op::Mul => a as f64 * b as f64,
        _ => unreachable!(),
    }
}

#[test]
fn overflow_repair_across_simd_lanes_tails_ownership_and_operand_order() {
    let boundary = [
        i64::MIN,
        i64::MIN + 1,
        -9_007_199_254_740_993,
        -2,
        -1,
        0,
        1,
        2,
        9_007_199_254_740_993,
        i64::MAX - 1,
        i64::MAX,
    ];
    for n in [1, 2, 15, 16, 17, 63, 64, 65, 79, 127, 128, 129, 1025] {
        for op in [Op::Add, Op::Sub, Op::Mul] {
            for &scalar in &boundary {
                for swapped in [false, true] {
                    for shared in [false, true] {
                        let values: Vec<i64> =
                            (0..n).map(|i| boundary[i % boundary.len()]).collect();
                        let array = Value::ints(vec![n], values.clone()).unwrap();
                        let array = if shared { array.into_shared() } else { array };
                        let saved = if shared { Some(array.clone()) } else { None };
                        let pairs: Vec<_> = values
                            .iter()
                            .map(|&x| if swapped { (scalar, x) } else { (x, scalar) })
                            .collect();
                        let got = if swapped {
                            atomic(op, Value::scalar(scalar), array)
                        } else {
                            atomic(op, array, Value::scalar(scalar))
                        }
                        .unwrap();
                        check(op, &pairs, &got);
                        if let Some(saved) = saved {
                            let Data::Int(v) = saved.data() else { panic!() };
                            assert_eq!(v.as_slice(), values);
                        }
                    }
                }
            }
            // Fresh and in-place vector/vector paths, with overflows in every
            // SIMD lane and at the last scalar tail position over these lengths.
            for position in (0..n.min(80)).chain(std::iter::once(n - 1)) {
                let mut a = vec![3; n];
                let mut b = vec![2; n];
                a[position] = i64::MAX;
                b[position] = if matches!(op, Op::Sub) { -2 } else { 2 };
                let pairs: Vec<_> = a.iter().copied().zip(b.iter().copied()).collect();
                for shared in [false, true] {
                    let left = Value::ints(vec![n], a.clone()).unwrap();
                    let left = if shared { left.into_shared() } else { left };
                    let saved = if shared { Some(left.clone()) } else { None };
                    let got = atomic(op, left, Value::ints(vec![n], b.clone()).unwrap()).unwrap();
                    check(op, &pairs, &got);
                    drop(saved);
                }
            }
        }
    }
}

#[test]
fn scalar_multiply_exact_bounds_and_no_overflow_cases() {
    for b in [
        i64::MIN,
        i64::MIN + 1,
        -1_000_003,
        -7,
        -3,
        -1,
        0,
        1,
        2,
        3,
        7,
        1_000_003,
        i64::MAX,
    ] {
        let mut candidates = vec![0, 1, -1];
        if b != 0 {
            for edge in [i64::MIN, i64::MAX] {
                let q = edge as i128 / b as i128;
                for delta in -2..=2 {
                    if let Ok(x) = i64::try_from(q + delta) {
                        candidates.push(x);
                    }
                }
            }
        }
        for x in candidates {
            let pairs = vec![(x, b); 129];
            let got = atomic(
                Op::Mul,
                Value::ints(vec![129], vec![x; 129]).unwrap(),
                Value::scalar(b),
            )
            .unwrap();
            check(Op::Mul, &pairs, &got);
        }
    }
}

#[test]
fn float_simd_matches_scalar_bits_and_signed_zero() {
    use rustj::storage::CpuStorage;
    for n in [63, 64, 65, 127, 128, 129] {
        let values = [
            -0.0,
            0.0,
            0.5,
            -0.5,
            f64::MIN_POSITIVE,
            f64::MAX,
            f64::NEG_INFINITY,
            f64::INFINITY,
        ];
        let x: Vec<_> = (0..n).map(|i| values[i % values.len()]).collect();
        for b in [-0.0, 0.0, 2.5, -2.5] {
            for op in [Op::Add, Op::Sub] {
                let got = atomic(
                    op,
                    Value::new(vec![n], Data::Float(CpuStorage::new(x.clone()))).unwrap(),
                    Value::new(vec![], Data::Float(CpuStorage::new(vec![b]))).unwrap(),
                )
                .unwrap();
                let Data::Float(v) = got.data() else { panic!() };
                for (&a, &actual) in x.iter().zip(v.iter()) {
                    let expected = if matches!(op, Op::Add) { a + b } else { a - b };
                    assert_eq!(actual.to_bits(), expected.to_bits());
                }
            }
        }
    }
}
fn check(op: Op, pairs: &[(i64, i64)], got: &Value) {
    let overflow = pairs.iter().any(|&(a, b)| expected(op, a, b).is_none());
    if overflow {
        let Data::Float(v) = got.data() else {
            panic!("expected float promotion")
        };
        let want: Vec<_> = pairs.iter().map(|&(a, b)| real(op, a, b)).collect();
        assert_eq!(v.as_slice(), want);
    } else {
        let Data::Int(v) = got.data() else {
            panic!("expected integer")
        };
        let want: Vec<_> = pairs
            .iter()
            .map(|&(a, b)| expected(op, a, b).unwrap())
            .collect();
        assert_eq!(v.as_slice(), want);
    }
}

#[test]
fn same_array_on_both_sides_and_empty_expansion() {
    let a = Value::ints(vec![129], vec![i64::MAX; 129]).unwrap();
    let a = a.into_shared();
    let b = a.clone();
    let save = a.clone();
    let out = atomic(Op::Add, a, b).unwrap();
    assert!(matches!(out.data(), Data::Float(_)));
    assert_eq!(save.int_at(0).unwrap(), i64::MAX);
    let out = atomic(
        Op::Add,
        Value::scalar(2),
        Value::ints(vec![0], vec![]).unwrap(),
    )
    .unwrap();
    assert!(out.is_empty());
}

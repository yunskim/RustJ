use rustj::{
    Data, Error, Value,
    physical::{BufferRegistry, Encoding, PhysicalArray},
    storage::{CpuStorage, CpuView},
    types::{DType, Scalar},
};

fn ints(shape: &[usize], values: Vec<i64>) -> Value {
    Value::ints(shape, values).unwrap()
}
fn int(array: &PhysicalArray, index: &[usize]) -> i64 {
    let Scalar::Int(n) = array.atom(index).unwrap() else {
        panic!("integer encoding")
    };
    n
}

#[test]
fn adapters_preserve_payload_and_infer_encoding() {
    let value = ints(&[2, 3], (0..6).collect());
    let Data::Int(data) = value.data() else {
        panic!()
    };
    let ptr = data.as_ptr();
    let mut registry = BufferRegistry::new().unwrap();
    let array = PhysicalArray::from_value(&mut registry, value).unwrap();
    let Some(CpuView::Int(slice)) = array.logical_slice() else {
        panic!()
    };
    assert_eq!(slice.as_ptr(), ptr);
    assert_eq!(slice, &[0, 1, 2, 3, 4, 5]);
    assert_eq!(array.strides(), &[3, 1]);
    assert_eq!(array.dtype(), DType::Int);
    assert_eq!(array.encoding(), Encoding::Int64);
    assert!(array.buffer().address_alignment().unwrap() >= std::mem::align_of::<i64>());
    assert_eq!(int(&array, &[1, 2]), 5);
}

#[test]
fn leases_outlive_registry_and_slot_reuse_rejects_stale_ids() {
    let mut registry = BufferRegistry::new().unwrap();
    let old = registry.register(ints(&[3], vec![7, 8, 9])).unwrap();
    let old_id = old.id();
    let view = PhysicalArray::new(old.clone(), [3], vec![-1], 2).unwrap();
    registry.remove(old_id).unwrap();
    assert_eq!(registry.resolve(old_id).unwrap_err(), Error::Index);
    assert_eq!(registry.remove(old_id).unwrap_err(), Error::Index);
    let new = registry.register(ints(&[3], vec![70, 80, 90])).unwrap();
    assert_ne!(old_id, new.id());
    assert!(!old.shares_backing(&new));
    assert_eq!(registry.resolve(old_id).unwrap_err(), Error::Index);
    let other = BufferRegistry::new().unwrap();
    assert_eq!(other.resolve(new.id()).unwrap_err(), Error::Index);
    drop(new);
    drop(old);
    drop(registry);
    assert_eq!(int(&view, &[0]), 9);
    assert_eq!(int(&view, &[2]), 7);
}

#[test]
fn inline_lease_survives_registry_growth_and_moves() {
    let mut registry = BufferRegistry::new().unwrap();
    let array = PhysicalArray::from_value(&mut registry, Value::scalar(42)).unwrap();
    let Some(CpuView::Int(slice)) = array.logical_slice() else {
        panic!()
    };
    let ptr = slice.as_ptr();
    for i in 0..1024 {
        registry.register(Value::scalar(i)).unwrap();
    }
    let registry = Box::new(registry);
    let array = Box::new(array);
    let Some(CpuView::Int(slice)) = array.logical_slice() else {
        panic!()
    };
    assert_eq!(slice.as_ptr(), ptr);
    assert_eq!(int(&array, &[]), 42);
    drop(registry);
    assert_eq!(int(&array, &[]), 42);
}

#[test]
fn distinct_ids_can_alias_without_copying_payload() {
    let value = ints(&[4], vec![10, 20, 30, 40]).into_shared();
    let mut registry = BufferRegistry::new().unwrap();
    let a = registry.register(value.clone()).unwrap();
    let b = registry.register(value).unwrap();
    assert_ne!(a.id(), b.id());
    assert!(a.shares_backing(&b));
    let reverse = PhysicalArray::new(a, [4], vec![-1], 3).unwrap();
    let overlap = PhysicalArray::new(b, [2, 2], vec![1, 1], 0).unwrap();
    assert_eq!(int(&reverse, &[0]), 40);
    assert_eq!(int(&overlap, &[0, 1]), 20);
    assert_eq!(int(&overlap, &[1, 0]), 20);
    assert!(reverse.logical_slice().is_none());
    assert!(overlap.logical_slice().is_none());
}

#[test]
fn transpose_and_prefix_repeat_preserve_j_logical_mapping() {
    let mut registry = BufferRegistry::new().unwrap();
    let buffer = registry.register(ints(&[2, 3], (0..6).collect())).unwrap();
    let transpose = PhysicalArray::new(buffer, [3, 2], vec![1, 3], 0).unwrap();
    let logical: Vec<_> = (0..3)
        .flat_map(|i| (0..2).map(move |j| [i, j]))
        .map(|i| int(&transpose, &i))
        .collect();
    assert_eq!(logical, vec![0, 3, 1, 4, 2, 5]);
    assert!(transpose.logical_slice().is_none());
    let buffer = registry.register(ints(&[2], vec![10, 20])).unwrap();
    let repeated = PhysicalArray::new(buffer, [2, 3], vec![1, 0], 0).unwrap();
    let logical: Vec<_> = (0..2)
        .flat_map(|i| (0..3).map(move |j| [i, j]))
        .map(|i| int(&repeated, &i))
        .collect();
    assert_eq!(logical, vec![10, 10, 10, 20, 20, 20]);
    assert!(repeated.logical_slice().is_none());
}

#[test]
fn equivalent_logical_array_can_use_different_physical_layouts() {
    let mut registry = BufferRegistry::new().unwrap();

    let standard_buffer = registry
        .register(ints(&[2, 3], vec![0, 1, 2, 3, 4, 5]))
        .unwrap();
    let standard = PhysicalArray::new(standard_buffer, [2, 3], vec![3, 1], 0).unwrap();

    let reversed_backing = registry
        .register(ints(&[2, 3], vec![5, 4, 3, 2, 1, 0]))
        .unwrap();
    let reversed_layout = PhysicalArray::new(reversed_backing, [2, 3], vec![-3, -1], 5).unwrap();

    let standard_logical = (0..2)
        .flat_map(|i| (0..3).map(move |j| [i, j]))
        .map(|index| int(&standard, &index))
        .collect::<Vec<_>>();
    let reversed_logical = (0..2)
        .flat_map(|i| (0..3).map(move |j| [i, j]))
        .map(|index| int(&reversed_layout, &index))
        .collect::<Vec<_>>();

    assert_eq!(standard.shape(), reversed_layout.shape());
    assert_ne!(standard.strides(), reversed_layout.strides());
    assert_ne!(standard.offset(), reversed_layout.offset());
    assert_eq!(standard_logical, vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(reversed_logical, standard_logical);
    assert!(standard.logical_slice().is_some());
    assert!(reversed_layout.logical_slice().is_none());
}

#[test]
fn empty_and_singleton_axes_have_canonical_metadata() {
    let mut registry = BufferRegistry::new().unwrap();
    let empty = registry.register(ints(&[0], vec![])).unwrap();
    let array = PhysicalArray::new(
        empty,
        [0, usize::MAX, usize::MAX],
        vec![isize::MIN, isize::MAX, -1],
        isize::MIN,
    )
    .unwrap();
    assert_eq!(array.shape(), &[0, usize::MAX, usize::MAX]);
    assert_eq!(array.offset(), 0);
    assert_eq!(array.strides(), &[0, 0, 0]);
    assert!(array.is_empty());
    assert!(array.buffer().address_alignment().is_none());
    assert!(matches!(array.logical_slice(),Some(CpuView::Int(v)) if v.is_empty()));
    assert_eq!(array.atom(&[0, 0, 0]).unwrap_err(), Error::Index);
    let buffer = registry.register(Value::scalar(99)).unwrap();
    let singleton = PhysicalArray::new(buffer, [1, 1], vec![isize::MIN, isize::MAX], 0).unwrap();
    assert_eq!(singleton.strides(), &[0, 0]);
    assert!(singleton.is_standard_layout());
    assert_eq!(int(&singleton, &[0, 0]), 99);
}

#[test]
fn scalar_and_index_boundaries_are_checked() {
    let mut registry = BufferRegistry::new().unwrap();
    let empty = registry.register(ints(&[0], vec![])).unwrap();
    assert_eq!(
        PhysicalArray::new(empty, [], vec![], 0).unwrap_err(),
        Error::Index
    );
    let buffer = registry.register(ints(&[4], vec![1, 2, 3, 4])).unwrap();
    let scalar = PhysicalArray::new(buffer.clone(), [], vec![], 3).unwrap();
    assert_eq!(int(&scalar, &[]), 4);
    let slice = PhysicalArray::new(buffer, [2], vec![1], 1).unwrap();
    assert!(matches!(slice.logical_slice(),Some(CpuView::Int(v)) if v == [2,3]));
    assert_eq!(slice.atom(&[]).unwrap_err(), Error::Rank);
    assert_eq!(slice.atom(&[2]).unwrap_err(), Error::Index);
    assert_eq!(scalar.atom(&[0]).unwrap_err(), Error::Rank);
}

#[test]
fn invalid_span_and_arithmetic_fail_before_access() {
    let mut registry = BufferRegistry::new().unwrap();
    let buffer = registry.register(ints(&[4], vec![1, 2, 3, 4])).unwrap();
    for (shape, strides, offset, error) in [
        (vec![2], vec![], 0, Error::Rank),
        (vec![2], vec![-1], 0, Error::Index),
        (vec![2], vec![1], 3, Error::Index),
        (vec![], vec![], -1, Error::Index),
        (vec![3], vec![isize::MAX], 0, Error::Limit),
        (vec![2], vec![1], isize::MAX, Error::Limit),
        (vec![usize::MAX, 2], vec![0, 0], 0, Error::Limit),
    ] {
        assert_eq!(
            PhysicalArray::new(buffer.clone(), shape, strides, offset).unwrap_err(),
            error
        );
    }
}

#[test]
fn exhaustive_small_strides_agree_with_independent_address_enumeration() {
    let mut registry = BufferRegistry::new().unwrap();
    let buffer = registry.register(ints(&[8], (0..8).collect())).unwrap();
    for rows in 1..=3 {
        for cols in 1..=3 {
            for row_stride in -3..=3 {
                for col_stride in -3..=3 {
                    for offset in -2..=10 {
                        let indices: Vec<_> = (0..rows)
                            .flat_map(|i| (0..cols).map(move |j| [i, j]))
                            .collect();
                        let addresses: Vec<i64> = indices
                            .iter()
                            .map(|i| {
                                offset as i64
                                    + i[0] as i64 * row_stride as i64
                                    + i[1] as i64 * col_stride as i64
                            })
                            .collect();
                        let valid = addresses.iter().all(|&i| (0..8).contains(&i));
                        match PhysicalArray::new(
                            buffer.clone(),
                            [rows, cols],
                            vec![row_stride, col_stride],
                            offset,
                        ) {
                            Ok(array) => {
                                assert!(valid);
                                for (index, address) in indices.iter().zip(addresses) {
                                    assert_eq!(int(&array, index), address);
                                }
                            }
                            Err(_) => assert!(!valid),
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn all_supported_encodings_read_without_dtype_reinterpretation() {
    let mut registry = BufferRegistry::new().unwrap();
    let cases = [
        (
            Value::new([2], Data::Bool(CpuStorage::new(vec![0, 1]))).unwrap(),
            Encoding::BoolByte,
        ),
        (
            Value::new([2], Data::Float(CpuStorage::new(vec![-0.0, f64::INFINITY]))).unwrap(),
            Encoding::Float64,
        ),
        (
            Value::new([2], Data::Char(CpuStorage::new(vec![b'a', b'z']))).unwrap(),
            Encoding::Char8,
        ),
    ];
    for (value, encoding) in cases {
        let array = PhysicalArray::from_value(&mut registry, value).unwrap();
        assert_eq!(array.encoding(), encoding);
        match array.atom(&[0]).unwrap() {
            Scalar::Bool(x) => assert!(!x),
            Scalar::Float(x) => assert_eq!(x.to_bits(), (-0.0f64).to_bits()),
            Scalar::Char(x) => assert_eq!(x, b'a'),
            _ => panic!(),
        }
    }
}

#[test]
fn non_affine_payloads_are_explicitly_rejected() {
    let mut registry = BufferRegistry::new().unwrap();
    let boxed = Value::boxed(Value::scalar(3));
    assert!(matches!(
        registry.register(boxed),
        Err(Error::Unsupported(_))
    ));
    let sparse = rustj::Engine::new().eval("$. i.3").unwrap().unwrap();
    assert!(matches!(
        PhysicalArray::from_value(&mut registry, sparse),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn high_rank_adapter_preserves_shape_and_sharing() {
    let shape = [1, 1, 2, 1, 3, 1];
    let value = ints(&shape, (0..6).collect()).into_shared();
    let mut registry = BufferRegistry::new().unwrap();
    let a = PhysicalArray::from_value(&mut registry, value.clone()).unwrap();
    let b = PhysicalArray::from_value(&mut registry, value).unwrap();
    assert_eq!(a.shape(), &shape);
    assert_eq!(a.strides(), &[0, 0, 3, 0, 1, 0]);
    assert!(a.buffer().shares_backing(b.buffer()));
    assert_eq!(int(&a, &[0, 0, 1, 0, 2, 0]), 5);
}

#[test]
fn byte_encodings_detect_shared_allocation_even_across_dtype() {
    let bytes = std::sync::Arc::new(vec![0u8, 1]);
    let bools = Value::new([2], Data::Bool(CpuStorage::Shared(bytes.clone()))).unwrap();
    let chars = Value::new([2], Data::Char(CpuStorage::Shared(bytes))).unwrap();
    let mut registry = BufferRegistry::new().unwrap();
    let a = registry.register(bools).unwrap();
    let b = registry.register(chars).unwrap();
    assert_ne!(a.encoding(), b.encoding());
    assert!(a.shares_backing(&b));
}

#[test]
fn partial_view_reports_retained_capacity_and_actual_offset_alignment() {
    let mut values = Vec::with_capacity(64);
    values.extend([1i64, 2, 3, 4]);
    let expected = values.capacity() * std::mem::size_of::<i64>();
    let mut registry = BufferRegistry::new().unwrap();
    let buffer = registry.register(ints(&[4], values)).unwrap();
    let one = PhysicalArray::new(buffer, [1], vec![0], 1).unwrap();
    assert_eq!(one.buffer().retained_payload_bytes(), expected);
    let Some(CpuView::Int(slice)) = one.logical_slice() else {
        panic!()
    };
    let address = slice.as_ptr() as usize;
    assert_eq!(
        one.address_alignment(),
        Some(1usize << address.trailing_zeros())
    );
    assert_eq!(int(&one, &[0]), 2);
}

#[test]
fn physical_search_planner_uses_registered_routes_with_runtime_guards() {
    use rustj::{
        logical_ir::SearchOutputKind as O,
        lowering::{SearchAlgorithm as A, TargetCapabilities},
        physical::{SearchSelectionBasis as B, SearchWorkload, plan_search_algorithm},
    };
    let cpu = TargetCapabilities::cpu_baseline();
    let mut facts = SearchWorkload {
        indexed_items: 5,
        query_items: 7,
        integer_span: Some(4),
        immutable_shared_index: false,
        prehash_available: false,
        allow_reverse: true,
    };
    let chosen = plan_search_algorithm(O::FirstIndex, &cpu, facts, true);
    assert_eq!(chosen.algorithm, A::DirectAddress);
    assert_eq!(chosen.basis, B::RuntimeExactScalarGuard);
    assert_eq!(chosen.estimated_table_entries, 4);

    // Target-matched candidate without an exact scalar runtime witness
    // cannot become a selected implementation.
    let withheld = plan_search_algorithm(O::FirstIndex, &cpu, facts, false);
    assert_eq!(withheld.algorithm, A::Sequential);
    assert_eq!(withheld.basis, B::Reference);

    facts.indexed_items = 100;
    facts.query_items = 3;
    facts.integer_span = None;
    assert_eq!(
        plan_search_algorithm(O::LastIndex, &cpu, facts, true).algorithm,
        A::ReverseQueryHash,
    );
    facts.allow_reverse = false;
    assert_eq!(
        plan_search_algorithm(O::LastIndex, &cpu, facts, true).algorithm,
        A::IndexedHash,
    );
    facts.immutable_shared_index = true;
    facts.prehash_available = true;
    assert_eq!(
        plan_search_algorithm(O::MembershipMask, &cpu, facts, true).algorithm,
        A::PreparedHash,
    );
    facts.indexed_items = 3;
    facts.query_items = 2;
    assert_eq!(
        plan_search_algorithm(O::FirstIndex, &cpu, facts, true).algorithm,
        A::Sequential,
    );

    assert_eq!(
        plan_search_algorithm(O::IntervalIndex, &cpu, facts, true).basis,
        B::Unavailable,
    );
    assert_eq!(
        plan_search_algorithm(
            O::FirstIndex,
            &TargetCapabilities::gpu_generic(),
            facts,
            true
        )
        .basis,
        B::Unavailable,
    );
}

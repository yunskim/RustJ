use rustj::{Error, Value, sparse::SparseArray};

#[test]
fn arbitrary_sparse_axes_preserve_dense_cell_order_and_nonzero_fill() {
    let array = SparseArray::new(
        [2, 3, 2],
        vec![0, 2],
        vec![0, 1, 1, 0],
        Value::scalar(9),
        Value::ints([2, 3], vec![1, 2, 3, 4, 5, 6]).unwrap(),
    )
    .unwrap();
    let dense = array.to_dense(12).unwrap();
    assert_eq!(
        dense.json(),
        Value::ints([2, 3, 2], vec![9, 1, 9, 2, 9, 3, 4, 9, 5, 9, 6, 9])
            .unwrap()
            .json()
    );
    assert_eq!(array.stored_rows(), 2);
    assert!(matches!(array.to_dense(11), Err(Error::Limit)));
    assert_eq!(array.clone().to_dense(12).unwrap().json(), dense.json());
}

#[test]
fn sparse_construction_rejects_invalid_coordinate_invariants() {
    for coords in [vec![1, 1], vec![2, 0]] {
        assert!(matches!(
            SparseArray::new(
                [3],
                vec![0],
                coords,
                Value::scalar(0),
                Value::ints([2], vec![4, 5]).unwrap()
            ),
            Err(Error::Domain)
        ));
    }
    assert!(matches!(
        SparseArray::new(
            [3],
            vec![0],
            vec![3],
            Value::scalar(0),
            Value::ints([1], vec![4]).unwrap()
        ),
        Err(Error::Index)
    ));
    assert!(matches!(
        SparseArray::new(
            [2, 3],
            vec![1, 0],
            vec![],
            Value::scalar(0),
            Value::ints([0], vec![]).unwrap()
        ),
        Err(Error::Domain)
    ));
    assert!(matches!(
        SparseArray::new(
            [2, 3],
            vec![0],
            vec![0],
            Value::scalar(0),
            Value::ints([1, 2], vec![1, 2]).unwrap()
        ),
        Err(Error::Length)
    ));
}

#[test]
fn huge_logical_arrays_do_not_allocate_dense_storage() {
    let array = SparseArray::new(
        [usize::MAX, 2],
        vec![0, 1],
        vec![0, 0],
        Value::scalar(7),
        Value::ints([1], vec![8]).unwrap(),
    )
    .unwrap();
    assert_eq!(array.values().len(), 1);
    assert!(matches!(array.to_dense(1024), Err(Error::Limit)));
    let all_fill = SparseArray::new(
        [2, 3],
        vec![0, 1],
        vec![],
        Value::scalar(7),
        Value::ints([0], vec![]).unwrap(),
    )
    .unwrap();
    assert_eq!(
        all_fill.to_dense(6).unwrap().json(),
        Value::ints([2, 3], vec![7; 6]).unwrap().json()
    );
    let scalar = SparseArray::new(
        [],
        vec![],
        vec![],
        Value::scalar(0),
        Value::ints([1], vec![42]).unwrap(),
    )
    .unwrap();
    assert_eq!(scalar.to_dense(1).unwrap().int_at(0).unwrap(), 42);
}

#[test]
fn zero_axes_and_non_integer_fills_preserve_types() {
    use rustj::{Data, storage::CpuStorage};
    let empty = SparseArray::new(
        [usize::MAX, 2, 0],
        vec![0, 1, 2],
        vec![],
        Value::scalar(0),
        Value::ints([0], vec![]).unwrap(),
    )
    .unwrap();
    assert_eq!(empty.to_dense(0).unwrap().shape(), &[usize::MAX, 2, 0]);
    let fill = Value::new([], Data::Float(CpuStorage::Inline(-0.0))).unwrap();
    let values = Value::new([1], Data::Float(CpuStorage::Inline(1.5))).unwrap();
    let sparse = SparseArray::new([3], vec![0], vec![1], fill, values).unwrap();
    let dense = sparse.to_dense(3).unwrap();
    assert_eq!(dense.float_at(0).unwrap().to_bits(), (-0.0f64).to_bits());
    assert_eq!(dense.float_at(1).unwrap(), 1.5);
    let fill = Value::new([], Data::Char(CpuStorage::Inline(b' '))).unwrap();
    let values = Value::new([1], Data::Char(CpuStorage::Inline(b'x'))).unwrap();
    assert_eq!(
        SparseArray::new([3], vec![0], vec![1], fill, values)
            .unwrap()
            .to_dense(3)
            .unwrap()
            .display(),
        " x "
    );
}

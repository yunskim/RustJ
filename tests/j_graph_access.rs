use rustj::j_graph_access::{AxisAccessError, StaticAxisAccess};

#[test]
fn axis_permutation_composition_matches_two_stage_pullback() {
    let first = StaticAxisAccess::new(vec![2, 3, 4], vec![1, 2, 0]).unwrap();
    let second = StaticAxisAccess::new(vec![3, 4, 2], vec![1, 0, 2]).unwrap();
    let composed = first.compose_after(&second).unwrap();

    assert_eq!(composed.input_shape(), &[2, 3, 4]);
    assert_eq!(composed.output_shape(), vec![4, 3, 2]);
    for i in 0..4 {
        for j in 0..3 {
            for k in 0..2 {
                let output = [i, j, k];
                let middle = second.pullback_point(&output).unwrap();
                assert_eq!(
                    composed.pullback_point(&output).unwrap(),
                    first.pullback_point(&middle).unwrap()
                );
                assert_eq!(composed.pullback_point(&output).unwrap(), vec![k, j, i]);
            }
        }
    }
}

#[test]
fn axis_access_rejects_unproved_shape_and_index_relations() {
    assert_eq!(
        StaticAxisAccess::new(vec![2, 3], vec![0]).unwrap_err(),
        AxisAccessError::RankMismatch
    );
    assert_eq!(
        StaticAxisAccess::new(vec![2, 3], vec![1, 1]).unwrap_err(),
        AxisAccessError::NotPermutation
    );
    assert_eq!(
        StaticAxisAccess::new(vec![2, 3], vec![0, 2]).unwrap_err(),
        AxisAccessError::NotPermutation
    );

    let first = StaticAxisAccess::new(vec![2, 3], vec![1, 0]).unwrap();
    let incompatible = StaticAxisAccess::new(vec![2, 3], vec![0, 1]).unwrap();
    assert_eq!(
        first.compose_after(&incompatible).unwrap_err(),
        AxisAccessError::IntermediateShapeMismatch
    );
    assert_eq!(
        first.pullback_point(&[3, 0]).unwrap_err(),
        AxisAccessError::CoordinateOutOfBounds
    );
    assert_eq!(
        first.pullback_point(&[0]).unwrap_err(),
        AxisAccessError::CoordinateOutOfBounds
    );
}

#[test]
fn axis_composition_preserves_scalar_and_zero_extent_shapes() {
    let scalar = StaticAxisAccess::new(vec![], vec![]).unwrap();
    let scalar_twice = scalar.compose_after(&scalar).unwrap();
    assert_eq!(scalar_twice.output_shape(), Vec::<usize>::new());
    assert_eq!(scalar_twice.pullback_point(&[]).unwrap(), Vec::<usize>::new());

    let first = StaticAxisAccess::new(vec![0, 2], vec![1, 0]).unwrap();
    let second = StaticAxisAccess::new(vec![2, 0], vec![1, 0]).unwrap();
    let composed = first.compose_after(&second).unwrap();
    assert_eq!(composed.input_shape(), &[0, 2]);
    assert_eq!(composed.output_shape(), vec![0, 2]);
    assert_eq!(
        composed.pullback_point(&[0, 0]).unwrap_err(),
        AxisAccessError::CoordinateOutOfBounds
    );
    // An empty value demand cannot erase J prototype/fill/assembly obligations.
}

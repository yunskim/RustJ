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
    assert_eq!(
        scalar_twice.pullback_point(&[]).unwrap(),
        Vec::<usize>::new()
    );

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

#[test]
fn axis_region_pullback_matches_composition_and_exact_requested_points() {
    let first = StaticAxisAccess::new(vec![2, 3, 4], vec![1, 2, 0]).unwrap();
    let second = StaticAxisAccess::new(vec![3, 4, 2], vec![1, 0, 2]).unwrap();
    let composed = first.compose_after(&second).unwrap();
    let requested = [1..3, 0..2, 0..2];
    let input = composed.pullback_region(&requested).unwrap();

    assert_eq!(input, vec![0..2, 0..2, 1..3]);
    let intermediate = second.pullback_region(&requested).unwrap();
    assert_eq!(input, first.pullback_region(&intermediate).unwrap());

    for i in 0..4 {
        for j in 0..3 {
            for k in 0..2 {
                let output = [i, j, k];
                let input_point = composed.pullback_point(&output).unwrap();
                let selected_output = requested
                    .iter()
                    .zip(output)
                    .all(|(range, coordinate)| range.contains(&coordinate));
                let selected_input = input
                    .iter()
                    .zip(input_point)
                    .all(|(range, coordinate)| range.contains(&coordinate));
                assert_eq!(selected_input, selected_output);
            }
        }
    }

    assert_eq!(
        composed.pullback_region(&[1..1, 0..2, 0..2]).unwrap(),
        vec![0..2, 0..2, 1..1]
    );
    let rank_one_region = 0..1;
    assert_eq!(
        composed
            .pullback_region(std::slice::from_ref(&rank_one_region))
            .unwrap_err(),
        AxisAccessError::RankMismatch
    );
    assert_eq!(
        composed.pullback_region(&[0..5, 0..2, 0..2]).unwrap_err(),
        AxisAccessError::RegionOutOfBounds
    );
    let descending = std::ops::Range { start: 3, end: 2 };
    assert_eq!(
        composed
            .pullback_region(&[descending, 0..2, 0..2])
            .unwrap_err(),
        AxisAccessError::RegionOutOfBounds
    );
}

#[test]
fn graph_monadic_transpose_projects_witnessed_axis_access() {
    let mut engine = rustj::Engine::new();
    engine.eval("a=: i. 2 3").unwrap();
    let graph = engine.analyze_j_graph("|: a").unwrap();
    graph.verify().unwrap();

    let output = graph.result.unwrap();
    let projected = graph.static_transpose_access(output).unwrap();
    assert_eq!(projected.output, output);
    assert_eq!(projected.access.input_shape(), &[2, 3]);
    assert_eq!(projected.access.output_shape(), vec![3, 2]);
    assert_eq!(
        projected.access.pullback_point(&[2, 1]).unwrap(),
        vec![1, 2]
    );
    assert_eq!(
        projected.access.pullback_region(&[1..3, 0..2]).unwrap(),
        vec![0..2, 1..3]
    );

    let graph = engine.analyze_j_graph("|: |: a").unwrap();
    graph.verify().unwrap();
    let outer = graph
        .static_transpose_access(graph.result.unwrap())
        .unwrap();
    let inner = graph.static_transpose_access(outer.input).unwrap();
    let composed = inner.access.compose_after(&outer.access).unwrap();
    assert_eq!(composed.input_shape(), &[2, 3]);
    assert_eq!(composed.output_shape(), vec![2, 3]);
    assert_eq!(
        composed.pullback_region(&[0..2, 1..3]).unwrap(),
        vec![0..2, 1..3]
    );
}

#[test]
fn graph_transpose_access_refuses_unknown_and_unrelated_reindex_facts() {
    let mut engine = rustj::Engine::new();
    engine.eval("a=: i. 2 3").unwrap();
    for source in ["|. a", ", a", "a"] {
        let graph = engine.analyze_j_graph(source).unwrap();
        assert!(
            graph
                .static_transpose_access(graph.result.unwrap())
                .is_none(),
            "{source} must not acquire a transpose access witness"
        );
    }

    let graph = engine.analyze_j_graph("|: a").unwrap();
    let output = graph.result.unwrap();
    let access = graph.static_transpose_access(output).unwrap();

    let mut unknown = graph.clone();
    unknown.nodes[access.input.0].facts.shape = None;
    assert!(unknown.static_transpose_access(output).is_none());

    let mut wrong_shape = graph.clone();
    wrong_shape.nodes[output.0].facts.shape = Some(vec![2, 3]);
    assert!(wrong_shape.static_transpose_access(output).is_none());

    let mut wrong_rank = graph;
    wrong_rank.nodes[output.0].facts.rank = Some(1);
    assert!(wrong_rank.static_transpose_access(output).is_none());
}

#[test]
fn graph_monadic_map_and_transpose_share_region_pullback() {
    use rustj::j_graph_access::GraphValueAccessKind;

    let mut engine = rustj::Engine::new();
    engine.eval("a=: i. 2 3").unwrap();

    let graph = engine.analyze_j_graph("|: - a").unwrap();
    graph.verify().unwrap();
    let output = graph.result.unwrap();
    let transpose = graph.static_value_access(output).unwrap();
    assert_eq!(transpose.kind, GraphValueAccessKind::MonadicTranspose);
    assert_eq!(transpose.output, output);

    let map = graph.static_value_access(transpose.input).unwrap();
    assert_eq!(map.kind, GraphValueAccessKind::UnaryMap);
    assert_eq!(map.output, transpose.input);
    assert_eq!(map.access.input_shape(), &[2, 3]);
    assert_eq!(map.access.output_shape(), vec![2, 3]);

    let requested = [1..3, 0..2];
    let map_result_box = transpose.access.pullback_region(&requested).unwrap();
    assert_eq!(map_result_box, vec![0..2, 1..3]);
    assert_eq!(
        map.access.pullback_region(&map_result_box).unwrap(),
        map_result_box
    );

    let composed = map.access.compose_after(&transpose.access).unwrap();
    assert_eq!(
        composed.pullback_region(&requested).unwrap(),
        map.access.pullback_region(&map_result_box).unwrap()
    );
    assert_eq!(composed.pullback_point(&[2, 1]).unwrap(), vec![1, 2]);

    // The same transfer handles empty logical value demand, but does not
    // authorize skipping J prototype or error work outside this demand.
    assert_eq!(
        composed.pullback_region(&[1..1, 0..2]).unwrap(),
        vec![0..2, 1..1]
    );
}

#[test]
fn graph_unary_map_access_rejects_unsupported_calls_and_unknown_facts() {
    use rustj::j_graph_access::GraphValueAccessKind;

    let mut engine = rustj::Engine::new();
    engine.eval("a=: i. 2 3").unwrap();

    for source in ["1 + a", "|. a", ", a", "a", "1 { a", "+/ a", "-\"1 a"] {
        let graph = engine.analyze_j_graph(source).unwrap();
        assert!(
            graph.static_value_access(graph.result.unwrap()).is_none(),
            "{source} must not acquire an unwitnessed unary Map/transpose access"
        );
    }

    let graph = engine.analyze_j_graph("- a").unwrap();
    let result = graph.result.unwrap();
    let witness = graph.static_value_access(result).unwrap();
    assert_eq!(witness.kind, GraphValueAccessKind::UnaryMap);

    let mut missing = graph.clone();
    missing.nodes[witness.input.0].facts.shape = None;
    assert!(missing.static_value_access(result).is_none());

    let mut changed_shape = graph.clone();
    changed_shape.nodes[result.0].facts.shape = Some(vec![3, 2]);
    assert!(changed_shape.static_value_access(result).is_none());

    let mut bad_rank = graph.clone();
    bad_rank.nodes[result.0].facts.rank = Some(1);
    assert!(bad_rank.static_value_access(result).is_none());

    // Classified operation kind alone is not an eligibility proof.
    let mut bad_access_contract = graph;
    if let rustj::j_graph_ir::NodeKind::Apply { contract, .. } =
        &mut bad_access_contract.nodes[result.0].kind
    {
        contract.access = rustj::j_graph_ir::AccessContract::Unknown;
    }
    assert!(bad_access_contract.static_value_access(result).is_none());
}

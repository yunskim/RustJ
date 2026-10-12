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

#[test]
fn graph_value_demand_closure_across_witnessed_chain() {
    use rustj::{j_graph_access::GraphDemandStop, j_graph_ir::NodeKind};

    let mut engine = rustj::Engine::new();
    engine.eval("a=: i. 2 3").unwrap();
    let graph = engine.analyze_j_graph("- |: - a").unwrap();
    graph.verify().unwrap();
    let output = graph.result.unwrap();
    let requested = [1..3, 0..2];
    let trace = graph.value_demand_trace(output, &requested).unwrap();

    assert_eq!(trace.requested_output, output);
    assert_eq!(trace.stop, GraphDemandStop::Source);
    assert_eq!(trace.steps.len(), 4);
    assert_eq!(trace.steps[0].value, output);
    assert_eq!(trace.steps[0].region, vec![1..3, 0..2]);
    assert_eq!(trace.steps[1].region, vec![1..3, 0..2]);
    assert_eq!(trace.steps[2].region, vec![0..2, 1..3]);
    assert_eq!(trace.steps[3].region, vec![0..2, 1..3]);
    assert!(matches!(
        &graph.nodes[trace.steps[3].value.0].kind,
        NodeKind::ReadNoun { name, .. } if name == "a"
    ));

    let empty = graph.value_demand_trace(output, &[1..1, 0..2]).unwrap();
    assert_eq!(empty.stop, GraphDemandStop::Source);
    assert_eq!(empty.steps[3].region, vec![0..2, 1..1]);

    let scalar = rustj::Engine::new().analyze_j_graph("- 3").unwrap();
    let scalar_trace = scalar
        .value_demand_trace(scalar.result.unwrap(), &[])
        .unwrap();
    assert_eq!(scalar_trace.stop, GraphDemandStop::Source);
    assert_eq!(scalar_trace.steps.len(), 2);
    assert!(scalar_trace.steps.iter().all(|step| step.region.is_empty()));
}

#[test]
fn graph_value_demand_stops_at_unproved_operation_and_invalid_box() {
    use rustj::{j_graph_access::GraphDemandStop, j_graph_ir::NodeKind};

    let mut engine = rustj::Engine::new();
    engine.eval("a=: i. 2 3").unwrap();
    let graph = engine.analyze_j_graph("- |. a").unwrap();
    graph.verify().unwrap();
    let output = graph.result.unwrap();
    let trace = graph.value_demand_trace(output, &[0..2, 1..3]).unwrap();
    assert_eq!(trace.stop, GraphDemandStop::UnprovedOperation);
    assert_eq!(trace.steps.len(), 2);
    assert_eq!(trace.steps[1].region, vec![0..2, 1..3]);
    assert!(matches!(
        &graph.nodes[trace.steps[1].value.0].kind,
        NodeKind::Apply { .. }
    ));

    for source in ["1 + a", "+/ a", "-\"1 a"] {
        let unsupported = engine.analyze_j_graph(source).unwrap();
        let result = unsupported.result.unwrap();
        let shape = unsupported.nodes[result.0].facts.shape.as_ref().unwrap();
        let full: Vec<_> = shape.iter().map(|&extent| 0..extent).collect();
        let trace = unsupported.value_demand_trace(result, &full).unwrap();
        assert_eq!(trace.stop, GraphDemandStop::UnprovedOperation);
        assert_eq!(trace.steps.len(), 1);
    }

    assert!(graph.value_demand_trace(output, &[0..4, 0..2]).is_none());
    let wrong_rank = 0..2;
    assert!(
        graph
            .value_demand_trace(output, std::slice::from_ref(&wrong_rank))
            .is_none()
    );
    let descending = std::ops::Range { start: 2, end: 1 };
    assert!(
        graph
            .value_demand_trace(output, &[descending, 0..2])
            .is_none()
    );

    let mut unknown_shape = graph;
    unknown_shape.nodes[output.0].facts.shape = None;
    assert!(
        unknown_shape
            .value_demand_trace(output, &[0..2, 0..3])
            .is_none()
    );
}

#[test]
fn graph_fanout_demands_remain_conservative_when_boxes_are_disjoint() {
    use rustj::j_graph_ir::RegionKind;

    let mut engine = rustj::Engine::new();
    engine.eval("a=: i. 2 3").unwrap();
    let graph = engine.analyze_j_graph("(|: + |:) a").unwrap();
    graph.verify().unwrap();

    let result = graph.result.unwrap();
    let (_, fork) = graph.region_for_result(result).unwrap();
    let RegionKind::Fork { branch_results, .. } = &fork.kind else {
        panic!("expected fork with two separate transpose consumers")
    };
    assert_eq!(branch_results.len(), 2);
    let source = fork.inputs[0];
    assert!(graph.use_counts()[source.0] >= 2);
    for &branch in branch_results {
        assert_eq!(graph.static_value_access(branch).unwrap().input, source);
    }

    let requests = [
        (branch_results[0], vec![0..1, 0..2]),
        (branch_results[1], vec![1..3, 1..2]),
    ];
    let union = graph
        .witnessed_value_demand_union(source, &requests)
        .unwrap();
    assert_eq!(union.value, source);
    assert_eq!(union.boxes, vec![vec![0..2, 0..1], vec![1..2, 1..3]]);

    // The bounding rectangle 0..2 × 0..3 would include (0,2).
    // An exact union must not invent that requested coordinate.
    for row in 0..2 {
        for column in 0..3 {
            assert_eq!(union.contains(&[row, column]), column == 0 || row == 1);
        }
    }
    assert!(!union.contains(&[0, 2]));

    let empty_demand = [(branch_results[0], vec![1..1, 0..2])];
    let empty = graph
        .witnessed_value_demand_union(source, &empty_demand)
        .unwrap();
    assert_eq!(empty.boxes, vec![vec![0..2, 1..1]]);
    assert!(!empty.contains(&[0, 1]));
}

#[test]
fn graph_fanout_demand_union_rejects_unproved_join_and_invalid_requests() {
    use rustj::j_graph_ir::RegionKind;

    let mut engine = rustj::Engine::new();
    engine.eval("a=: i. 2 3").unwrap();
    let graph = engine.analyze_j_graph("(|: + |:) a").unwrap();
    let result = graph.result.unwrap();
    let (_, fork) = graph.region_for_result(result).unwrap();
    let RegionKind::Fork { branch_results, .. } = &fork.kind else {
        panic!("expected fork")
    };
    let source = fork.inputs[0];

    // A dyadic join does not inherit the unary branch access witnesses.
    assert!(
        graph
            .witnessed_value_demand_union(source, &[(result, vec![0..3, 0..2])])
            .is_none()
    );
    assert!(
        graph
            .witnessed_value_demand_union(result, &[(branch_results[0], vec![0..1, 0..2])])
            .is_none()
    );
    assert!(graph.witnessed_value_demand_union(source, &[]).is_none());
    assert!(
        graph
            .witnessed_value_demand_union(source, &[(branch_results[0], vec![0..4, 0..2])])
            .is_none()
    );
    let invalid_rank = 0..2;
    assert!(
        graph
            .witnessed_value_demand_union(source, &[(branch_results[0], vec![invalid_rank])])
            .is_none()
    );
}

#[test]
fn graph_fanout_consumer_occurrences_match_original_edges() {
    use rustj::j_graph_access::{GraphUseOccurrence, GraphUseOperand};
    use rustj::j_graph_ir::{RegionKind, ValueId};

    let mut engine = rustj::Engine::new();
    engine.eval("a=: i. 2 3").unwrap();

    let fork = engine.analyze_j_graph("(|: + |:) a").unwrap();
    fork.verify().unwrap();
    let (_, region) = fork.region_for_result(fork.result.unwrap()).unwrap();
    let RegionKind::Fork { branch_results, .. } = &region.kind else {
        panic!("expected original fork branch graph");
    };
    let source = region.inputs[0];
    let uses = fork.graph_use_occurrences(source).unwrap();
    assert_eq!(uses.len(), 2);
    for &consumer in branch_results {
        assert!(uses.contains(&GraphUseOccurrence {
            consumer,
            operand: GraphUseOperand::Right,
        }));
    }
    // A syntactic join is not a direct, witnessed unary demand transfer.
    assert!(
        fork.graph_use_occurrences(fork.result.unwrap())
            .unwrap()
            .is_empty()
    );
    assert!(fork.graph_use_occurrences(ValueId(usize::MAX)).is_none());

    let hook = engine.analyze_j_graph("(- +) a").unwrap();
    hook.verify().unwrap();
    let (_, region) = hook.region_for_result(hook.result.unwrap()).unwrap();
    let RegionKind::Hook { .. } = &region.kind else {
        panic!("expected original hook graph");
    };
    let uses = hook.graph_use_occurrences(region.inputs[0]).unwrap();
    assert_eq!(uses.len(), 2);
    assert!(
        uses.iter()
            .any(|use_site| use_site.operand == GraphUseOperand::Left)
    );
    assert!(
        uses.iter()
            .any(|use_site| use_site.operand == GraphUseOperand::Right)
    );
}

#[test]
fn graph_declared_fanout_requests_do_not_imply_all_consumer_demand() {
    use rustj::j_graph_access::{GraphUseDemand, GraphUseOccurrence, GraphUseOperand};
    use rustj::j_graph_ir::{RegionKind, ValueId};

    let mut engine = rustj::Engine::new();
    engine.eval("a=: i. 2 3").unwrap();
    let fork = engine.analyze_j_graph("(|: + |:) a").unwrap();
    fork.verify().unwrap();

    let (_, region) = fork.region_for_result(fork.result.unwrap()).unwrap();
    let RegionKind::Fork { branch_results, .. } = &region.kind else {
        panic!("expected fork");
    };
    let source = region.inputs[0];
    let uses = fork.graph_use_occurrences(source).unwrap();
    assert_eq!(uses.len(), 2);
    let first = GraphUseDemand {
        use_site: GraphUseOccurrence {
            consumer: branch_results[0],
            operand: GraphUseOperand::Right,
        },
        output_region: vec![0..1, 0..2],
    };
    let second = GraphUseDemand {
        use_site: GraphUseOccurrence {
            consumer: branch_results[1],
            operand: GraphUseOperand::Right,
        },
        output_region: vec![1..3, 1..2],
    };
    let partial = fork
        .graph_declared_fanout_coverage(source, std::slice::from_ref(&first))
        .unwrap();
    assert!(!partial.all_structural_uses_declared());
    assert_eq!(partial.missing, vec![second.use_site]);
    assert!(partial.unproved_access.is_empty());

    let complete = fork
        .graph_declared_fanout_coverage(source, &[first.clone(), second.clone()])
        .unwrap();
    assert!(complete.all_structural_uses_declared());
    assert!(complete.missing.is_empty());
    assert!(complete.unproved_access.is_empty());
    assert_eq!(complete.declared.len(), 2);
    assert_eq!(complete.all_uses, uses);

    let duplicated = fork
        .graph_declared_fanout_coverage(source, &[first.clone(), first.clone()])
        .unwrap();
    assert_eq!(duplicated.declared.len(), 2);
    assert_eq!(duplicated.missing, vec![second.use_site]);

    let no_requests = fork.graph_declared_fanout_coverage(source, &[]).unwrap();
    assert_eq!(no_requests.missing.len(), 2);
    assert!(!no_requests.all_structural_uses_declared());

    let wrong_slot = GraphUseDemand {
        use_site: GraphUseOccurrence {
            consumer: branch_results[0],
            operand: GraphUseOperand::Left,
        },
        output_region: vec![0..1, 0..2],
    };
    assert!(fork.graph_declared_fanout_coverage(source, &[wrong_slot]).is_none());
    let wrong_bounds = GraphUseDemand {
        output_region: vec![0..4, 0..2],
        ..first.clone()
    };
    assert!(fork.graph_declared_fanout_coverage(source, &[wrong_bounds]).is_none());
    let wrong_rank = GraphUseDemand {
        output_region: vec![],
        ..first
    };
    assert!(fork.graph_declared_fanout_coverage(source, &[wrong_rank]).is_none());
    assert!(fork.graph_declared_fanout_coverage(ValueId(usize::MAX), &[]).is_none());

    // Hook deliberately uses the same original source in a right and a left
    // operand slot. A "complete" declaration does not prove the left dyadic
    // access relation or permit dropping any original J operation.
    let hook = engine.analyze_j_graph("(- +) a").unwrap();
    hook.verify().unwrap();
    let (_, region) = hook.region_for_result(hook.result.unwrap()).unwrap();
    let source = region.inputs[0];
    let hook_uses = hook.graph_use_occurrences(source).unwrap();
    assert_eq!(hook_uses.len(), 2);
    let hook_requests: Vec<_> = hook_uses
        .iter()
        .map(|site| GraphUseDemand {
            use_site: *site,
            output_region: vec![0..2, 0..3],
        })
        .collect();
    let coverage = hook
        .graph_declared_fanout_coverage(source, &hook_requests)
        .unwrap();
    assert!(coverage.all_structural_uses_declared());
    assert!(coverage.unproved_access.iter().any(|site| {
        site.operand == GraphUseOperand::Left
    }));
    assert_eq!(coverage.declared.len(), 2);
}

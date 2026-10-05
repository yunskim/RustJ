use rustj::{
    Engine,
    j_graph_composition::{CompositionRelation as R, InputSlot, NestedBoundary, RegionConstructor},
    j_graph_ir::{GraphForm, GraphHint, NodeKind, Plan, RegionKind, ValueId},
};

fn graph(source: &str) -> Plan {
    let graph = Engine::new().analyze_j_graph(source).unwrap();
    let analysis = graph.composition_analysis().unwrap();
    analysis.verify(&graph).unwrap();
    graph
}

#[test]
fn pipeline_keeps_edges_and_completion_order_without_horizontal_branches() {
    let graph = graph("(|. @: , @: |.) 1 2 3");
    let sidecar = graph.composition_analysis().unwrap();
    let RegionKind::Pipeline { stage_results } = &graph.regions[0].kind else {
        panic!()
    };
    for pair in stage_results.windows(2) {
        assert!(sidecar.relations.contains(&R::Vertical {
            producer: pair[0],
            consumer: pair[1],
            slot: InputSlot::Right,
        }));
    }
    assert!(sidecar.relations.iter().any(|r| matches!(r,
        R::ObservableOrder { constructor: RegionConstructor::Composition, completion_order, .. }
        if completion_order == stage_results)));
    assert!(
        !sidecar
            .relations
            .iter()
            .any(|r| matches!(r, R::HorizontalCandidate { .. }))
    );
}

#[test]
fn fork_relations_overlap_and_unknown_branches_keep_h_f_g_order() {
    // An unimplemented core primitive must never acquire purity/error proofs
    // just because the fork has two common-input consumers.
    let graph = graph("(c. + -) 7");
    let sidecar = graph.composition_analysis().unwrap();
    let region = &graph.regions[0];
    let RegionKind::Fork {
        branch_results,
        join_result,
        live_across,
    } = &region.kind
    else {
        panic!()
    };
    assert!(sidecar.relations.iter().any(|r| matches!(r,
        R::HorizontalCandidate { inputs, branch_results: branches, .. }
        if inputs == &region.inputs && branches == branch_results)));
    assert!(sidecar.relations.iter().any(|r| matches!(r,
        R::ObservableOrder { constructor: RegionConstructor::OrdinaryFork, completion_order, live_across: retained, .. }
        if completion_order == &vec![branch_results[0], branch_results[1], *join_result]
        && retained == live_across)));
    assert_eq!(sidecar.use_counts[region.inputs[0].0], 2);
    for result in branch_results {
        assert!(sidecar.relations.contains(&R::Vertical {
            producer: region.inputs[0],
            consumer: *result,
            slot: InputSlot::Right,
        }));
    }
}

#[test]
fn capped_and_noun_left_forks_do_not_fabricate_parallel_branches() {
    let capped = graph("([: + -) 7");
    let sidecar = capped.composition_analysis().unwrap();
    assert!(sidecar.relations.iter().any(|r| matches!(
        r,
        R::ObservableOrder {
            constructor: RegionConstructor::CappedFork,
            ..
        }
    )));
    assert!(
        !sidecar
            .relations
            .iter()
            .any(|r| matches!(r, R::HorizontalCandidate { .. }))
    );

    let noun_left = graph("(3 + -) 7");
    let sidecar = noun_left.composition_analysis().unwrap();
    assert!(sidecar.relations.iter().any(|r| matches!(r,
        R::RetainedNounBoundary { operand_path, .. } if operand_path == &[0])));
    assert!(
        !sidecar
            .relations
            .iter()
            .any(|r| matches!(r, R::HorizontalCandidate { .. }))
    );
    let NodeKind::Apply { form, hints, .. } = &noun_left.nodes[noun_left.result.unwrap().0].kind
    else {
        panic!()
    };
    assert!(matches!(form, GraphForm::Modifier { .. }));
    assert!(!hints.contains(GraphHint::ParallelBranchCandidate));
    assert!(!hints.contains(GraphHint::BranchJoinFusionCandidate));
    assert_eq!(
        Engine::new()
            .eval("(3 + -) 7")
            .unwrap()
            .unwrap()
            .int_at(0)
            .unwrap(),
        -4
    );
}

#[test]
fn rank_window_reduction_boundaries_keep_distinct_operand_paths() {
    let graph = graph("(((+/)\\)\"1) 2 3 $ i.6");
    let sidecar = graph.composition_analysis().unwrap();
    let owner = graph.result.unwrap();
    for (path, boundary) in [
        (vec![], NestedBoundary::RankCell),
        (vec![0], NestedBoundary::PrefixWindow),
        (vec![0, 0], NestedBoundary::Reduction),
    ] {
        assert!(sidecar.relations.contains(&R::Nested {
            owner,
            function_path: path,
            boundary
        }));
    }
    assert!(
        sidecar
            .relations
            .iter()
            .any(|r| matches!(r, R::Vertical { consumer, .. } if *consumer == owner))
    );
}

#[test]
fn input_occurrences_and_hook_retention_survive_sidecar_verification() {
    let graph = graph("(+ -) 7");
    let mut sidecar = graph.composition_analysis().unwrap();
    let hook = &graph.regions[0];
    assert!(sidecar.relations.iter().any(|r| matches!(r,
        R::ObservableOrder { constructor: RegionConstructor::Hook, live_across, .. }
        if live_across == &hook.inputs)));
    assert!(
        !sidecar
            .relations
            .iter()
            .any(|r| matches!(r, R::HorizontalCandidate { .. }))
    );
    // Same value in two slots is still two input occurrences, not one edge.
    let mut repeated = graph.clone();
    repeated.regions.clear();
    let result = repeated.result.unwrap();
    if let NodeKind::Apply { left, right, .. } = &mut repeated.nodes[result.0].kind {
        *left = Some(*right);
    }
    let repeated_analysis = repeated.composition_analysis().unwrap();
    assert_eq!(
        repeated_analysis
            .relations
            .iter()
            .filter(|r| matches!(r,
        R::Vertical { consumer, .. } if *consumer == result))
            .count(),
        2
    );
    sidecar.use_counts[0] += 1;
    assert!(sidecar.verify(&graph).is_err());
    let mut sidecar = graph.composition_analysis().unwrap();
    sidecar.relations.push(R::Vertical {
        producer: ValueId(999),
        consumer: graph.result.unwrap(),
        slot: InputSlot::Left,
    });
    assert!(sidecar.verify(&graph).is_err());
    assert!(
        graph
            .composition_analysis()
            .unwrap()
            .verify(&repeated)
            .is_err()
    );
    let mut invalid = graph.clone();
    if let NodeKind::Apply { right, .. } = &mut invalid.nodes[result.0].kind {
        *right = ValueId(999);
    }
    assert!(invalid.composition_analysis().is_err());
}

#[test]
fn copy_rank_header_source_is_not_a_nested_computation() {
    let graph = graph("(+\"(+\"1)) 7");
    let sidecar = graph.composition_analysis().unwrap();
    let paths: Vec<_> = sidecar
        .relations
        .iter()
        .filter_map(|r| match r {
            R::Nested {
                owner,
                function_path,
                boundary: NestedBoundary::RankCell,
            } if *owner == graph.result.unwrap() => Some(function_path.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(paths, vec![Vec::<usize>::new()]);
}

#[test]
fn verifier_rejects_reordered_fork_and_missing_nested_boundaries() {
    let graph = graph("((+/)\"1 + #) 2 3 $ i.6");
    let mut sidecar = graph.composition_analysis().unwrap();
    let order = sidecar
        .relations
        .iter_mut()
        .find_map(|r| match r {
            R::ObservableOrder {
                constructor: RegionConstructor::OrdinaryFork,
                completion_order,
                ..
            } => Some(completion_order),
            _ => None,
        })
        .unwrap();
    order.swap(0, 1);
    assert!(sidecar.verify(&graph).is_err());
    let mut sidecar = graph.composition_analysis().unwrap();
    sidecar.relations.retain(|r| !matches!(r, R::Nested { .. }));
    assert!(sidecar.verify(&graph).is_err());
}

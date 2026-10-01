use rustj::{
    Engine,
    j_graph_ir::{
        AccessContract, FusionStructure, GraphAnalyzability, GraphBasisKind, GraphForm, GraphHint,
        GraphRuleRef, IterationContract, J_GRAPH_SCHEMA_VERSION, NodeKind,
        RegionKind, ResourceCompositionRule, ResourceRuleRef, SymbolicResourceExpr,
    },
    primitive::{PrimitiveId, REGISTRY_VERSION},
};

#[test]
fn atop_is_an_explicit_pipeline_region_with_stage_values() {
    let graph = Engine::new()
        .analyze_j_graph("(|. @: , @: |.) 1 2 3")
        .unwrap();
    graph.verify().unwrap();

    let result = graph.result.unwrap();
    let (_, region) = graph
        .region_for_result(result)
        .expect("atop should retain a pipeline region");
    let RegionKind::Pipeline { stage_results } = &region.kind else {
        panic!("expected pipeline region")
    };

    assert_eq!(stage_results.len(), 3);
    assert_eq!(stage_results.last().copied(), Some(result));
    assert!(region.hints.contains(GraphHint::PipelineFusionCandidate));
    assert!(
        region
            .hints
            .contains(GraphHint::IntermediateMaterializationElision)
    );
    assert_eq!(
        region.resource_composition,
        ResourceCompositionRule::Pipeline
    );

    // The pipeline is no longer a compressed list only: every stage is an
    // actual applied graph node with its own propagated facts.
    for stage in stage_results {
        let NodeKind::Apply { form, .. } = &graph.nodes[stage.0].kind else {
            panic!("pipeline stage must be an applied graph node")
        };
        assert!(matches!(form, GraphForm::Atomic));
        assert_eq!(graph.nodes[stage.0].facts.shape.as_deref(), Some(&[3][..]));
    }
}

#[test]
fn fork_is_an_explicit_branch_join_region_and_common_input_is_visible() {
    let graph = Engine::new()
        .analyze_j_graph("(+/ % #) 1 2 3 4")
        .unwrap();
    graph.verify().unwrap();

    let result = graph.result.unwrap();
    let (_, region) = graph
        .region_for_result(result)
        .expect("fork should retain a branch/join region");
    let RegionKind::Fork {
        branch_results,
        join_result,
        live_across,
    } = &region.kind
    else {
        panic!("expected fork region")
    };

    assert_eq!(branch_results.len(), 2);
    assert_eq!(*join_result, result);
    assert_eq!(live_across, &region.inputs);
    assert!(region.hints.contains(GraphHint::BranchJoinFusionCandidate));
    assert!(region.hints.contains(GraphHint::RetainedValueCandidate));
    assert!(region.hints.contains(GraphHint::ParallelBranchCandidate));
    assert_eq!(
        region.resource_composition,
        ResourceCompositionRule::BranchJoin
    );

    let input = region.inputs[0];
    assert!(
        graph.use_counts()[input.0] >= 2,
        "fork common input should have multiple consumers in the J graph itself"
    );

    for branch in branch_results {
        assert!(branch.0 < graph.nodes.len());
    }
    assert_eq!(graph.nodes[result.0].facts.shape.as_deref(), Some(&[][..]));
}

#[test]
fn modifiers_expose_collective_and_cell_parallel_contracts_before_execution_lowering() {
    let graph = Engine::new().analyze_j_graph("+/1 2 3").unwrap();
    let result = graph.result.unwrap();
    let NodeKind::Apply {
        form,
        basis,
        hints,
        contract,
        ..
    } = &graph.nodes[result.0].kind
    else {
        panic!()
    };
    assert!(matches!(form, GraphForm::Reduce { .. }));
    assert_eq!(basis.layers, vec![GraphBasisKind::Reduce]);
    assert!(hints.contains(GraphHint::ReductionStructure));
    assert_eq!(contract.iteration, IterationContract::Reduction);
    assert_eq!(contract.access, AccessContract::ReductionAxis);
    assert_eq!(contract.fusion_structure, FusionStructure::ReductionAware);
    assert_eq!(
        contract.accumulator,
        SymbolicResourceExpr::ReductionAccumulator
    );
    assert_eq!(graph.nodes[result.0].facts.shape.as_deref(), Some(&[][..]));

    let mut engine = Engine::new();
    engine.eval("a=:i.2 3").unwrap();
    let graph = engine.analyze_j_graph("+/\"1 a").unwrap();
    let result = graph.result.unwrap();
    let NodeKind::Apply {
        form,
        basis,
        hints,
        contract,
        ..
    } = &graph.nodes[result.0].kind
    else {
        panic!()
    };
    assert!(matches!(form, GraphForm::Rank { .. }));
    assert_eq!(
        basis.layers,
        vec![GraphBasisKind::CellApply, GraphBasisKind::Reduce]
    );
    assert!(hints.contains(GraphHint::CellParallelStructure));
    assert_eq!(contract.iteration, IterationContract::CellMap);
    assert_eq!(contract.access, AccessContract::CellRelative);
    assert_eq!(graph.nodes[result.0].facts.shape.as_deref(), Some(&[2][..]));
}


#[test]
fn prefix_infix_exposes_window_graph_basis_without_collapsing_inner_reduction() {
    let graph = Engine::new().analyze_j_graph("(+/)\\ 1 2 3 4").unwrap();
    let result = graph.result.unwrap();
    let NodeKind::Apply {
        form,
        basis,
        hints,
        contract,
        ..
    } = &graph.nodes[result.0].kind
    else {
        panic!()
    };

    assert!(matches!(form, GraphForm::PrefixInfix { .. }));
    assert_eq!(
        basis.layers,
        vec![GraphBasisKind::Window, GraphBasisKind::Reduce]
    );
    assert!(hints.contains(GraphHint::WindowStructure));
    assert_eq!(contract.iteration, IterationContract::WindowFamily);
    assert_eq!(contract.access, AccessContract::WindowRelative);
    assert_eq!(contract.fusion_structure, FusionStructure::WindowAware);
    assert_eq!(
        contract.accumulator,
        SymbolicResourceExpr::ReductionAccumulator
    );
    assert_eq!(
        contract.working_state,
        SymbolicResourceExpr::WindowWorkingSet
    );

    // Scan is intentionally not asserted as a separate Graph Basis yet.
    // Historical JAXA treated that as an open basis-taxonomy question.
    graph.verify().unwrap();
}

#[test]
fn graph_basis_is_distinct_from_execution_basis_and_preserves_graph_granularity() {
    let graph = Engine::new().analyze_j_graph("1+2").unwrap();
    let result = graph.result.unwrap();
    let NodeKind::Apply { basis, .. } = &graph.nodes[result.0].kind else {
        panic!()
    };
    assert_eq!(basis.layers, vec![GraphBasisKind::Elementwise]);

    let graph = Engine::new().analyze_j_graph("1 { 10 20 30").unwrap();
    let result = graph.result.unwrap();
    let NodeKind::Apply { basis, .. } = &graph.nodes[result.0].kind else {
        panic!()
    };
    assert_eq!(basis.layers, vec![GraphBasisKind::DynamicGather]);

    // Execution classification is a separate type and a separate lowering
    // decision even where the vocabulary happens to use similar names.
    let analysis = Engine::new().analyze_compilation("1+2").unwrap();
    assert_eq!(
        analysis.execution.nodes[analysis.execution.result.unwrap().0]
            .basis
            .layers,
        vec![rustj::analysis::ExecutionBasisKind::Elementwise]
    );
}

#[test]
fn structural_index_ops_expose_virtual_indexing_candidate() {
    for source in ["|. 1 2 3", ", 1 2 3", "|: i. 2 3"] {
        let graph = Engine::new().analyze_j_graph(source).unwrap();
        let result = graph.result.unwrap();
        let NodeKind::Apply { hints, .. } = &graph.nodes[result.0].kind else {
            panic!()
        };
        assert!(hints.contains(GraphHint::VirtualIndexingCandidate));
    }
}

#[test]
fn execution_ir_is_derived_from_explicit_j_graph_stages() {
    let analysis = Engine::new()
        .analyze_compilation("(|. @: , @: |.) 1 2 3")
        .unwrap();

    let graph_result = analysis.j_graph.result.unwrap();
    let (_, region) = analysis
        .j_graph
        .region_for_result(graph_result)
        .expect("pipeline region");
    let RegionKind::Pipeline { stage_results } = &region.kind else {
        panic!()
    };

    for stage in stage_results {
        assert!(
            analysis
                .execution
                .nodes
                .iter()
                .any(|node| node.j_origin == Some(*stage)),
            "each J Graph stage must survive as execution provenance"
        );
    }
    assert_eq!(analysis.execution.opportunities.len(), 1);
    analysis.execution.verify().unwrap();
}

#[test]
fn graph_and_execution_ir_answer_different_questions_without_recovering_topology() {
    let analysis = Engine::new()
        .analyze_compilation("(+/ % #) 1 2 3 4")
        .unwrap();

    let graph_result = analysis.j_graph.result.unwrap();
    let (_, region) = analysis
        .j_graph
        .region_for_result(graph_result)
        .expect("fork region");
    assert!(matches!(region.kind, RegionKind::Fork { .. }));
    assert!(region.hints.contains(GraphHint::BranchJoinFusionCandidate));

    // Execution lowering receives already-explicit branch operations and only
    // projects the region onto execution ValueIds.
    for value in region
        .inputs
        .iter()
        .copied()
        .chain(std::iter::once(region.result))
    {
        assert!(
            analysis
                .execution
                .nodes
                .iter()
                .any(|node| node.j_origin == Some(value))
        );
    }
    assert!(!analysis.execution.opportunities.is_empty());
}

#[test]
fn graph_header_and_rule_provenance_are_explicit() {
    let graph = Engine::new().analyze_j_graph("+/1 2 3").unwrap();
    assert_eq!(graph.header.schema, J_GRAPH_SCHEMA_VERSION);
    assert_eq!(graph.header.primitive_registry_version, REGISTRY_VERSION);

    let result = graph.result.unwrap();
    let NodeKind::Apply { rules, .. } = &graph.nodes[result.0].kind else {
        panic!()
    };
    assert_eq!(rules.resource, ResourceRuleRef::StructuralComposition);

    let graph = Engine::new().analyze_j_graph("+ 1").unwrap();
    let result = graph.result.unwrap();
    let NodeKind::Apply { rules, .. } = &graph.nodes[result.0].kind else {
        panic!()
    };
    assert_eq!(rules.shape, GraphRuleRef::Primitive(PrimitiveId::Add));
    assert_eq!(rules.resource, ResourceRuleRef::Unknown);
}

#[test]
fn graph_analyzability_distinguishes_static_graph_from_missing_facts() {
    let graph = Engine::new().analyze_j_graph("+/1 2 3").unwrap();
    assert_eq!(graph.analyzability(), GraphAnalyzability::Static);

    // Verb values have no call-instance result facts yet, so the graph remains
    // structurally known but explicitly reports incomplete analysis facts.
    let graph = Engine::new().analyze_j_graph("+").unwrap();
    assert_eq!(
        graph.analyzability(),
        GraphAnalyzability::StaticWithUnknownFacts
    );
}

#[test]
fn graph_verifier_rejects_unknown_schema() {
    let mut graph = Engine::new().analyze_j_graph("1+2").unwrap();
    graph.header.schema.minor = graph.header.schema.minor.saturating_add(1);
    let error = graph.verify().unwrap_err();
    assert!(error.contains("schema version"));
}


#[test]
fn hook_keeps_retained_value_without_claiming_parallel_siblings() {
    let graph = Engine::new().analyze_j_graph("(+ -) 3").unwrap();
    let result = graph.result.unwrap();
    let (_, region) = graph.region_for_result(result).expect("hook region");

    assert!(matches!(region.kind, RegionKind::Hook { .. }));
    assert!(region.hints.contains(GraphHint::RetainedValueCandidate));
    assert!(region.hints.contains(GraphHint::BranchJoinFusionCandidate));
    assert!(!region.hints.contains(GraphHint::ParallelBranchCandidate));
}

#[test]
fn graph_facts_are_early_semantic_facts_not_representation_facts() {
    let graph = Engine::new().analyze_j_graph("|. 1 2 3").unwrap();
    let result = graph.result.unwrap();
    let facts = &graph.nodes[result.0].facts;

    assert_eq!(facts.shape.as_deref(), Some(&[3][..]));
    assert_eq!(facts.rank, Some(1));
    // GraphFacts intentionally has no layout/representation field.  Execution
    // lowering is responsible for resolved representation-side facts.
}

#[test]
fn nested_regions_do_not_assume_unique_result_ownership() {
    let graph = Engine::new()
        .analyze_j_graph("((+/ % #) @: |.) 1 2 3 4")
        .unwrap();
    graph.verify().unwrap();

    let result = graph.result.unwrap();
    let regions = graph.regions_for_result(result).collect::<Vec<_>>();
    assert!(
        regions.len() >= 1,
        "a result may be owned by one or more nested J combinator regions"
    );
    let (_, outermost) = graph.region_for_result(result).expect("outer region");
    assert!(matches!(outermost.kind, RegionKind::Pipeline { .. }));
}

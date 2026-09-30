use rustj::{
    Engine,
    j_graph_ir::{GraphForm, GraphHint, NodeKind},
};

#[test]
fn atop_is_a_first_class_J_graph_pipeline() {
    let graph = Engine::new()
        .analyze_j_graph("(|. @: , @: |.) 1 2 3")
        .unwrap();
    graph.verify().unwrap();

    let result = graph.result.unwrap();
    let NodeKind::Apply { form, hints, .. } = &graph.nodes[result.0].kind else {
        panic!("expected applied J graph node")
    };
    let GraphForm::Pipeline { stages } = form else {
        panic!("@: must be represented as a pipeline graph form")
    };
    assert_eq!(stages.len(), 3);
    assert!(hints.contains(GraphHint::PipelineFusionCandidate));
    assert!(hints.contains(GraphHint::IntermediateMaterializationElision));
}

#[test]
fn fork_is_a_first_class_J_graph_branch_join() {
    let graph = Engine::new()
        .analyze_j_graph("(+/ % #) 1 2 3 4")
        .unwrap();
    graph.verify().unwrap();

    let result = graph.result.unwrap();
    let NodeKind::Apply { form, hints, .. } = &graph.nodes[result.0].kind else {
        panic!("expected fork application")
    };
    assert!(matches!(form, GraphForm::Fork { .. }));
    assert!(hints.contains(GraphHint::BranchJoinFusionCandidate));
    assert!(hints.contains(GraphHint::RetainedValueCandidate));
    assert!(hints.contains(GraphHint::ParallelBranchCandidate));
}

#[test]
fn J_modifiers_expose_collective_and_cell_parallel_structure_before_execution_lowering() {
    let graph = Engine::new().analyze_j_graph("+/1 2 3").unwrap();
    let result = graph.result.unwrap();
    let NodeKind::Apply { form, hints, .. } = &graph.nodes[result.0].kind else {
        panic!()
    };
    assert!(matches!(form, GraphForm::Reduce { .. }));
    assert!(hints.contains(GraphHint::ReductionStructure));

    let mut engine = Engine::new();
    engine.eval("a=:i.2 3").unwrap();
    let graph = engine.analyze_j_graph("+/\"1 a").unwrap();
    let result = graph.result.unwrap();
    let NodeKind::Apply { form, hints, .. } = &graph.nodes[result.0].kind else {
        panic!()
    };
    assert!(matches!(form, GraphForm::Rank { .. }));
    assert!(hints.contains(GraphHint::CellParallelStructure));
}

#[test]
fn execution_IR_is_derived_from_and_points_back_to_the_J_graph() {
    let analysis = Engine::new()
        .analyze_compilation("(|. @: , @: |.) 1 2 3")
        .unwrap();

    let graph_result = analysis.j_graph.result.unwrap();
    let matching: Vec<_> = analysis
        .execution
        .nodes
        .iter()
        .filter(|node| node.j_origin == Some(graph_result))
        .collect();

    // One compact J pipeline application expands to multiple execution ops.
    assert!(matching.len() >= 3);
    assert_eq!(analysis.execution.opportunities.len(), 1);
    analysis.execution.verify().unwrap();
}

#[test]
fn J_graph_and_execution_IR_answer_different_questions() {
    let analysis = Engine::new()
        .analyze_compilation("(+/ % #) 1 2 3 4")
        .unwrap();

    let graph_result = analysis.j_graph.result.unwrap();
    let NodeKind::Apply { form, hints, .. } =
        &analysis.j_graph.nodes[graph_result.0].kind
    else {
        panic!()
    };
    assert!(matches!(form, GraphForm::Fork { .. }));
    assert!(hints.contains(GraphHint::BranchJoinFusionCandidate));

    // Execution lowering expands the fork into ordinary calls while retaining
    // origin/opportunity provenance.
    assert!(
        analysis
            .execution
            .nodes
            .iter()
            .filter(|node| node.j_origin == Some(graph_result))
            .count()
            >= 3
    );
    assert!(!analysis.execution.opportunities.is_empty());
}


#[test]
fn J_graph_header_and_rule_provenance_are_explicit() {
    use rustj::{
        j_graph_ir::{
            GraphRuleRef, J_GRAPH_SCHEMA_VERSION, ResourceRuleRef,
        },
        primitive::{PrimitiveId, REGISTRY_VERSION},
    };

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
fn J_graph_verifier_rejects_unknown_schema() {
    let mut graph = Engine::new().analyze_j_graph("1+2").unwrap();
    graph.header.schema.minor = graph.header.schema.minor.saturating_add(1);
    let error = graph.verify().unwrap_err();
    assert!(error.contains("schema version"));
}

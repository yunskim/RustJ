use rustj::{
    Engine,
    j_graph_ir::ResourceCompositionRule,
};

#[test]
fn pipeline_resource_composition_counts_internal_and_elidable_atoms() {
    let graph = Engine::new()
        .analyze_j_graph("(|. @: , @: |.) 1 2 3")
        .unwrap();
    let resources = graph.symbolic_resource_analysis();

    assert_eq!(resources.regions.len(), 1);
    let region = &resources.regions[0];
    assert_eq!(region.composition, ResourceCompositionRule::Pipeline);
    assert_eq!(region.internal_atoms.known, 6);
    assert!(!region.internal_atoms.has_unknown);
    assert_eq!(region.elidable_materialization_atoms.known, 6);
    assert_eq!(region.retained_live_atoms.known, 0);
    assert!(!region.has_reduction_accumulator);
    assert!(!region.has_unknown_resource_requirement);
    assert!(region.graph_order_peak_live_atoms.known >= 3);
}

#[test]
fn fork_resource_composition_exposes_retained_input_and_reduction_state() {
    let graph = Engine::new()
        .analyze_j_graph("(+/ % #) 1 2 3 4")
        .unwrap();
    let resources = graph.symbolic_resource_analysis();

    assert_eq!(resources.regions.len(), 1);
    let region = &resources.regions[0];
    assert_eq!(region.composition, ResourceCompositionRule::BranchJoin);

    // Two scalar branch results exist before the scalar join.
    assert_eq!(region.internal_atoms.known, 2);
    assert_eq!(region.elidable_materialization_atoms.known, 2);

    // The original four-atom input remains logically live across branch work.
    assert_eq!(region.retained_live_atoms.known, 4);
    assert!(region.graph_order_peak_live_atoms.known >= 4);

    // +/ branch exposes a symbolic accumulator requirement before a concrete
    // target decides register/shared-memory realization.
    assert!(region.has_reduction_accumulator);
}


#[test]
fn window_resource_contract_composes_reuse_and_inner_reduction_state() {
    let graph = Engine::new()
        .analyze_j_graph("(+/)\\ 1 2 3 4")
        .unwrap();
    let resources = graph.symbolic_resource_analysis();
    let result = graph.result.unwrap();
    let summary = &resources.nodes[result.0];

    assert!(matches!(
        summary.accumulator,
        rustj::j_graph_ir::SymbolicResourceExpr::ReductionAccumulator
    ));
    assert!(matches!(
        summary.working_state,
        rustj::j_graph_ir::SymbolicResourceExpr::WindowWorkingSet
    ));
}

#[test]
fn resource_summary_keeps_unknown_distinct_from_zero() {
    let graph = Engine::new().analyze_j_graph("1 i. 1 2 3").unwrap();
    let resources = graph.symbolic_resource_analysis();
    let result = graph.result.unwrap();
    let summary = &resources.nodes[result.0];

    // Search/lookup implementation details are not modeled yet.  The graph
    // analysis must report Unknown rather than silently treating this as no cost.
    assert!(matches!(
        summary.temporary,
        rustj::j_graph_ir::SymbolicResourceExpr::Unknown
    ));
}


#[test]
fn hook_identity_input_is_not_counted_as_internal_intermediate() {
    let graph = Engine::new().analyze_j_graph("(+ -) 3").unwrap();
    let resources = graph.symbolic_resource_analysis();

    assert_eq!(resources.regions.len(), 1);
    let region = &resources.regions[0];

    // The retained left/identity path is an external region input, not a newly
    // allocated branch intermediate.
    assert_eq!(region.internal_atoms.known, 1);
    assert_eq!(region.retained_live_atoms.known, 1);
    assert!(!region.has_unknown_resource_requirement);
}

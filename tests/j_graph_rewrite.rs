use rustj::{
    Engine,
    j_graph_ir::{GraphBasisKind, NodeKind},
    j_graph_rewrite::{
        GraphEquivalenceWitness, GraphOptimizationPhase, GraphRewriteRuleId,
        PruningMonotonicity, ResourceBoundLocality, RewriteInput, RewriteNodeSemantics,
        GRAPH_OPTIMIZATION_ORDER, RULES,
    },
    j_graph_ir::SymbolicResourceExpr,
    primitive::PrimitiveId,
    semantic::FunctionHead,
};

#[test]
fn find_produces_a_witnessed_graph_rewrite_candidate_without_mutating_source_graph() {
    let analysis = Engine::new()
        .analyze_compilation("'ana' E. 'banana'")
        .unwrap();

    assert_eq!(analysis.graph_rewrites.len(), 1);
    let candidate = &analysis.graph_rewrites[0];
    candidate.verify(&analysis.j_graph).unwrap();

    assert_eq!(analysis.graph_rewrite_resources.len(), 1);
    let resources = &analysis.graph_rewrite_resources[0];
    assert_eq!(resources.source_value, candidate.provenance.source_value);
    assert!(
        resources.source.has_unknown_state_requirement,
        "Search implementation state is not modeled precisely yet"
    );
    assert_eq!(
        resources.replacement.internal_materialization_atoms.known,
        18
    );
    assert!(!resources
        .replacement
        .internal_materialization_atoms
        .has_unknown);
    assert_eq!(
        resources.replacement.unfused_internal_traffic_atoms.known,
        36
    );
    assert_eq!(
        resources.replacement.elidable_internal_traffic_atoms.known,
        36
    );
    assert!(
        resources
            .replacement
            .state_requirements
            .contains(&SymbolicResourceExpr::WindowWorkingSet)
    );
    assert!(
        resources
            .replacement
            .state_requirements
            .contains(&SymbolicResourceExpr::StructuralComposition)
    );
    assert!(
        !resources.early_pruning_allowed,
        "resource evaluation must not bypass the rule's pruning proof contract"
    );
    assert!(
        resources.source.has_unknown_implementation_resource,
        "graph-visible zero must not be interpreted as a proven zero Search implementation cost"
    );
    assert!(
        resources.replacement.has_unknown_implementation_resource,
        "symbolic WindowWorkingSet/structural state must remain unknown even when window volume is known"
    );
    let comparison = resources.comparison();
    assert_eq!(
        comparison.unfused_internal_traffic,
        rustj::j_graph_resource::ResourceMetricOrdering::Incomparable
    );
    assert_eq!(
        comparison.elidable_internal_traffic,
        rustj::j_graph_resource::ResourceMetricOrdering::Incomparable
    );

    assert_eq!(candidate.rule, GraphRewriteRuleId::FindViaWindowMatch);
    assert_eq!(
        candidate.witness,
        GraphEquivalenceWitness::JFindCutMatchIdentity
    );
    assert_eq!(
        candidate.provenance.source_basis.layers,
        vec![GraphBasisKind::Search]
    );
    let source_facts = &analysis.j_graph.nodes[candidate.provenance.source_value.0].facts;
    assert_eq!(
        source_facts.dtype,
        rustj::facts::TypeFact::Exact(rustj::types::DType::Bool)
    );
    assert_eq!(source_facts.shape.as_deref(), Some(&[6][..]));
    assert_eq!(source_facts.rank, Some(1));
    assert_eq!(resources.source.output_atoms, Some(6));
    assert_eq!(resources.replacement.output_atoms, Some(6));

    let source = &analysis.j_graph.nodes[candidate.provenance.source_value.0];
    let NodeKind::Apply {
        function, basis, ..
    } = &source.kind
    else {
        panic!("rewrite source should remain an applied J Graph node")
    };
    assert!(matches!(
        &function.head,
        FunctionHead::PrimitiveVerb(PrimitiveId::Find)
    ));
    assert_eq!(basis.layers, vec![GraphBasisKind::Search]);

    assert_eq!(candidate.replacement.nodes.len(), 2);
    assert_eq!(
        candidate.replacement.nodes[1].facts,
        analysis.j_graph.nodes[candidate.provenance.source_value.0].facts
    );
    let RewriteInput::Source(window_source) =
        candidate.replacement.nodes[0].inputs[0]
    else {
        panic!("window should read the original right argument")
    };
    assert_eq!(
        candidate.replacement.nodes[0].facts.dtype,
        analysis.j_graph.nodes[window_source.0].facts.dtype
    );
    assert_eq!(
        candidate.replacement.nodes[0].facts.shape.as_deref(),
        Some(&[6, 3][..])
    );
    assert_eq!(candidate.replacement.nodes[0].facts.rank, Some(2));

    assert_eq!(
        candidate.replacement.nodes[0].basis,
        GraphBasisKind::Window
    );
    assert_eq!(
        candidate.replacement.nodes[0].semantics,
        RewriteNodeSemantics::WindowByPatternShape
    );
    assert_eq!(
        candidate.replacement.nodes[1].basis,
        GraphBasisKind::CellApply
    );
    assert_eq!(
        candidate.replacement.nodes[1].semantics,
        RewriteNodeSemantics::MatchPatternCell
    );
    assert!(matches!(
        candidate.replacement.nodes[1].inputs.as_slice(),
        [RewriteInput::Source(_), RewriteInput::Node(_)]
    ));
}


#[test]
fn rewrite_verifier_rejects_stale_rule_derived_facts() {
    let graph = Engine::new()
        .analyze_j_graph("'ana' E. 'banana'")
        .unwrap();
    let mut candidate = graph.rewrite_candidates().pop().unwrap();
    candidate.replacement.nodes[0].facts.rank = Some(99);
    let error = candidate.verify(&graph).unwrap_err();
    assert!(error.contains("facts"));
}

#[test]
fn rewrite_registry_is_a_rule_registry_not_a_profitability_ranking() {
    assert_eq!(RULES.len(), 1);
    assert_eq!(RULES[0].id, GraphRewriteRuleId::FindViaWindowMatch);
    assert_eq!(RULES[0].source_outer_basis, GraphBasisKind::Search);
    assert_eq!(RULES[0].replacement_outer_basis, GraphBasisKind::Window);
    assert_eq!(
        RULES[0].pruning.locality,
        ResourceBoundLocality::GlobalContextDependent
    );
    assert_eq!(
        RULES[0].pruning.monotonicity,
        PruningMonotonicity::Unproven
    );
    assert!(!RULES[0].pruning.sound_for_early_pruning());

    assert_eq!(
        GRAPH_OPTIMIZATION_ORDER,
        &[
            GraphOptimizationPhase::BasisDiscovery,
            GraphOptimizationPhase::RewriteCandidateGeneration,
            GraphOptimizationPhase::EquivalenceValidation,
            GraphOptimizationPhase::CandidateResourceEvaluation,
            GraphOptimizationPhase::SoundResourcePruning,
        ]
    );
}

#[test]
fn unrelated_graphs_do_not_get_invented_algebraic_rewrites() {
    for source in ["1+2", "+/1 2 3", "(+/)\\ 1 2 3"] {
        let graph = Engine::new().analyze_j_graph(source).unwrap();
        assert!(graph.rewrite_candidates().is_empty(), "{source}");
    }
}

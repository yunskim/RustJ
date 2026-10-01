use rustj::{
    Engine,
    j_graph_ir::{GraphBasisKind, NodeKind},
    j_graph_rewrite::{
        GraphEquivalenceWitness, GraphOptimizationPhase, GraphRewriteRuleId,
        PruningMonotonicity, ResourceBoundLocality, RewriteInput, RewriteNodeSemantics,
        GRAPH_OPTIMIZATION_ORDER, RULES,
    },
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

    assert_eq!(candidate.rule, GraphRewriteRuleId::FindViaWindowMatch);
    assert_eq!(
        candidate.witness,
        GraphEquivalenceWitness::JFindCutMatchIdentity
    );
    assert_eq!(
        candidate.provenance.source_basis.layers,
        vec![GraphBasisKind::Search]
    );

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

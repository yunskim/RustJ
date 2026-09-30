use rustj::{
    Engine,
    analysis::BasisKind,
    expansion::{
        EquivalenceWitness, ExpansionInput, ExpansionNodeSemantics, ExpansionRuleId, discover,
    },
    logical_ir::OpKind,
};

#[test]
fn find_keeps_its_semantic_identity_and_offers_a_window_match_expansion() {
    let plan = Engine::new().analyze_a3("'co' E. 'cocoa'").unwrap();
    plan.verify().unwrap();

    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    assert!(matches!(
        plan.operations[producer.0].kind,
        OpKind::SemanticCall(_)
    ));

    let expansions = discover(&plan);
    assert_eq!(expansions.len(), 1);
    let expansion = &expansions[0];
    assert_eq!(expansion.source_op, producer);
    assert_eq!(expansion.source_result, result);
    assert_eq!(expansion.rule, ExpansionRuleId::FindViaWindowMatch);
    assert_eq!(
        expansion.witness,
        EquivalenceWitness::JFindCutMatchIdentity
    );
    expansion.graph.verify().unwrap();

    assert_eq!(expansion.graph.nodes.len(), 2);
    assert_eq!(expansion.graph.nodes[0].basis, BasisKind::WindowView);
    assert_eq!(
        expansion.graph.nodes[0].semantics,
        ExpansionNodeSemantics::WindowByPatternShape
    );
    assert_eq!(expansion.graph.nodes[1].basis, BasisKind::CellApply);
    assert_eq!(
        expansion.graph.nodes[1].semantics,
        ExpansionNodeSemantics::MatchPatternCell
    );
    assert_eq!(
        expansion.graph.nodes[1].inputs[1],
        ExpansionInput::Node(rustj::expansion::ExpansionNodeId(0))
    );
}

#[test]
fn ordinary_basis_calls_do_not_gain_unrelated_expansions() {
    let plan = Engine::new().analyze_a3("1+2").unwrap();
    assert!(discover(&plan).is_empty());
}

#[test]
fn expansion_graph_rejects_forward_node_dependencies() {
    let plan = Engine::new().analyze_a3("'co' E. 'cocoa'").unwrap();
    let mut expansion = discover(&plan).pop().unwrap();
    expansion.graph.nodes[0]
        .inputs
        .push(ExpansionInput::Node(rustj::expansion::ExpansionNodeId(1)));
    assert!(expansion.graph.verify().is_err());
}

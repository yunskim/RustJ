use rustj::{
    Engine,
    analysis::ExecutionBasisKind,
    expansion::{
        EquivalenceWitness, ExpansionInput, ExpansionNodeSemantics, ExpansionRuleId,
        discover, execute_reference, for_graph_rewrite,
    },
    logical_ir::{ExecutionBasisPayload, OpKind, WindowShapeSpec},
};


fn literal_source_values(plan: &rustj::logical_ir::Plan) -> Vec<Option<rustj::Value>> {
    let mut values = vec![None; plan.values.len()];
    for operation in &plan.operations {
        if let OpKind::Literal(value) = &operation.kind {
            let [result] = operation.results.as_slice() else {
                panic!("literal should have exactly one result")
            };
            values[result.0] = Some(value.clone());
        }
    }
    values
}

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
    assert_eq!(expansion.graph.nodes[0].basis, ExecutionBasisKind::WindowView);
    let [ExpansionInput::Source(window_source), ExpansionInput::Source(pattern)] =
        expansion.graph.nodes[0].inputs.as_slice()
    else {
        panic!("window expansion inputs")
    };
    assert_eq!(
        expansion.graph.nodes[0].payload,
        ExecutionBasisPayload::WindowView {
            source: *window_source,
            shape: WindowShapeSpec::PatternShape { pattern: *pattern },
        }
    );
    assert_eq!(
        expansion.graph.nodes[0].semantics,
        ExpansionNodeSemantics::WindowByPatternShape
    );
    assert_eq!(expansion.graph.nodes[1].basis, ExecutionBasisKind::CellApply);
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
fn find_window_match_reference_realization_matches_current_find_subset() {
    for source in [
        "'co' E. 'cocoa'",
        "'ana' E. 'banana'",
        "2 3 E. 1 2 3 2 3",
        "'' E. 'abc'",
    ] {
        let plan = Engine::new().analyze_a3(source).unwrap();
        let expansion = discover(&plan).pop().expect("find expansion");
        let values = literal_source_values(&plan);
        let expanded = execute_reference(&expansion, &values)
            .unwrap()
            .json();
        let direct = Engine::new()
            .eval(source)
            .unwrap()
            .unwrap()
            .json();
        assert_eq!(expanded, direct, "{source}");
    }

    let source = "1 2 E. 1";
    let plan = Engine::new().analyze_a3(source).unwrap();
    let expansion = discover(&plan).pop().expect("find expansion");
    let values = literal_source_values(&plan);
    let expanded = execute_reference(&expansion, &values).unwrap_err();
    let direct = Engine::new().eval(source).unwrap_err();
    assert_eq!(expanded.kind(), direct.kind());
}


#[test]
fn graph_rewrite_links_to_execution_expansion_by_j_origin() {
    let analysis = Engine::new()
        .analyze_compilation("'co' E. 'cocoa'")
        .unwrap();
    let candidate = &analysis.graph_rewrites[0];
    let plan = rustj::logical_ir::Plan::from_transition(&analysis.execution);
    plan.verify().unwrap();

    let expansion = for_graph_rewrite(&plan, candidate).unwrap();
    assert_eq!(
        plan.operations[expansion.source_op.0].j_origin,
        Some(candidate.provenance.source_value)
    );
    assert_eq!(expansion.rule, candidate.rule);
    assert_eq!(expansion.witness, candidate.witness);

    let values = literal_source_values(&plan);
    let expanded = execute_reference(&expansion, &values).unwrap().json();
    let direct = Engine::new()
        .eval("'co' E. 'cocoa'")
        .unwrap()
        .unwrap()
        .json();
    assert_eq!(expanded, direct);
}

#[test]
fn ordinary_basis_calls_do_not_gain_unrelated_expansions() {
    let plan = Engine::new().analyze_a3("1+2").unwrap();
    assert!(discover(&plan).is_empty());
}


#[test]
fn expansion_graph_rejects_payload_input_drift() {
    let plan = Engine::new().analyze_a3("'co' E. 'cocoa'").unwrap();
    let mut expansion = discover(&plan).pop().unwrap();
    expansion.graph.nodes[0].payload = ExecutionBasisPayload::CellApply;
    assert!(expansion.graph.verify().is_err());
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

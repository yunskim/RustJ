use rustj::{
    Engine,
    j_graph_ir::{GraphForm, NodeKind, Plan},
    parser_capture::CaptureEvent,
    primitive::PrimitiveId,
    semantic::{ExprKind, FunctionHead, FunctionOperand},
};
fn shape(engine: &mut Engine, source: &str) -> Vec<usize> {
    engine.eval(source).unwrap().unwrap().shape().to_vec()
}
#[test]
fn right_verb_rank_is_metadata_and_never_an_execution() {
    let mut engine = Engine::new();
    assert_eq!(shape(&mut engine, "(,\"+) i.2 3"), vec![2, 3, 1]);
    assert_eq!(shape(&mut engine, "(,\"#) i.2 3"), vec![6]);
    // E.'s monad is invalid, but its header is still a valid rank request.
    assert_eq!(shape(&mut engine, "(,\"E.) i.2 3"), vec![2, 3, 1]);
    assert_eq!(shape(&mut engine, "(,\"i.) i.2 3"), vec![2, 3]);
    assert_eq!(shape(&mut engine, "(,\"(+\"_1)) i.2 3"), vec![6]);
    assert_eq!(shape(&mut engine, "(,\"(+\"1 0 1)) i.2 3"), vec![2, 3]);
    assert_eq!(shape(&mut engine, "(+ (\"-)) i.4"), vec![4]);
    assert_eq!(shape(&mut engine, "(,\"+) i.0 3"), vec![0, 3, 1]);
    assert_eq!(shape(&mut engine, "(,\"+) 0 3$'x'"), vec![0, 3, 1]);
    engine.eval("count=:0").unwrap();
    engine.eval("op=:1 : 0\ncount=:count+1\nu y\n)").unwrap();
    engine.eval("right=:+op").unwrap();
    engine.eval("f=:,\"right").unwrap();
    assert_eq!(shape(&mut engine, "f i.2 3"), vec![6]);
    assert_eq!(engine.eval("count").unwrap().unwrap().int_at(0).unwrap(), 0);
}
#[test]
fn names_copy_header_at_stack_entry_while_execution_remains_late() {
    let mut engine = Engine::new();
    engine.eval("right=:+").unwrap();
    engine.eval("alias=:right").unwrap();
    engine.eval("right=:#").unwrap();
    engine.eval("f=:,\"alias").unwrap();
    for source in ["alias=:#", "alias=:9", "alias=:/"] {
        engine.eval(source).unwrap();
        assert_eq!(shape(&mut engine, "f i.2 3"), vec![2, 3, 1]);
    }
    engine.eval("left=:-").unwrap();
    engine.eval("f=:left\"+").unwrap();
    assert_eq!(engine.eval("f 7").unwrap().unwrap().int_at(0).unwrap(), -7);
    engine.eval("left=:+").unwrap();
    assert_eq!(engine.eval("f 7").unwrap().unwrap().int_at(0).unwrap(), 7);
    engine.eval("left=:9").unwrap();
    assert_eq!(engine.eval("f 7").unwrap_err().kind(), "domain error");
    engine.eval("future=:,\"uninstalled").unwrap();
    assert_eq!(shape(&mut engine, "future i.2 3"), vec![6]);
    engine.eval("uninstalled=:+").unwrap();
    assert_eq!(shape(&mut engine, "future i.2 3"), vec![6]);
}
#[test]
fn source_rhs_verb_and_name_header_evidence_survive_in_graphs() {
    let mut engine = Engine::new();
    engine.eval("right=:+").unwrap();
    let bound = engine.prepare_semantic("f=:,\"right").unwrap();
    assert!(
        !bound
            .verb_references
            .iter()
            .any(|(name, _)| name == "right")
    );
    let ExprKind::VerbValue(verb) = &bound.program.expression.as_ref().unwrap().kind else {
        panic!()
    };
    let FunctionOperand::Function(rhs) = &verb.entity.operands[1] else {
        panic!()
    };
    assert_eq!(rhs.head, FunctionHead::NameRef("right".into()));
    assert_eq!(rhs.innate_ranks(), Some([0; 3]));
    assert_eq!(bound.program.name_rank_snapshots.len(), 1);
    let snapshot = &bound.program.name_rank_snapshots[0];
    assert_eq!(snapshot.version, engine.binding_version("right"));
    let report = engine.eval_captured("(,\"right) i.2 3");
    report.result.unwrap();
    report.capture.verify().unwrap();
    assert!(report.capture.events.iter().any(
        |event| matches!(event,CaptureEvent::FunctionNameRank{snapshot}
        if snapshot.name=="right"&&snapshot.ranks==Some([0;3]))
    ));
    let graph = Plan::from_capture(&report.capture).unwrap().graph;
    graph.verify().unwrap();
    assert!(
        !graph
            .verb_references
            .iter()
            .any(|(name, _)| name == "right")
    );
    assert!(graph.nodes.iter().any(|node| matches!(
        &node.kind,
        NodeKind::Apply {
            form: GraphForm::Rank {
                rank_spec: None,
                requested_ranks: Some([0, 0, 0]),
                ..
            },
            ..
        }
    )));
    assert!(
        graph
            .name_rank_snapshots
            .iter()
            .any(|snapshot| snapshot.name == "right")
    );
    let plan = engine.analyze_a3("(,\"+) i.2 3").unwrap();
    assert_eq!(
        rustj::logical_executor::execute_closed(&plan)
            .unwrap()
            .unwrap()
            .shape(),
        &[2, 3, 1]
    );
}
#[test]
fn explicit_operand_value_and_implicit_operand_primitive_have_different_headers() {
    let mut engine = Engine::new();
    engine.eval("a=:1 : ',\"u y'").unwrap();
    engine.eval("f=:+a").unwrap();
    assert_eq!(shape(&mut engine, "f i.2 3"), vec![2, 3, 1]);
    engine.eval("a=:1 : ',\"u. y'").unwrap();
    engine.eval("f=:+a").unwrap();
    assert_eq!(shape(&mut engine, "f i.2 3"), vec![6]);
    engine.eval("a=:1 : 0\nright=.u\n(,\"right) y\n)").unwrap();
    engine.eval("f=:+a").unwrap();
    assert_eq!(shape(&mut engine, "f i.2 3"), vec![2, 3, 1]);
}
#[test]
fn asymmetric_dyadic_headers_preserve_j_prefix_agreement_and_recovery() {
    let mut engine = Engine::new();
    assert_eq!(shape(&mut engine, "1 2 (+\"{) i.2 3"), vec![2, 2, 3]);
    assert_eq!(
        engine
            .eval("1 2 3 (+\"(+\"0 0 1)) i.2 3")
            .unwrap_err()
            .kind(),
        "length error"
    );
    assert_eq!(shape(&mut engine, "1 2 (+\"{) i.2 3"), vec![2, 2, 3]);
    assert_eq!(shape(&mut engine, "1 2 (+\"i.) i.2 3"), vec![2, 3]);
}
#[test]
fn supported_primitive_headers_are_intrinsic_without_executing_arguments() {
    for &id in PrimitiveId::ALL {
        let program = rustj::semantic::parse(id.spelling()).unwrap();
        let ExprKind::VerbValue(verb) = program.expression.unwrap().kind else {
            panic!()
        };
        assert_eq!(verb.entity.innate_ranks(), Some(id.innate_ranks()));
    }
    let mut engine = Engine::new();
    for (source, expected) in [
        ("+/", [63; 3]),
        ("+\\", [63, 0, 63]),
        ("+\"_1 0 1", [63, 0, 1]),
        ("[: + -", [63; 3]),
        ("+@:-", [63; 3]),
    ] {
        let bound = engine.prepare_semantic(source).unwrap();
        let ExprKind::VerbValue(verb) = bound.program.expression.unwrap().kind else {
            panic!()
        };
        assert_eq!(verb.entity.innate_ranks(), Some(expected), "{source}");
    }
    let report = engine.eval_captured("ger=:(,<'+')\"0");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let function = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::ConstructionSuccess { function, .. }
                if matches!(
                    function.head,
                    FunctionHead::PrimitiveConjunction(rustj::primitive::ConjunctionId::Rank)
                ) =>
            {
                Some(function)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(function.innate_ranks(), Some([63; 3]));
    engine.eval("first=:+").unwrap();
    assert_eq!(shape(&mut engine, "(,\"first) i.2 3"), vec![2, 3, 1]);
}
#[test]
fn static_catalog_header_evidence_analyzes_large_arrays_without_input_payloads() {
    use rustj::{
        facts::TypeFact, j_graph_ir::GraphFacts, static_analysis::StaticAnalyzer, types::DType,
    };
    let mut analyzer = StaticAnalyzer::new();
    analyzer
        .declare_noun(
            "input",
            GraphFacts {
                shape: Some(vec![1_000_000_000_000, 3]),
                rank: Some(2),
                dtype: TypeFact::Exact(DType::Int),
            },
        )
        .unwrap();
    analyzer
        .declare_function("right", rustj::semantic::FunctionPartOfSpeech::Verb)
        .unwrap();
    assert_eq!(
        analyzer.analyze("(,\"right) input").unwrap_err().kind(),
        "unsupported"
    );
    analyzer
        .declare_primitive_verb("right", PrimitiveId::Add)
        .unwrap();
    let report = analyzer.analyze("(,\"right) input").unwrap();
    report.graph.verify().unwrap();
    assert_eq!(
        report.graph.nodes[report.graph.result.unwrap().0]
            .facts
            .shape,
        Some(vec![1_000_000_000_000, 3, 1])
    );
    assert!(
        report
            .graph
            .nodes
            .iter()
            .all(|node| !matches!(node.kind, NodeKind::Literal(_)))
    );
    assert!(
        !report
            .graph
            .verb_references
            .iter()
            .any(|(name, _)| name == "right")
    );
    assert!(
        report
            .graph
            .name_rank_snapshots
            .iter()
            .any(|snapshot| snapshot.name == "right"
                && snapshot.ranks == Some([0; 3])
                && snapshot.version == Some(analyzer.binding("right").unwrap().version))
    );
    let call = analyzer.analyze("right input").unwrap();
    assert!(
        call.graph
            .verb_references
            .iter()
            .any(|(name, _)| name == "right")
    );
    assert!(call.boundaries.iter().any(|boundary| boundary.reason
        == rustj::static_analysis::BoundaryReason::FunctionSpecialization));
}

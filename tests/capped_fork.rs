use rustj::{
    Engine,
    j_graph_ir::{GraphForm, GraphHint, Plan, RegionKind},
    parser_capture::CaptureEvent,
    primitive::PrimitiveId,
    semantic::{ExprKind, ForkSemantics, FunctionHead, FunctionOperand},
};

fn scalar(engine: &mut Engine, source: &str) -> i64 {
    engine.eval(source).unwrap().unwrap().int_at(0).unwrap()
}
#[test]
fn cap_valence_literal_and_single_name_construction_match_j() {
    let mut engine = Engine::new();
    for source in ["[: 7", "3 [: 7", "[: 'x'", "[: i.0"] {
        assert_eq!(
            engine.eval(source).unwrap_err().kind(),
            "valence error",
            "{source}"
        );
    }
    assert_eq!(scalar(&mut engine, "([: + -) 7"), -7);
    assert_eq!(scalar(&mut engine, "3 ([: + -) 7"), -4);
    engine.eval("capname=:[:").unwrap();
    let result = engine.eval_captured("f=:capname + -");
    result.result.unwrap();
    result.capture.verify().unwrap();
    let source_function = result
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::ConstructionSuccess { function, .. }
                if function.head == FunctionHead::Fork =>
            {
                Some(function)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(source_function.fork_semantics, Some(ForkSemantics::Capped));
    let FunctionOperand::Function(first) = &source_function.operands[0] else {
        panic!()
    };
    assert_eq!(first.head, FunctionHead::NameRef("capname".into()));
    assert_eq!(&result.capture.source()[first.span.clone()], "capname");
    assert!(result.capture.events.iter().any(|event| matches!(event,
        CaptureEvent::ForkNameResolved {read, capped:true, ..} if read.name == "capname")));
    for source in ["capname=:+", "capname=:9", "capname=:/"] {
        engine.eval(source).unwrap();
        assert_eq!(scalar(&mut engine, "f 7"), -7);
    }
    engine.eval("capname=:[:").unwrap();
    engine.eval("alias=:capname").unwrap();
    engine.eval("ordinary=:alias + -").unwrap();
    assert_eq!(
        engine.eval("ordinary 7").unwrap_err().kind(),
        "valence error"
    );
    engine.eval("capname=:+").unwrap();
    assert_eq!(scalar(&mut engine, "ordinary 7"), 0);
    engine.eval("capname=:9").unwrap();
    assert_eq!(
        engine.eval("ordinary 7").unwrap_err().kind(),
        "domain error"
    );
}
#[test]
fn named_cap_static_and_captured_graph_keep_dependencies_and_pipeline_only() {
    let mut engine = Engine::new();
    engine.eval("capname=:[:").unwrap();
    let bound = engine.prepare_semantic("capname + -").unwrap();
    assert!(bound.reads.iter().any(|read| read.name == "capname"));
    assert!(
        !bound
            .verb_references
            .iter()
            .any(|(name, _)| name == "capname")
    );
    let ExprKind::VerbValue(verb) = &bound.program.expression.as_ref().unwrap().kind else {
        panic!()
    };
    let (form, hints) = rustj::j_graph_ir::classify_function(&verb.entity);
    let GraphForm::Pipeline { stages } = form else {
        panic!()
    };
    assert_eq!(stages.len(), 2);
    assert_eq!(
        stages[0].head,
        FunctionHead::PrimitiveVerb(PrimitiveId::Subtract)
    );
    assert_eq!(
        stages[1].head,
        FunctionHead::PrimitiveVerb(PrimitiveId::Add)
    );
    assert!(!hints.contains(GraphHint::ParallelBranchCandidate));
    assert!(!hints.contains(GraphHint::RetainedValueCandidate));
    let report = engine.eval_captured("(capname + -) 1 2 3");
    report.result.unwrap();
    let graph = Plan::from_capture(&report.capture).unwrap().graph;
    graph.verify().unwrap();
    assert_eq!(graph.fork_name_reads.len(), 1);
    assert!(
        !graph
            .verb_references
            .iter()
            .any(|(name, _)| name == "capname")
    );
    let region = graph
        .regions
        .iter()
        .find(|region| region.function.head == FunctionHead::Fork)
        .unwrap();
    assert!(matches!(region.kind, RegionKind::Pipeline { .. }));
    assert!(!region.hints.contains(GraphHint::ParallelBranchCandidate));
    assert!(!region.hints.contains(GraphHint::RetainedValueCandidate));
    assert_eq!(region.function.fork_semantics, Some(ForkSemantics::Capped));
}
#[test]
fn capped_call_runs_inner_before_monadic_outer_and_preserves_effects_on_failure() {
    let mut engine = Engine::new();
    engine.eval("count=:0").unwrap();
    engine
        .eval("inner=:1 : 0\ncount=:(count*10)+1\nu y\n)")
        .unwrap();
    engine
        .eval("outer=:1 : 0\ncount=:(count*10)+2\nu y\n)")
        .unwrap();
    engine.eval("h=:-inner").unwrap();
    engine.eval("g=:+outer").unwrap();
    engine.eval("f=:[: g h").unwrap();
    assert_eq!(scalar(&mut engine, "f 7"), -7);
    assert_eq!(scalar(&mut engine, "count"), 12);
    engine.eval("count=:0").unwrap();
    engine.eval("h=:[:inner").unwrap();
    assert_eq!(engine.eval("f 7").unwrap_err().kind(), "valence error");
    assert_eq!(scalar(&mut engine, "count"), 1);
    engine.eval("h=:-inner").unwrap();
    engine.eval("g=:[:outer").unwrap();
    assert_eq!(engine.eval("f 7").unwrap_err().kind(), "valence error");
    assert_eq!(scalar(&mut engine, "count"), 112);
    engine.eval("g=:+outer").unwrap();
    assert_eq!(scalar(&mut engine, "f 7"), -7);
    assert_eq!(scalar(&mut engine, "count"), 11212);
}
#[test]
fn cap_operand_substitution_observes_current_local_constructor_scope() {
    let mut engine = Engine::new();
    engine.eval("a=:1 : '(u + -) y'").unwrap();
    engine.eval("f=:[:a").unwrap();
    assert_eq!(scalar(&mut engine, "f 7"), -7);
    engine.eval("capname=:+").unwrap();
    engine
        .eval("b=:1 : 0\ncapname=.u\n(capname + -) y\n)")
        .unwrap();
    engine.eval("f=:[:b").unwrap();
    assert_eq!(scalar(&mut engine, "f 7"), -7);
    assert_eq!(scalar(&mut engine, "capname 7"), 7);
}

#[test]
fn named_cap_in_gerund_ar_retains_carrier_provenance() {
    let mut engine = Engine::new();
    engine.eval("capname=:[:").unwrap();
    engine
        .eval("capar=:(<'3'),<((<'capname'),(<'+'),<'-')")
        .unwrap();
    let report = engine.eval_captured("f=:(,<capar)\\");
    report.result.unwrap();
    report.capture.verify().unwrap();
    assert!(report.capture.events.iter().any(|event| matches!(event,
        CaptureEvent::ForkNameResolved { read, capped: true, .. } if read.name == "capname")));
}

use rustj::{
    Engine,
    contracts::{self, Effect, Valence},
};

fn scalar(engine: &mut Engine, source: &str, expected: i64) {
    assert_eq!(
        engine.eval(source).unwrap().unwrap().int_at(0).unwrap(),
        expected,
        "{source}"
    );
}

#[test]
fn returned_implicit_operands_fix_once_and_ordinary_names_stay_late() {
    let mut engine = Engine::new();
    for source in [
        "a=:1 : 'u.'",
        "f=:-a",
        "r=:1 : 'u./'",
        "sum=:+r",
        "c=:2 : 'v.'",
        "g=:+c -",
    ] {
        engine.eval(source).unwrap();
    }
    scalar(&mut engine, "f 7", -7);
    scalar(&mut engine, "3 f 7", -4);
    scalar(&mut engine, "sum 1 2 3", 6);
    scalar(&mut engine, "g 7", -7);
    for source in ["name=:+", "late=:name a", "name=:-"] {
        engine.eval(source).unwrap();
    }
    scalar(&mut engine, "late 7", -7);
    engine.eval("name=:0").unwrap();
    assert_eq!(engine.eval("late 7").unwrap_err().kind(), "domain error");
    assert_eq!(engine.eval("3 a").unwrap_err().kind(), "domain error");
    scalar(&mut engine, "f 7", -7);
}

#[test]
fn return_fix_uses_final_local_binding_without_fixing_global_publication() {
    let mut engine = Engine::new();
    engine.eval("a=:1 : 0\nu=.-\nu.\n)").unwrap();
    engine.eval("f=:+a").unwrap();
    scalar(&mut engine, "f 7", -7);
    engine.eval("a=:1 : 0\npub=:u.\nu.\n)").unwrap();
    engine.eval("f=:-a").unwrap();
    scalar(&mut engine, "f 7", -7);
    // A raw publication has no operand frame after the modifier returns.
    assert_eq!(engine.eval("pub 7").unwrap_err().kind(), "value error");
    engine.eval("a=:1 : 'v.'").unwrap();
    assert_eq!(engine.eval("f=:+a").unwrap_err().kind(), "unsupported");
    scalar(&mut engine, "f 7", -7);
}

#[test]
fn direct_definitions_count_implicit_primitives_as_operands() {
    let mut engine = Engine::new();
    engine.eval("a=:{{u.}}").unwrap();
    engine.eval("f=:-a").unwrap();
    scalar(&mut engine, "f 7", -7);
    engine.eval("c=:{{v.}}").unwrap();
    engine.eval("f=:+c -").unwrap();
    scalar(&mut engine, "f 7", -7);
    engine.eval("a=:1 : 'u. + u.'").unwrap();
    let returned = engine.eval_captured("f=:-a");
    returned.result.unwrap();
    let fork = returned
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            rustj::parser_capture::CaptureEvent::FunctionResult { function, .. }
                if matches!(function.head, rustj::semantic::FunctionHead::Fork) =>
            {
                Some(function)
            }
            _ => None,
        })
        .unwrap();
    let [
        rustj::semantic::FunctionOperand::Function(left),
        _,
        rustj::semantic::FunctionOperand::Function(right),
    ] = fork.operands.as_slice()
    else {
        panic!()
    };
    assert!(std::sync::Arc::ptr_eq(left, right));
    engine.eval("a=:1 : 'u.\"0'").unwrap();
    let ranked = engine.eval_captured("f=:+a");
    ranked.result.unwrap();
    assert!(ranked.capture.events.iter().any(|event| matches!(event,
        rustj::parser_capture::CaptureEvent::FunctionResult { function, .. }
        if matches!(function.head, rustj::semantic::FunctionHead::PrimitiveConjunction(rustj::primitive::ConjunctionId::Rank)))));
    for spelling in ["u.", "v."] {
        let graph = engine.analyze_j_graph(&format!("{spelling} 7")).unwrap();
        graph.verify().unwrap();
        let result = graph.result.unwrap();
        let rustj::j_graph_ir::NodeKind::Apply { rules, .. } = &graph.nodes[result.0].kind else {
            panic!()
        };
        assert_eq!(
            rules.effect,
            rustj::j_graph_ir::GraphRuleRef::DynamicOrUnknown
        );
        assert!(graph.nodes[result.0].facts.shape.is_none());
        for valence in [Valence::Monad, Valence::Dyad] {
            assert_eq!(contracts::lookup(spelling, valence), contracts::unknown());
            assert_eq!(contracts::lookup(spelling, valence).effect, Effect::Unknown);
        }
    }
}

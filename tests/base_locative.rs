use rustj::{
    Engine,
    frontend_context::{FoundScope, NameUseId, ScopeSearch, SimpleNameGuard},
    parser_capture::CaptureEvent,
};

fn scalar(engine: &mut Engine, source: &str, expected: i64) {
    assert_eq!(
        engine.eval(source).unwrap().unwrap().int_at(0).unwrap(),
        expected,
        "{source}"
    );
}

#[test]
fn base_nouns_share_the_base_binding_and_preserve_snapshots() {
    let mut engine = Engine::new();
    engine.eval("a=:i.4").unwrap();
    let version = engine.binding_version("a");
    engine.eval("saved=:a__").unwrap();
    engine.eval("a__=.a__+10").unwrap();
    assert_ne!(engine.binding_version("a"), version);
    assert!(engine.binding_version("a__").is_none()); // No flattened duplicate key.
    assert_eq!(
        engine.eval("a").unwrap().unwrap().json(),
        engine.eval("a__").unwrap().unwrap().json()
    );
    assert_eq!(
        engine.eval("saved").unwrap().unwrap().json(),
        engine.eval("i.4").unwrap().unwrap().json()
    );
    engine.eval("newbase__=:7").unwrap();
    scalar(&mut engine, "newbase", 7);
}

#[test]
fn base_reads_and_writes_bypass_local_shadowing() {
    for semantic in [false, true] {
        let mut engine = Engine::new();
        engine.eval("a=:7").unwrap();
        engine.eval("f=:3 : '(a=.9)+a__'").unwrap();
        let result = if semantic {
            engine.eval_semantic_reference("f 0")
        } else {
            engine.eval("f 0")
        };
        assert_eq!(result.unwrap().unwrap().int_at(0).unwrap(), 16);
        scalar(&mut engine, "a", 7);
        engine.eval("f=:3 : 0\na=.9\na__=.11\na+a__\n)").unwrap();
        let result = if semantic {
            engine.eval_semantic_reference("f 0")
        } else {
            engine.eval("f 0")
        };
        assert_eq!(result.unwrap().unwrap().int_at(0).unwrap(), 20);
        scalar(&mut engine, "a", 11);
    }
}

#[test]
fn base_lookup_snapshots_follow_right_to_left_effect_order() {
    let mut engine = Engine::new();
    engine.eval("a=:1").unwrap();
    scalar(&mut engine, "a__+(a__=:2)", 4);
    scalar(&mut engine, "(a__=:3)+a__", 5);
    let error = engine.eval("missing__+(a__=:4)").unwrap_err();
    assert_eq!(error.kind(), "unsupported"); // Undefined future-function reference is still outside this slice.
    scalar(&mut engine, "a", 4);
    let error = engine.eval("a__=:1 2+1 2 3").unwrap_err();
    assert_eq!(error.kind(), "length error");
    scalar(&mut engine, "a", 4);
}

#[test]
fn base_capture_records_actual_scope_and_rejects_simple_name_guard() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let report = engine.eval_captured("a__+1");
    assert_eq!(report.result.unwrap().unwrap().int_at(0).unwrap(), 8);
    report.capture.verify().unwrap();
    assert!(rustj::j_graph_ir::Plan::from_capture(&report.capture).is_err());
    let context = report.capture.frontend.as_ref().unwrap();
    let lookup = context.name_uses[0].lookup.as_ref().unwrap();
    assert_eq!(lookup.search, ScopeSearch::BaseLocaleOnly);
    assert!(matches!(lookup.found, FoundScope::Global(_)));
    assert!(SimpleNameGuard::from_name_use(context, NameUseId(0)).is_err());
    let report = engine.eval_captured("a__=:8");
    report.result.unwrap();
    report.capture.verify().unwrap();
    assert!(
        report
            .capture
            .events
            .iter()
            .any(|event| matches!(event, CaptureEvent::Commit {name, ..} if name == "a__"))
    );
}

#[test]
fn function_locatives_and_static_admission_remain_explicit_boundaries() {
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    let version = engine.binding_version("f");
    for source in ["f__", "f__=:*", "a_probe_=:*", "f___:"] {
        assert_eq!(
            engine.eval(source).unwrap_err().kind(),
            "unsupported",
            "{source}"
        );
        assert_eq!(engine.binding_version("f"), version);
    }
    engine.eval("a=:7").unwrap();
    assert_eq!(
        engine.parse_frontend("a__").unwrap_err().error.kind(),
        "unsupported"
    );
    assert_eq!(
        engine.prepare_semantic("a__").unwrap_err().kind(),
        "unsupported"
    );
}

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
fn named_function_locative_falls_back_to_z_until_own_binding_shadows_it() {
    let mut engine = Engine::new();
    engine.eval("route_z_=:*").unwrap();
    scalar(&mut engine, "route_probe_ 4", 1);

    engine.eval("route_probe_=:7").unwrap();
    scalar(&mut engine, "route_probe_", 7);
    scalar(&mut engine, "route_z_ 4", 1);

    engine.eval("route_probe_=:-").unwrap();
    scalar(&mut engine, "route_probe_ 4", -4);
    scalar(&mut engine, "route_z_ 4", 1);
}

#[test]
fn named_function_rebinding_obeys_right_to_left_sentence_effects() {
    let mut engine = Engine::new();
    engine.eval("op_probe_=:-").unwrap();
    engine.eval("change=:{{\nop_probe_=:*\ny\n}}").unwrap();

    // The rightmost call mutates the named function before the left verb is used.
    scalar(&mut engine, "op_probe_ 4 + change 0", 1);
    scalar(&mut engine, "op_probe_ 4", 1);

    engine.eval("op_probe_=:-").unwrap();
    // The rightmost function call still sees minus, before the left call mutates it.
    scalar(&mut engine, "change 0 + op_probe_ 4", -4);
    scalar(&mut engine, "op_probe_ 4", 1);
}

#[test]
fn named_locative_alias_retains_verb_pos_and_rejects_noun_rebinding() {
    let mut engine = Engine::new();
    engine.eval("op_probe_=:+").unwrap();
    engine.eval("alias=:op_probe_").unwrap();
    scalar(&mut engine, "alias 4", 4);

    engine.eval("op_probe_=:7").unwrap();
    // A constructed alias retains the verb POS. Its target lookup is late,
    // so changing the target into a noun must not silently turn it into a noun.
    assert_eq!(engine.eval("alias 4").unwrap_err().kind(), "domain error");
    scalar(&mut engine, "op_probe_", 7);

    engine.eval("op_probe_=:-").unwrap();
    scalar(&mut engine, "alias 4", -4);
}

#[test]
fn direct_definition_reparses_named_locative_after_pos_restoration() {
    let mut engine = Engine::new();
    engine.eval("op_probe_=:+").unwrap();
    engine.eval("call=:{{op_probe_ y}}").unwrap();
    scalar(&mut engine, "call 4", 4);

    engine.eval("op_probe_=:7").unwrap();
    scalar(&mut engine, "op_probe_", 7);
    // No stale noun class should remain after a later verb assignment.
    engine.eval("op_probe_=:-").unwrap();
    scalar(&mut engine, "call 4", -4);
    engine.eval("op_probe_=:+").unwrap();
    scalar(&mut engine, "call 4", 4);
}

#[test]
fn right_hand_pos_restoration_allows_later_left_hand_direct_call() {
    let mut engine = Engine::new();
    for source in [
        "op_probe_=:+",
        "call=:{{op_probe_ y}}",
        "restore=:{{\\nop_probe_=:+\\n0\\n}}",
    ] {
        engine.eval(source).unwrap();
    }
    engine.eval("op_probe_=:7").unwrap();
    scalar(&mut engine, "op_probe_", 7);
    // J evaluates the RHS first, restoring the verb before the direct body
    // performs a fresh name lookup. This positive path does not classify the
    // disputed syntax-vs-domain error when the noun remains bound.
    scalar(&mut engine, "call 4 + restore 0", 4);
    scalar(&mut engine, "op_probe_ 4", 4);
    scalar(&mut engine, "call 5", 5);
}

#[test]
#[ignore = "pending pinned J oracle for POS-change error precedence (RustJ: syntax error, expected domain error)"]
fn named_function_pos_rebinding_observes_right_to_left_call_order() {
    let mut engine = Engine::new();
    for source in [
        "op_probe_=:+",
        "call=:{{op_probe_ y}}",
        "nounify=:{{\nop_probe_=:7\n0\n}}",
        "restore=:{{\nop_probe_=:+\n0\n}}",
    ] {
        engine.eval(source).unwrap();
    }
    scalar(&mut engine, "call 4", 4);

    // RHS changes the verb to a noun before the LHS's late-bound call.
    assert_eq!(
        engine.eval("call 4 + nounify 0").unwrap_err().kind(),
        "domain error"
    );
    scalar(&mut engine, "op_probe_", 7);
    // RHS restores the function before the LHS call.
    scalar(&mut engine, "call 4 + restore 0", 4);
    scalar(&mut engine, "op_probe_ 4", 4);

    engine.eval("op_probe_=:7").unwrap();
    // The RHS call fails first; restore does not execute on the left.
    assert_eq!(
        engine.eval("restore 0 + call 4").unwrap_err().kind(),
        "domain error"
    );
    scalar(&mut engine, "op_probe_", 7);
}

#[test]
fn failed_named_function_rhs_preserves_binding_but_not_prior_effects() {
    let mut engine = Engine::new();
    engine.eval("op_probe_=:-").unwrap();
    engine.eval("count=:0").unwrap();
    engine
        .eval("fails=:{{\ncount=:count+1\n1 2+1 2 3\n}}")
        .unwrap();

    assert_eq!(
        engine.eval("op_probe_=:1 2+1 2 3").unwrap_err().kind(),
        "length error"
    );
    scalar(&mut engine, "count", 0);
    scalar(&mut engine, "op_probe_ 4", -4);

    assert_eq!(
        engine.eval("op_probe_=:fails 0").unwrap_err().kind(),
        "length error"
    );
    // Effects completed before the failing RHS remain; the target stays intact.
    scalar(&mut engine, "count", 1);
    scalar(&mut engine, "op_probe_ 4", -4);
}

#[test]
fn function_locatives_write_runtime_bindings_while_static_admission_stays_bounded() {
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    let version = engine.binding_version("f");
    engine.eval("f__=:*").unwrap();
    assert_ne!(engine.binding_version("f"), version);
    assert!(engine.binding_version("f__").is_none()); // Not a flattened key.
    scalar(&mut engine, "f__ 3", 1);
    scalar(&mut engine, "f 3", 1);

    engine.eval("a_probe_=:*").unwrap();
    scalar(&mut engine, "a_probe_ 3", 1);
    // The named-locale binding must not overwrite base.
    assert!(engine.binding_version("a").is_none());
    assert_eq!(engine.eval("f___:").unwrap_err().kind(), "unsupported");

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

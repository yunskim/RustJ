use rustj::{
    Engine,
    frontend_context::{NameGuardCheck, NamePolicy, NameUseId, NodeKind, SimpleNameGuard},
    parser_capture::CaptureEvent,
    semantic::ExprKind,
};

fn scalar(e: &mut Engine, source: &str, n: i64) {
    assert_eq!(
        e.eval(source).unwrap().unwrap().json(),
        format!("{{\"type\":4,\"shape\":[],\"data\":[{n}]}}")
    );
}

#[test]
fn frontend_preserves_abandon_without_reading_or_deleting() {
    let queue = rustj::enqueuer::enqueue("a_:").unwrap();
    assert!(queue[0].flags.abandon_name && queue[0].flags.lookup_name);
    assert_eq!(queue[0].span, 0..3);
    let program = rustj::parser::parse_frontend("a_:").unwrap();
    assert!(
        matches!(program.expression.as_ref().unwrap().kind, ExprKind::TakeName { ref name, single_word: true } if name == "a")
    );
    let context = program.frontend.as_ref().unwrap();
    context.verify().unwrap();
    assert!(
        context
            .nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::TakeName(_)))
    );
    assert_eq!(context.name_uses[0].policy, NamePolicy::CaptureAndAbandon);
    let mut e = Engine::new();
    e.eval("a=:7").unwrap();
    assert_eq!(e.prepare_semantic("a_:").unwrap_err().kind(), "unsupported");
    assert_eq!(e.analyze_j_graph("a_:").unwrap_err().kind(), "unsupported");
    scalar(&mut e, "a", 7);
}

#[test]
fn runtime_order_and_failures_preserve_completed_deletion() {
    let mut e = Engine::new();
    e.eval("a=:7").unwrap();
    scalar(&mut e, "a_:+a", 14);
    assert_eq!(e.eval("a").unwrap_err().kind(), "value error");
    e.eval("a=:7").unwrap();
    assert_eq!(e.eval("(a+a_:)0").unwrap_err().kind(), "value error");
    assert_eq!(e.eval("a").unwrap_err().kind(), "value error");
    e.eval("a=:7").unwrap();
    assert_eq!(e.eval("a_:+1 2+1 2 3").unwrap_err().kind(), "length error");
    scalar(&mut e, "a", 7);
    assert_eq!(e.eval("missing_:").unwrap_err().kind(), "value error");
    e.eval("a=:1 2 3").unwrap();
    e.eval("b=:a").unwrap();
    assert_eq!(
        e.eval("a_:").unwrap().unwrap().json(),
        e.eval("b").unwrap().unwrap().json()
    );
    assert_eq!(e.eval("a").unwrap_err().kind(), "value error");
}

#[test]
fn function_and_modifier_values_survive_abandon_and_rebinding() {
    let mut e = Engine::new();
    e.eval("f=:+").unwrap();
    e.eval("g=:f_:").unwrap();
    e.eval("f=:*").unwrap();
    scalar(&mut e, "g 3", 3);
    e.eval("adv=:/").unwrap();
    e.eval("sum=:+adv_:").unwrap();
    scalar(&mut e, "sum 1 2 3", 6);
    assert_eq!(e.eval("adv").unwrap_err().kind(), "value error");
    e.eval("conj=:@:").unwrap();
    assert_eq!(e.eval("h=:-conj_:+").unwrap_err().kind(), "unsupported");
    // Unsupported is an admission boundary, never a successful C emulation.
    e.eval("h=:-conj+").unwrap();
    scalar(&mut e, "h 3", -3);
}

#[test]
fn local_single_word_fast_path_and_assignment_target_are_distinct() {
    for definition in ["f=:{{a=.9\na_:\na}}", "f=:3 : 0\na=.9\na_:\na\n)"] {
        let mut e = Engine::new();
        e.eval("a=:7").unwrap();
        e.eval(definition).unwrap();
        scalar(&mut e, "f 0", 9);
        scalar(&mut e, "a", 7);
    }
    for definition in ["f=:{{a=.9\n(a_:)\na}}", "f=:3 : 0\na=.9\n(a_:)\na\n)"] {
        let mut e = Engine::new();
        e.eval("a=:7").unwrap();
        e.eval(definition).unwrap();
        scalar(&mut e, "f 0", 7);
        scalar(&mut e, "a", 7);
    }
    let mut e = Engine::new();
    e.eval("a_:=:9").unwrap();
    scalar(&mut e, "a", 9);
    e.eval("f=:{{a_:=.3\na}}").unwrap();
    scalar(&mut e, "f 0", 3);
    scalar(&mut e, "a", 9);
}

#[test]
fn declared_unbound_local_deletes_found_global_without_caller_capture() {
    let mut e = Engine::new();
    e.eval("a=:7").unwrap();
    e.eval("f=:{{a_:\na=.9\na}}").unwrap();
    scalar(&mut e, "f 0", 9);
    assert_eq!(e.eval("a+0").unwrap_err().kind(), "value error");
    e.eval("a=:7").unwrap();
    e.eval("inner=:{{a_:}}").unwrap();
    e.eval("outer=:{{a=.99\ninner y}}").unwrap();
    scalar(&mut e, "outer 0", 7);
    assert_eq!(e.eval("a+0").unwrap_err().kind(), "value error");
}

#[test]
fn abandoning_an_alias_keeps_its_inner_late_reference() {
    let mut e = Engine::new();
    e.eval("base=:+").unwrap();
    e.eval("alias=:base").unwrap();
    e.eval("saved=:alias_:").unwrap();
    e.eval("base=:-").unwrap();
    scalar(&mut e, "saved 3", -3);
    assert_eq!(e.eval("alias 3").unwrap_err().kind(), "value error");
    // Result lookup must report the name deleted earlier in this sentence.
    e.eval("a=:7").unwrap();
    assert_eq!(e.eval("a+a_:").unwrap_err().kind(), "value error");
    assert_eq!(e.eval("a+0").unwrap_err().kind(), "value error");
}

#[test]
fn capture_retains_pre_delete_binding_and_cannot_become_pure_graph() {
    let mut e = Engine::new();
    e.eval("a=:7").unwrap();
    let before = e.eval_captured("a");
    let guard =
        SimpleNameGuard::from_name_use(before.capture.frontend.as_ref().unwrap(), NameUseId(0))
            .unwrap();
    let captured = e.eval_captured("a_:");
    captured.result.unwrap();
    captured.capture.verify().unwrap();
    let context = captured.capture.frontend.as_ref().unwrap();
    context.verify().unwrap();
    assert!(SimpleNameGuard::from_name_use(context, NameUseId(0)).is_err());
    assert!(captured.capture.events.iter().any(|event| matches!(event, CaptureEvent::Abandon { deleted: true, lookup, .. } if lookup.binding_generation.is_some() && lookup.binding_version.is_some())));
    assert!(captured.capture.requires_ordered_effect_graph());
    assert!(rustj::j_graph_ir::Plan::from_capture(&captured.capture).is_err());
    e.eval("a=:7").unwrap();
    assert_ne!(e.check_name_guard(&guard), NameGuardCheck::ValidAtCheck);
}

#[test]
fn read_only_loop_abandon_is_bounded_and_not_catchable() {
    let mut e = Engine::new();
    e.eval("f=:{{for_i. i.1 do. i_index_: end.}}").unwrap();
    scalar(&mut e, "f 0", 0);
    e.eval("f=:{{try. for_i. i.1 do. (i_index_:) end. catch. 99 end.}}")
        .unwrap();
    assert_eq!(e.eval("f 0").unwrap_err().kind(), "unsupported");
}

#[test]
fn explicit_conjunction_abandon_returns_value_and_deletes_found_binding() {
    let mut e = Engine::new();
    e.eval("c=:2 : 'u@:v'").unwrap();
    let captured = e.eval_captured("h=:-c_:+");
    captured.result.unwrap();
    captured.capture.verify().unwrap();
    captured
        .capture
        .frontend
        .as_ref()
        .unwrap()
        .verify()
        .unwrap();
    assert!(captured.capture.events.iter().any(
        |event| matches!(event, CaptureEvent::Abandon { name, deleted: true, .. } if name == "c")
    ));
    scalar(&mut e, "h 3", -3);
    assert_eq!(e.eval("c 0").unwrap_err().kind(), "value error");
    e.eval("c=:2 : 'u@:v'").unwrap();
    e.eval("saved=:c_:").unwrap();
    e.eval("c=:2 : 'u@:u'").unwrap();
    e.eval("h=:-saved+").unwrap();
    scalar(&mut e, "h 3", -3);
}

#[test]
fn local_explicit_conjunction_preserves_single_word_exception_and_global_fallback() {
    for definition in [
        "f=:{{c=.2 : 'u@:v'\nh=.-c_:+\nh y}}",
        "f=:3 : 0\nc=.2 : 'u@:v'\nh=.-c_:+\nh y\n)",
    ] {
        let mut e = Engine::new();
        e.eval("c=:7").unwrap();
        e.eval(definition).unwrap();
        scalar(&mut e, "f 3", -3);
        scalar(&mut e, "c", 7);
    }
    let mut e = Engine::new();
    e.eval("f=:{{c=.2 : 'u@:v'\ntry.\nc_:\ncatch.\nh=.-c+\nend.\nh y}}")
        .unwrap();
    scalar(&mut e, "f 3", -3);
    e.eval("c=:2 : 'u@:v'").unwrap();
    e.eval("f=:{{h=.-c_:+\nc=.7\nh y}}").unwrap();
    scalar(&mut e, "f 3", -3);
    assert_eq!(e.eval("c 0").unwrap_err().kind(), "value error");
}

#[test]
fn failed_explicit_conjunction_construction_keeps_abandon_effect() {
    let mut e = Engine::new();
    e.eval("c=:2 : 'u+v'").unwrap();
    let captured = e.eval_captured("h=:1 2 c_:1 2 3");
    assert_eq!(captured.result.unwrap_err().kind(), "length error");
    captured.capture.verify().unwrap();
    captured
        .capture
        .frontend
        .as_ref()
        .unwrap()
        .verify()
        .unwrap();
    assert_eq!(e.eval("c 0").unwrap_err().kind(), "value error");
    assert_eq!(e.eval("h 0").unwrap_err().kind(), "value error");
    e.eval("c=:2 : 'u+v'").unwrap();
    e.eval("f=:{{try. h=.1 2 c_:1 2 3 catch. 99 end.}}")
        .unwrap();
    scalar(&mut e, "f 0", 99);
    assert_eq!(e.eval("c 0").unwrap_err().kind(), "value error");
}

#[test]
fn engine_frontend_exposes_deferred_function_transport_before_binding() {
    use rustj::semantic::{FunctionHead, FunctionPartOfSpeech};
    let mut e = Engine::new();
    for (setup, source, name, pos) in [
        ("f=:+", "g=:f_:", "f", FunctionPartOfSpeech::Verb),
        (
            "adv=:/",
            "saved=:adv_:",
            "adv",
            FunctionPartOfSpeech::Adverb,
        ),
        (
            "c=:2 : 'u@:v'",
            "saved=:c_:",
            "c",
            FunctionPartOfSpeech::Conjunction,
        ),
    ] {
        e.eval(setup).unwrap();
        let version = e.binding_version(name);
        let program = e.parse_frontend(source).unwrap();
        let context = program.frontend.as_ref().unwrap();
        context.verify().unwrap();
        assert!(context.complete);
        let function = match &program.expression.as_ref().unwrap().kind {
            ExprKind::VerbValue(v) => &v.entity,
            ExprKind::ModifierValue(f) => f,
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(function.result_pos, pos);
        assert!(
            matches!(&function.head, FunctionHead::TakeName { name: base, single_word: false } if base == name)
        );
        assert_eq!(
            e.prepare_semantic(source).unwrap_err().kind(),
            "unsupported"
        );
        assert_eq!(e.analyze_j_graph(source).unwrap_err().kind(), "unsupported");
        assert_eq!(
            e.analyze_compilation_diagnostic(source).unwrap_err().kind(),
            "unsupported"
        );
        assert_eq!(e.binding_version(name), version);
    }
    scalar(&mut e, "f 3", 3);
    scalar(&mut e, "+adv 1 2 3", 6);
    e.eval("h=:-c+").unwrap();
    scalar(&mut e, "h 3", -3);
}

#[test]
fn deferred_modifier_boundary_retains_name_and_pending_parser_action() {
    let mut e = Engine::new();
    e.eval("c=:2 : 'u@:v'").unwrap();
    let failure = e.parse_frontend("h=:-c_:+").unwrap_err();
    assert_eq!(failure.error.kind(), "unsupported");
    failure.context.verify().unwrap();
    assert_eq!(
        failure.context.pending.as_ref().unwrap().row,
        rustj::parser::ParseRow::Conjunction
    );
    assert_eq!(
        failure.context.name_uses[0].policy,
        NamePolicy::CaptureAndAbandon
    );
    e.eval("h=:-c+").unwrap();
    scalar(&mut e, "h 3", -3);
}

#[test]
fn deferred_abandon_does_not_guess_first_fork_operand_is_not_cap() {
    let mut e = Engine::new();
    e.eval("cap=:[:").unwrap();
    let before = e.binding_version("cap");
    let failure = e.parse_frontend("f=:(cap_: + *)").unwrap_err();
    assert_eq!(failure.error.kind(), "unsupported");
    failure.context.verify().unwrap();
    assert_eq!(
        failure.context.pending.as_ref().unwrap().row,
        rustj::parser::ParseRow::Fork
    );
    assert_eq!(e.binding_version("cap"), before);
    e.eval("f=:(cap_: + *)").unwrap();
    scalar(&mut e, "f 3", 1);
    assert_eq!(e.eval("cap 0").unwrap_err().kind(), "value error");
}

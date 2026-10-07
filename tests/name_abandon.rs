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

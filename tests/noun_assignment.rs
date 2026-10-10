use rustj::{Engine, frontend_context::NodeKind, parser_capture::CaptureEvent};

fn json(e: &mut Engine, source: &str) -> String {
    e.eval(source).unwrap().unwrap().json()
}

#[test]
fn single_string_target_preserves_whole_rhs_and_late_function_reference() {
    let mut e = Engine::new();
    assert!(e.eval("' a '=:7 8").unwrap().is_none());
    assert_eq!(
        json(&mut e, "a"),
        "{\"type\":4,\"shape\":[2],\"data\":[7,8]}"
    );
    e.eval("op=:+").unwrap();
    e.eval("'f'=:op").unwrap();
    e.eval("op=:-").unwrap();
    assert_eq!(
        json(&mut e, "f 3"),
        "{\"type\":4,\"shape\":[],\"data\":[-3]}"
    );
    e.eval("'adv'=:/").unwrap();
    assert_eq!(
        json(&mut e, "+adv 1 2 3"),
        "{\"type\":4,\"shape\":[],\"data\":[6]}"
    );
}

#[test]
fn multiple_names_select_items_extend_atoms_and_open_once() {
    let mut e = Engine::new();
    e.eval("'a b'=:3 4").unwrap();
    assert_eq!(
        json(&mut e, "a+b"),
        "{\"type\":4,\"shape\":[],\"data\":[7]}"
    );
    e.eval("'a b'=:7").unwrap();
    assert_eq!(
        json(&mut e, "a+b"),
        "{\"type\":4,\"shape\":[],\"data\":[14]}"
    );
    e.eval("'a b'=:2 3$i.6").unwrap();
    assert_eq!(
        json(&mut e, "b"),
        "{\"type\":4,\"shape\":[3],\"data\":[3,4,5]}"
    );
    e.eval("'a b'=:<7 8").unwrap();
    assert_eq!(json(&mut e, "a"), json(&mut e, "b"));
    assert_eq!(
        json(&mut e, "a"),
        "{\"type\":4,\"shape\":[2],\"data\":[7,8]}"
    );
    e.eval("'a a'=:3 4").unwrap();
    assert_eq!(json(&mut e, "a"), "{\"type\":4,\"shape\":[],\"data\":[4]}");
    assert!(e.eval("''=:i.0").unwrap().is_none());
    e.eval("'a b'=:(<7),<8 9").unwrap();
    assert_eq!(
        json(&mut e, "b"),
        "{\"type\":4,\"shape\":[2],\"data\":[8,9]}"
    );
}

#[test]
fn partial_commits_and_failure_precedence_match_j_assignment_order() {
    let mut e = Engine::new();
    e.eval("a=:9").unwrap();
    assert_eq!(e.eval("'a b'=:3 4 5").unwrap_err().kind(), "length error");
    assert_eq!(json(&mut e, "a"), "{\"type\":4,\"shape\":[],\"data\":[9]}");
    let captured = e.eval_captured("'a 1bad'=:3 4");
    captured.capture.verify().unwrap();
    assert_eq!(captured.result.unwrap_err().kind(), "ill-formed name");
    assert_eq!(json(&mut e, "a"), "{\"type\":4,\"shape\":[],\"data\":[3]}");
    assert_eq!(
        captured
            .capture
            .events
            .iter()
            .filter(|x| matches!(x, CaptureEvent::Commit { .. }))
            .count(),
        1
    );
    assert_eq!(
        e.eval("'a 1bad'=:1 2+1 2 3").unwrap_err().kind(),
        "length error"
    );
    assert_eq!(e.eval("''=:7").unwrap_err().kind(), "ill-formed name");
    assert_eq!(e.eval("''=:+").unwrap_err().kind(), "ill-formed name");
    assert_eq!(e.eval("'a b'=:+").unwrap_err().kind(), "domain error");
    assert_eq!(e.eval("(2 1$'a')=:7").unwrap_err().kind(), "rank error");
}

#[test]
fn dynamic_target_uses_current_frame_and_keeps_global_scope_distinct() {
    for definition in [
        "f=:{{\nnames=.'a b'\n(names)=.y,y+1\na+b\n}}",
        "f=:3 : 0\nnames=.'a b'\n(names)=.y,y+1\na+b\n)",
    ] {
        let mut e = Engine::new();
        e.eval("a=:99").unwrap();
        e.eval(definition).unwrap();
        assert_eq!(
            json(&mut e, "f 3"),
            "{\"type\":4,\"shape\":[],\"data\":[7]}"
        );
        assert_eq!(json(&mut e, "a"), "{\"type\":4,\"shape\":[],\"data\":[99]}");
        assert!(e.binding_version("b").is_none());
    }
    let mut e = Engine::new();
    e.eval("names=.'a'").unwrap();
    e.eval("(names)=:7").unwrap();
    assert_eq!(json(&mut e, "a"), "{\"type\":4,\"shape\":[],\"data\":[7]}");
    e.eval("('b', 'c')=:3 4").unwrap();
    assert_eq!(
        json(&mut e, "bc"),
        "{\"type\":4,\"shape\":[2],\"data\":[3,4]}"
    );
    e.eval("f=:{{for_i. i.1 do. try. 'a i_index'=.3 4 catch. a return. end. end.}}")
        .unwrap();
    assert_eq!(
        json(&mut e, "f 0"),
        "{\"type\":4,\"shape\":[],\"data\":[3]}"
    );
}

#[test]
fn frontend_retains_noun_target_and_ordered_writes_are_not_one_write_graph() {
    let e = Engine::new();
    for source in ["'a'=:7", "'a b'=:3 4"] {
        let program = rustj::parser::parse_frontend(source).unwrap();
        assert!(program.has_assignment());
        assert!(program.noun_assignment.is_some());
        let context = program.frontend.as_ref().unwrap();
        context.verify().unwrap();
        assert!(
            context
                .nodes
                .iter()
                .any(|n| matches!(n.kind, NodeKind::WriteName { .. }))
        );
    }
    assert!(e.analyze_j_graph("'a'=:7").is_ok());
    assert_eq!(
        e.analyze_j_graph("'a b'=:3 4").unwrap_err().kind(),
        "unsupported"
    );
    let mut e = Engine::new();
    let captured = e.eval_captured("'a b'=:3 4");
    captured.result.unwrap();
    captured.capture.verify().unwrap();
    assert!(captured.capture.requires_ordered_effect_graph());
    let commits: Vec<_> = captured
        .capture
        .events
        .iter()
        .filter_map(|event| match event {
            CaptureEvent::Commit {
                name,
                source,
                final_assignment,
                ..
            } => Some((
                name.as_str(),
                source.selection.as_ref().unwrap().item,
                *final_assignment,
            )),
            _ => None,
        })
        .collect();
    assert_eq!(commits, vec![("a", Some(0), false), ("b", Some(1), true)]);
    let captured = e.eval_captured("('d', 'e')=:7");
    captured.result.unwrap();
    captured.capture.verify().unwrap();
    assert!(rustj::j_graph_ir::Plan::from_capture(&captured.capture).is_err());
}

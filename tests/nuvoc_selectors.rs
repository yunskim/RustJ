use rustj::{
    Engine,
    enqueuer::{EnqueueClass, enqueue},
    semantic::{ExprKind, FunctionHead},
};

#[test]
fn current_nuvoc_selector_words_have_distinct_modifier_parts_of_speech() {
    let words = enqueue("[. ]. ]:").unwrap();
    assert_eq!(
        words.iter().map(|w| w.class).collect::<Vec<_>>(),
        vec![
            EnqueueClass::Conjunction,
            EnqueueClass::Conjunction,
            EnqueueClass::Adverb
        ]
    );
    assert_eq!(
        words
            .iter()
            .map(|w| &"[. ]. ]:"[w.span.clone()])
            .collect::<Vec<_>>(),
        vec!["[.", "].", "]:"]
    );
    // @: is At; the rank-sensitive @ Atop is not an alias for it.
    assert_eq!(rustj::primitive::ConjunctionId::Atop.spelling(), "@:");
    assert_eq!(rustj::primitive::ConjunctionId::from_spelling("@"), None);
}

#[test]
fn selectors_return_actual_noun_or_verb_and_keep_late_names() {
    let mut e = Engine::new();
    for (s, n) in [
        ("2 [. 3", 2),
        ("2 ]. 3", 3),
        ("2 ]:", 2),
        ("2 [. +", 2),
        ("+ ]. 3", 3),
        ("(+ [. -) 7", 7),
        ("(+ ]. -) 7", -7),
        ("(+ ]:) 7", 7),
        ("3 (+ [. -) 7", 10),
    ] {
        assert_eq!(e.eval(s).unwrap().unwrap().int_at(0).unwrap(), n, "{s}");
    }
    e.eval("right=:-").unwrap();
    e.eval("f=:+ ]. right").unwrap();
    e.eval("right=:+").unwrap();
    assert_eq!(e.eval("f 7").unwrap().unwrap().int_at(0).unwrap(), 7);
    e.eval("right=:9").unwrap();
    assert_eq!(e.eval("f 7").unwrap_err().kind(), "domain error");
    e.eval("unused=:9").unwrap();
    e.eval("g=:unused [. +").unwrap();
    assert_eq!(e.eval("g").unwrap().unwrap().int_at(0).unwrap(), 9);
    e.eval("unused=:3").unwrap();
    assert_eq!(e.eval("g").unwrap().unwrap().int_at(0).unwrap(), 9);
}

#[test]
fn selecting_an_entity_does_not_skip_operand_reduction_effects_or_errors() {
    let mut e = Engine::new();
    e.eval("count=:0").unwrap();
    assert_eq!(
        e.eval("2 [. (count=:3)")
            .unwrap()
            .unwrap()
            .int_at(0)
            .unwrap(),
        2
    );
    assert_eq!(e.eval("count").unwrap().unwrap().int_at(0).unwrap(), 3);
    assert_eq!(
        e.eval("2 [. (1 2+1 2 3)").unwrap_err().kind(),
        "length error"
    );
    let report = e.eval_captured("f=:+ ]. -");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let bound = e.prepare_semantic("+ ]. -").unwrap();
    let ExprKind::VerbValue(v) = bound.program.expression.unwrap().kind else {
        panic!()
    };
    assert_eq!(
        v.entity.head,
        FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Subtract)
    );
}

#[test]
fn selector_modifier_trains_apply_to_the_current_operands() {
    let mut e = Engine::new();
    for (operator, expected) in [("(]: [.)", 0), ("(]: ].)", 0)] {
        e.eval(&format!("c=:{operator}")).unwrap();
        e.eval("f=:+c-").unwrap();
        assert_eq!(e.eval("f 7").unwrap().unwrap().int_at(0).unwrap(), expected);
    }
    e.eval("a=:(]: ]:)").unwrap();
    e.eval("f=:+a").unwrap();
    assert_eq!(e.eval("f 7").unwrap().unwrap().int_at(0).unwrap(), 7);
}

#[test]
fn captured_noun_selection_reuses_dependency_nodes_without_erasing_reductions() {
    use rustj::j_graph_ir::{NodeKind, Plan};
    use rustj::parser_capture::{CaptureEvent, OccurrenceId};
    let mut e = Engine::new();
    for source in [
        "2 [. 3",
        "2 ]. 3",
        "2 ]:",
        "(1+2) [. (3*4)",
        "((1+2) ]. (3*4)) ]:",
    ] {
        let report = e.eval_captured(source);
        report.result.unwrap();
        let graph = Plan::from_capture(&report.capture).unwrap();
        graph.graph.verify().unwrap();
        let output = report.capture.result.unwrap();
        let CaptureEvent::ConstructionNounSuccess {
            selected_input: Some(input),
            ..
        } = report
            .capture
            .events
            .iter()
            .rev()
            .find(|event| matches!(event, CaptureEvent::ConstructionNounSuccess { .. }))
            .unwrap()
        else {
            panic!()
        };
        let value = |id| {
            graph
                .occurrences
                .iter()
                .find(|(occurrence, _)| *occurrence == id)
                .unwrap()
                .1
        };
        assert_eq!(value(output), value(*input), "{source}");
        if source.contains("3*4") {
            assert_eq!(
                graph
                    .graph
                    .nodes
                    .iter()
                    .filter(|n| matches!(n.kind, NodeKind::Apply { .. }))
                    .count(),
                2
            );
        }
        let mut malformed = report.capture.clone();
        for event in &mut malformed.events {
            if let CaptureEvent::ConstructionNounSuccess { selected_input, .. } = event {
                *selected_input = Some(OccurrenceId(999));
                break;
            }
        }
        assert!(malformed.verify().is_err());
    }
    // Static parsing must not erase a discarded computation (including errors).
    for source in ["2 [. (3*4)", "(3*4) ]. 2", "2 [. (1 2+1 2 3)"] {
        assert_eq!(
            e.prepare_semantic(source).unwrap_err().kind(),
            "unsupported"
        );
    }
    assert!(e.prepare_semantic("(3*4) ]:").is_ok());
    let report = e.eval_captured("2 [. (count=:3)");
    report.result.unwrap();
    assert!(
        Plan::from_capture(&report.capture)
            .unwrap_err()
            .to_string()
            .contains("ordered assignment/effect graph")
    );
}

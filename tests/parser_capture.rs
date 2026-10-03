use rustj::{
    Engine,
    parser::ParseRow,
    parser_capture::{CaptureEvent, ParseCapture},
    primitive::PrimitiveId,
    semantic::FunctionHead,
};

fn output(
    result: rustj::Result<Option<rustj::Value>>,
) -> std::result::Result<Option<String>, String> {
    result
        .map(|value| value.map(|v| v.json()))
        .map_err(|error| error.kind().into())
}

#[test]
fn capture_on_off_preserves_values_errors_and_binding_versions() {
    let mut plain = Engine::new();
    let mut observed = Engine::new();
    for source in [
        "a=:1 2 3",
        "a+10",
        "a=:a*2",
        "a=:1 2+1 2 3",
        "a",
        "f=:+\"(1+0)",
        "f i.2 3",
        "'a'+1",
        "1+2*3",
    ] {
        let expected = output(plain.eval(source));
        let report = observed.eval_captured(source);
        report.capture.verify().unwrap();
        assert_eq!(output(report.result), expected, "{source}");
        for name in ["a", "f"] {
            assert_eq!(observed.binding_version(name), plain.binding_version(name));
        }
    }
}

#[test]
fn each_runtime_row_application_executes_once_and_keeps_input_edges() {
    let mut engine = Engine::new();
    let report = engine.eval_captured("2+3*4");
    let expected = engine.eval("14").unwrap().unwrap().json();
    assert_eq!(report.result.unwrap().unwrap().json(), expected);
    report.capture.verify().unwrap();
    let calls: Vec<_> = report
        .capture
        .events
        .iter()
        .filter_map(|event| match event {
            CaptureEvent::ApplyAttempt {
                id,
                function,
                left,
                right,
                ..
            } => Some((*id, function.head.clone(), *left, *right)),
            _ => None,
        })
        .collect();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[0].1,
        FunctionHead::PrimitiveVerb(PrimitiveId::Multiply)
    );
    assert_eq!(calls[1].1, FunctionHead::PrimitiveVerb(PrimitiveId::Add));
    assert_eq!(calls[1].3, calls[0].0);
    assert_eq!(report.capture.result, Some(calls[1].0));
    assert_eq!(
        report
            .capture
            .events
            .iter()
            .filter(|e| matches!(e, CaptureEvent::ApplySuccess { .. }))
            .count(),
        2
    );
}

#[test]
fn computed_rank_operand_connects_construction_to_the_producing_operation() {
    let mut engine = Engine::new();
    let report = engine.eval_captured("f=:+\"(1+0)");
    assert!(report.result.unwrap().is_none());
    report.capture.verify().unwrap();
    let producer = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::ApplySuccess { id, .. } => Some(*id),
            _ => None,
        })
        .unwrap();
    let inputs = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::ConstructionAttempt {
                row: ParseRow::Conjunction,
                noun_inputs,
                ..
            } => Some(noun_inputs),
            _ => None,
        })
        .unwrap();
    assert_eq!(inputs, &[producer]);
    assert!(
        matches!(report.capture.events.last().unwrap(), CaptureEvent::Commit { name, .. } if name == "f")
    );
    assert_eq!(
        engine.eval("f i.2 3").unwrap().unwrap().json(),
        engine.eval("+\"1 i.2 3").unwrap().unwrap().json()
    );
    // A static analysis request still has no permission to execute this operand.
    assert_eq!(
        engine.prepare_semantic("g=:+\"(1+0)").unwrap_err().kind(),
        "unsupported"
    );
    assert_eq!(engine.binding_version("g"), None);
}

#[test]
fn failure_preserves_partial_capture_and_does_not_commit_outer_assignment() {
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    let version = engine.binding_version("f");
    let report = engine.eval_captured("f=:+\"('a'+1)");
    assert_eq!(report.result.unwrap_err().kind(), "domain error");
    report.capture.verify().unwrap();
    assert!(
        report.capture.events.iter().any(
            |e| matches!(e, CaptureEvent::ApplyFailure { kind, .. } if kind == "domain error")
        )
    );
    assert!(
        !report
            .capture
            .events
            .iter()
            .any(|e| matches!(e, CaptureEvent::Commit { .. }))
    );
    assert_eq!(engine.binding_version("f"), version);
    assert_eq!(
        engine.eval("f 3").unwrap().unwrap().json(),
        engine.eval("+3").unwrap().unwrap().json()
    );
    let report = engine.eval_captured("f=:+\"'bad'");
    assert_eq!(report.result.unwrap_err().kind(), "domain error");
    report.capture.verify().unwrap();
    assert!(report.capture.events.iter().any(|e| matches!(
        e,
        CaptureEvent::ConstructionFailure {
            row: ParseRow::Conjunction,
            ..
        }
    )));
}

#[test]
fn parenthesis_preserves_occurrence_and_large_reads_capture_only_facts() {
    let mut engine = Engine::new();
    engine.eval("a=:i.100000").unwrap();
    let report = engine.eval_captured("((a))+a");
    assert_eq!(report.result.unwrap().unwrap().len(), 100000);
    report.capture.verify().unwrap();
    let reads: Vec<_> = report
        .capture
        .events
        .iter()
        .filter_map(|event| match event {
            CaptureEvent::Input {
                id,
                name: Some(name),
                version,
                facts,
                ..
            } => Some((*id, name, version, facts)),
            _ => None,
        })
        .collect();
    assert_eq!(reads.len(), 2);
    assert!(reads.iter().all(|(_, name, version, facts)| *name == "a"
        && **version == engine.binding_version("a")
        && facts.shape.as_deref() == Some(&[100000])));
    assert_ne!(reads[0].0, reads[1].0); // occurrences are not CSE/optimization
    assert_eq!(engine.eval("a").unwrap().unwrap().int_at(1).unwrap(), 1);
}

#[test]
fn computed_noun_left_fork_constructs_without_erasing_operand_origin() {
    let mut engine = Engine::new();
    let report = engine.eval_captured("f=:(1+2) + *");
    assert!(report.result.unwrap().is_none());
    report.capture.verify().unwrap();
    assert!(report.capture.events.iter().any(|event| matches!(event,
        CaptureEvent::ConstructionAttempt { row: ParseRow::Fork, noun_inputs, .. } if noun_inputs.len() == 1)));
    // Executor support for arbitrary fork calls is tracked separately.
    assert_eq!(engine.eval("f 4").unwrap_err().kind(), "unsupported");
}

#[test]
fn malformed_capture_edges_and_orphaned_outcomes_are_rejected() {
    use rustj::parser_capture::OccurrenceId;
    let mut capture = ParseCapture::default();
    capture.events.push(CaptureEvent::ApplySuccess {
        id: OccurrenceId(0),
        facts: Default::default(),
    });
    assert!(capture.verify().is_err());
}

#[test]
fn capture_adapter_preserves_applications_and_observed_facts_separately() {
    use rustj::j_graph_ir::{NodeKind, Plan};
    let mut engine = Engine::new();
    let report = engine.eval_captured("2+3*4");
    report.result.unwrap();
    let graph = Plan::from_capture(&report.capture).unwrap();
    graph.graph.verify().unwrap();
    assert_eq!(graph.occurrences.len(), 5);
    assert_eq!(graph.observed_facts.len(), 5);
    let root = graph.graph.result.unwrap();
    let NodeKind::Apply {
        right, function, ..
    } = &graph.graph.nodes[root.0].kind
    else {
        panic!()
    };
    assert_eq!(function.head, FunctionHead::PrimitiveVerb(PrimitiveId::Add));
    assert!(matches!(
        graph.graph.nodes[right.0].kind,
        NodeKind::Apply { .. }
    ));
    assert_eq!(
        graph
            .graph
            .static_memory_analysis()
            .extent(root)
            .unwrap()
            .atoms,
        1
    );
}

#[test]
fn capture_adapter_retains_computed_constructor_dependencies_and_final_entity() {
    use rustj::j_graph_ir::{NodeKind, Plan};
    let mut engine = Engine::new();
    for source in ["f=:+\"(1+0)", "f=:(1+2) + *"] {
        let report = engine.eval_captured(source);
        report.result.unwrap();
        let captured = Plan::from_capture(&report.capture).unwrap();
        captured.graph.verify().unwrap();
        let constructor = captured.constructors.last().unwrap();
        assert_eq!(constructor.noun_inputs.len(), 1);
        assert!(matches!(
            captured.graph.nodes[constructor.noun_inputs[0].0].kind,
            NodeKind::Apply { .. }
        ));
        let result = captured.graph.result.unwrap();
        let NodeKind::VerbValue { function } = &captured.graph.nodes[result.0].kind else {
            panic!()
        };
        assert!(std::sync::Arc::ptr_eq(function, &constructor.function));
        let write = captured.graph.write.unwrap();
        assert_eq!(write.name, "f");
        assert_eq!(write.value, result);
        assert_eq!(write.proposed, engine.binding_version("f").unwrap());
    }
}

#[test]
fn captured_binding_versions_are_occurrence_witnesses_not_current_workspace_reads() {
    use rustj::j_graph_ir::{GraphAnalyzability, NodeKind, Plan};
    let mut engine = Engine::new();
    engine.eval("a=:1 2 3").unwrap();
    let old = engine.binding_version("a");
    let report = engine.eval_captured("a=:a+1");
    report.result.unwrap();
    let captured = Plan::from_capture(&report.capture).unwrap();
    let read = captured
        .graph
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            NodeKind::ReadNoun { version, .. } => Some(*version),
            _ => None,
        })
        .unwrap();
    assert_eq!(Some(read), old);
    assert_eq!(captured.graph.write.as_ref().unwrap().previous, old);
    assert_ne!(Some(read), engine.binding_version("a"));
    engine.eval("f=:+").unwrap();
    let report = engine.eval_captured("f a");
    report.result.unwrap();
    let captured = Plan::from_capture(&report.capture).unwrap();
    assert!(
        captured
            .graph
            .verb_references
            .iter()
            .any(|(name, _)| name == "f")
    );
    assert_eq!(
        captured.graph.nodes[captured.graph.result.unwrap().0].analyzability,
        GraphAnalyzability::RequiresSpecialization
    );
}

#[test]
fn capture_adapter_rejects_failures_and_inconsistent_literal_facts() {
    use rustj::j_graph_ir::Plan;
    let mut engine = Engine::new();
    let failure = engine.eval_captured("'a'+1");
    assert!(failure.result.is_err());
    assert!(Plan::from_capture(&failure.capture).is_err());
    let mut report = engine.eval_captured("2+3");
    let CaptureEvent::Input { facts, .. } = &mut report.capture.events[0] else {
        panic!()
    };
    facts.shape = Some(vec![2]);
    facts.rank = Some(1);
    assert_eq!(
        Plan::from_capture(&report.capture).unwrap_err().kind(),
        "unsupported"
    );
    let report = engine.eval_captured("NB. empty");
    let graph = Plan::from_capture(&report.capture).unwrap();
    assert!(graph.graph.nodes.is_empty());
}

#[test]
fn parser_time_assignment_precedes_left_name_lookup_and_retains_versions() {
    use rustj::j_graph_ir::Plan;
    let mut engine = Engine::new();
    engine.eval("x=:0").unwrap();
    let previous = engine.binding_version("x");
    let report = engine.eval_captured("x+(x=:2)");
    assert_eq!(report.result.unwrap().unwrap().int_at(0).unwrap(), 4);
    report.capture.verify().unwrap();
    let events = &report.capture.events;
    let index = events
        .iter()
        .position(|e| matches!(e, CaptureEvent::Commit { name, .. } if name == "x"))
        .unwrap();
    let CaptureEvent::Commit {
        previous: old,
        version,
        value,
        class,
        source,
        ..
    } = &events[index]
    else {
        panic!()
    };
    assert_eq!(*old, previous);
    assert_eq!(Some(*version), engine.binding_version("x"));
    assert!(value.is_some());
    assert_eq!(*class, rustj::parser::ParseClass::Noun);
    assert!(source.flags.global_assignment);
    assert!(events[index+1..].iter().any(|e| matches!(e, CaptureEvent::Input { name: Some(name), version: observed, .. } if name == "x" && *observed == Some(*version))));
    assert!(report.capture.requires_ordered_effect_graph());
    assert_eq!(
        Plan::from_capture(&report.capture).unwrap_err().kind(),
        "unsupported"
    );
    assert_eq!(
        engine.prepare_semantic("x+(x=:2)").unwrap_err().kind(),
        "unsupported"
    );
    assert_eq!(engine.binding_version("x"), Some(*version));
}

#[test]
fn parser_time_commits_survive_later_error_and_do_not_commit_outer_target() {
    let mut plain = Engine::new();
    let mut observed = Engine::new();
    for source in [
        "x=:0",
        "out=:99",
        "out=:'a'+(x=:2)",
        "out",
        "x",
        "a=:b=:1",
        "a",
        "b",
        "x+(x=:x+1)",
        "(x=.4)",
        "x",
    ] {
        let expected = output(plain.eval(source));
        let report = observed.eval_captured(source);
        assert_eq!(output(report.result), expected, "{source}");
        report.capture.verify().unwrap();
        for name in ["x", "out", "a", "b"] {
            assert_eq!(plain.binding_version(name), observed.binding_version(name));
        }
    }
    assert_eq!(plain.eval("out").unwrap().unwrap().int_at(0).unwrap(), 99);
    assert_eq!(plain.eval("a+b").unwrap().unwrap().int_at(0).unwrap(), 2);
    assert_eq!(plain.eval("x").unwrap().unwrap().int_at(0).unwrap(), 4);
}

#[test]
fn intermediate_array_assignment_freezes_alias_without_losing_producer_identity() {
    let mut engine = Engine::new();
    engine.eval("a=:i.100000").unwrap();
    engine.eval("saved=:a").unwrap();
    let report = engine.eval_captured("a+(a=:a+1)");
    let value = report.result.unwrap().unwrap();
    assert_eq!(value.int_at(99999).unwrap(), 200000);
    report.capture.verify().unwrap();
    assert_eq!(
        engine
            .eval("saved")
            .unwrap()
            .unwrap()
            .int_at(99999)
            .unwrap(),
        99999
    );
    assert_eq!(
        engine.eval("a").unwrap().unwrap().int_at(99999).unwrap(),
        100000
    );
    let producer = report
        .capture
        .events
        .iter()
        .find_map(|e| match e {
            CaptureEvent::ApplySuccess { id, .. } => Some(*id),
            _ => None,
        })
        .unwrap();
    assert!(
        report
            .capture
            .events
            .iter()
            .any(|e| matches!(e, CaptureEvent::Commit { value: Some(id), .. } if *id == producer))
    );
}

#[test]
fn grouped_assignment_result_is_not_a_final_silent_write() {
    let mut engine = Engine::new();
    let report = engine.eval_captured("(x=:2)");
    assert_eq!(report.result.unwrap().unwrap().int_at(0).unwrap(), 2);
    report.capture.verify().unwrap();
    assert!(report.capture.requires_ordered_effect_graph());
    assert!(matches!(
        report.capture.events.last(),
        Some(CaptureEvent::Commit {
            final_assignment: false,
            ..
        })
    ));
}

#[test]
fn intermediate_function_assignment_preserves_actual_pos_and_entity() {
    use rustj::parser::ParseClass;
    let mut engine = Engine::new();
    for (source, class, expected) in [
        ("(f=:-) 3", ParseClass::Verb, -3),
        ("+(adv=:/) i.3", ParseClass::Adverb, 3),
        ("+(conj=:\")0 (3)", ParseClass::Conjunction, 3),
    ] {
        let report = engine.eval_captured(source);
        assert_eq!(
            report.result.unwrap().unwrap().int_at(0).unwrap(),
            expected,
            "{source}"
        );
        report.capture.verify().unwrap();
        let entity = report
            .capture
            .events
            .iter()
            .find_map(|e| match e {
                CaptureEvent::Commit {
                    class: actual,
                    function: Some(function),
                    value: None,
                    final_assignment: false,
                    ..
                } => {
                    assert_eq!(*actual, class);
                    Some(function)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(ParseClass::from(entity.result_pos), class);
        assert!(report.capture.requires_ordered_effect_graph());
    }
}

#[test]
fn commit_verifier_rejects_wrong_pos_version_and_final_order() {
    let mut engine = Engine::new();
    let report = engine.eval_captured("x=:2");
    report.result.unwrap();
    for mutation in 0..3 {
        let mut capture = report.capture.clone();
        let CaptureEvent::Commit { class, version, .. } = capture.events.last_mut().unwrap() else {
            panic!()
        };
        match mutation {
            0 => *class = rustj::parser::ParseClass::Verb,
            1 => *version = rustj::semantic::NameVersion(0),
            _ => capture.events.push(capture.events[0].clone()),
        }
        assert!(capture.verify().is_err());
    }
}

#[test]
fn unmatched_controls_do_not_preempt_reachable_assignment_actions() {
    let mut plain = Engine::new();
    let mut observed = Engine::new();
    for source in [
        "x=:0", "(x=:2", "x", "x=:0", "((x=:2)", "x", "x=:0", "x=:2)", "x", "x=:0", ")+(x=:2)", "x",
    ] {
        let report = observed.eval_captured(source);
        assert_eq!(
            output(report.result),
            output(plain.eval(source)),
            "{source}"
        );
        report.capture.verify().unwrap();
        if source.contains("=:2") {
            assert_eq!(plain.eval("x").unwrap().unwrap().int_at(0).unwrap(), 2);
            assert!(
                report
                    .capture
                    .events
                    .iter()
                    .any(|e| matches!(e, CaptureEvent::Commit { name, .. } if name == "x"))
            );
            assert!(rustj::j_graph_ir::Plan::from_capture(&report.capture).is_err());
        }
    }
}

#[test]
fn final_hook_assignment_is_retained_when_exit_parse_finds_unmatched_control() {
    let mut engine = Engine::new();
    engine.eval("a=:1 2 3").unwrap();
    let report = engine.eval_captured("a=:missing + )");
    assert_eq!(report.result.unwrap_err().kind(), "syntax error");
    report.capture.verify().unwrap();
    assert_eq!(
        engine.binding_version("a"),
        Some(rustj::semantic::NameVersion(2))
    );
    assert!(
        matches!(report.capture.events.last(), Some(CaptureEvent::Commit {
        class: rustj::parser::ParseClass::Verb, function: Some(entity), final_assignment: true, ..
    }) if entity.head == FunctionHead::Hook)
    );
    assert!(rustj::j_graph_ir::Plan::from_capture(&report.capture).is_err());
}

#[test]
fn terminal_syntax_and_enqueue_errors_cannot_be_adapted_as_completed_graphs() {
    let mut engine = Engine::new();
    for source in ["(1", "1 )", "'unclosed", "x=:2)"] {
        let report = engine.eval_captured(source);
        let error = report.result.unwrap_err();
        let failure = report.capture.failure.as_ref().unwrap();
        assert_eq!(failure.kind, error.kind());
        assert_eq!(
            failure.context.as_ref().and_then(|c| c.span.clone()),
            error.span().cloned()
        );
        report.capture.verify().unwrap();
        assert_eq!(
            rustj::j_graph_ir::Plan::from_capture(&report.capture)
                .unwrap_err()
                .kind(),
            "unsupported"
        );
    }
}

#[test]
fn named_modifiers_resolve_at_construction_and_keep_snapshot_dependencies() {
    let mut engine = Engine::new();
    engine.eval("adv=:/").unwrap();
    let alias = engine.eval_captured("alias=:adv");
    alias.result.unwrap();
    alias.capture.verify().unwrap();
    assert!(alias.capture.events.iter().any(|e| matches!(e, CaptureEvent::ModifierResolved { binding } if binding.name == "adv" && binding.row == rustj::parser::ParseRow::Assignment)));
    let report = engine.eval_captured("f=:+alias");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let graph = rustj::j_graph_ir::Plan::from_capture(&report.capture).unwrap();
    assert_eq!(
        graph.constructors.last().unwrap().function.span,
        3.."f=:+alias".len()
    );
    assert!(
        graph
            .modifier_bindings
            .iter()
            .all(|b| b.span == (4.."f=:+alias".len()))
    );
    assert_eq!(
        graph
            .modifier_bindings
            .iter()
            .map(|b| b.name.as_str())
            .collect::<Vec<_>>(),
        ["alias"]
    );
    assert!(
        graph
            .graph
            .verb_references
            .iter()
            .any(|(name, _)| name == "alias")
    );
    engine.eval("adv=:1").unwrap();
    assert_eq!(engine.eval("f i.3").unwrap().unwrap().int_at(0).unwrap(), 3);
    assert_eq!(
        engine
            .eval("+alias i.3")
            .unwrap()
            .unwrap()
            .int_at(0)
            .unwrap(),
        3
    );
    engine.eval("conj=:\"").unwrap();
    engine.eval("g=:+conj 0").unwrap();
    engine.eval("conj=:1").unwrap();
    assert_eq!(engine.eval("g i.3").unwrap().unwrap().len(), 3);
    engine.eval("adv=:/").unwrap();
    assert_eq!(engine.eval("3 adv").unwrap_err().kind(), "domain error");
}

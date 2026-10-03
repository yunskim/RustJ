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
    assert!(alias.capture.events.iter().any(
        |e| matches!(e, CaptureEvent::ModifierStacked { snapshot } if snapshot.name == "adv")
    ));
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
            .modifier_stack_snapshots
            .iter()
            .all(|b| b.span == (4.."f=:+alias".len()))
    );
    assert_eq!(
        graph
            .modifier_stack_snapshots
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
            .all(|(name, _)| name != "alias")
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

#[test]
fn modifier_train_capture_retains_runtime_noun_origin_and_real_pos() {
    use rustj::semantic::{FunctionOperand, FunctionPartOfSpeech};
    let mut engine = Engine::new();
    let report = engine.eval_captured("train=: (1+2) \"");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let entity = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::ConstructionSuccess { function, .. }
                if function.head == FunctionHead::ModifierTrain =>
            {
                Some(function)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(entity.result_pos, FunctionPartOfSpeech::Adverb);
    let FunctionOperand::Noun { value, .. } = &entity.operands[0] else {
        panic!();
    };
    assert_eq!(value.int_at(0).unwrap(), 3);
    let noun_origin = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::ApplySuccess { id, .. } => Some(*id),
            _ => None,
        })
        .unwrap();
    assert!(report.capture.events.iter().any(|event| matches!(event,
        CaptureEvent::ConstructionAttempt { row: ParseRow::Hook, noun_inputs, .. }
        if noun_inputs == &[noun_origin])));
    assert!(report.capture.events.iter().any(|event| matches!(event, CaptureEvent::Commit { function: Some(f), .. } if std::sync::Arc::ptr_eq(f, entity))));
    assert_eq!(
        engine.prepare_semantic("(1+2) \"").unwrap_err().kind(),
        "unsupported"
    );
}

#[test]
fn modifier_train_retains_named_array_by_value_across_reassignment() {
    use rustj::semantic::FunctionOperand;
    let mut engine = Engine::new();
    engine.eval("items=:i.65").unwrap();
    let report = engine.eval_captured("train=:items \"");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let function = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::ConstructionSuccess { function, .. }
                if function.head == FunctionHead::ModifierTrain =>
            {
                Some(function)
            }
            _ => None,
        })
        .unwrap();
    let FunctionOperand::Noun { value, .. } = &function.operands[0] else {
        panic!();
    };
    engine.eval("items=:items+1").unwrap();
    assert_eq!(value.shape(), &[65]);
    assert_eq!(value.int_at(0).unwrap(), 0);
    assert_eq!(value.int_at(64).unwrap(), 64);
    assert_eq!(engine.eval("items").unwrap().unwrap().int_at(0).unwrap(), 1);
}

#[test]
fn named_bound_modifier_application_retains_identity_version_and_current_use_span() {
    let mut engine = Engine::new();
    for source in ["bound=: \"1", "alias=:bound"] {
        engine.eval(source).unwrap();
    }
    let source = "- alias i.4";
    let version = engine.binding_version("alias").unwrap();
    let graph = engine.analyze_j_graph(source).unwrap();
    graph.verify().unwrap();
    let snapshot = &graph.modifier_snapshots[0];
    assert_eq!(snapshot.version, version);
    let report = engine.eval_captured(source);
    report.result.unwrap();
    report.capture.verify().unwrap();
    let binding = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::ModifierStacked { snapshot } if snapshot.name == "alias" => {
                Some(snapshot)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(binding.version, version);
    assert_eq!(&source[binding.span.clone()], "alias");
    assert!(std::sync::Arc::ptr_eq(
        &binding.function,
        &snapshot.function
    ));
    assert!(report.capture.events.iter().any(|event| matches!(event,
        CaptureEvent::ConstructionSuccess { function, .. }
        if function.head == FunctionHead::PrimitiveConjunction(rustj::primitive::ConjunctionId::Rank))));
}

#[test]
fn computed_left_bident_input_retains_noun_origin_without_static_execution() {
    let mut engine = Engine::new();
    engine.eval("left=: -\"").unwrap();
    let report = engine.eval_captured("fn=:(1+0) left");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let origin = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::ApplySuccess { id, .. } => Some(*id),
            _ => None,
        })
        .unwrap();
    assert!(report.capture.events.iter().any(|event| matches!(event,
        CaptureEvent::ConstructionAttempt { row: ParseRow::Adverb, noun_inputs, .. } if noun_inputs == &[origin])));
    assert!(report.capture.events.iter().any(|event| matches!(event,
        CaptureEvent::ConstructionSuccess { function, .. }
        if function.head == FunctionHead::PrimitiveConjunction(rustj::primitive::ConjunctionId::Rank))));
    assert_eq!(
        engine.prepare_semantic("(1+0) left").unwrap_err().kind(),
        "unsupported"
    );
}

#[test]
fn named_derived_conjunction_alias_keeps_identity_after_rebinding() {
    let mut engine = Engine::new();
    for source in ["conj=:@:/", "alias=:conj", "conj=:1"] {
        engine.eval(source).unwrap();
    }
    let source = "fn=: + alias -";
    let graph = engine.analyze_j_graph(source).unwrap();
    graph.verify().unwrap();
    let snapshot = &graph.modifier_snapshots[0];
    assert_eq!(snapshot.name, "alias");
    let report = engine.eval_captured(source);
    report.result.unwrap();
    report.capture.verify().unwrap();
    let binding = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::ModifierStacked { snapshot } if snapshot.name == "alias" => {
                Some(snapshot)
            }
            _ => None,
        })
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &binding.function,
        &snapshot.function
    ));
    assert_eq!(&source[binding.span.clone()], "alias");
}

#[test]
fn computed_noun_left_rank_capture_retains_both_noun_origins() {
    use rustj::semantic::FunctionOperand;
    let mut engine = Engine::new();
    let source = "constantfn=: (i.4)\"1";
    assert_eq!(
        engine.prepare_semantic(source).unwrap_err().kind(),
        "unsupported"
    );
    let report = engine.eval_captured(source);
    report.result.unwrap();
    report.capture.verify().unwrap();
    let entity = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::ConstructionSuccess {
                row: ParseRow::Conjunction,
                function,
                ..
            } => Some(function),
            _ => None,
        })
        .unwrap();
    let [
        FunctionOperand::Noun { value, span },
        FunctionOperand::Noun { value: ranks, .. },
    ] = entity.operands.as_slice()
    else {
        panic!();
    };
    assert_eq!(value.shape(), [4]);
    assert_eq!(value.int_at(3).unwrap(), 3);
    assert_eq!(ranks.int_at(0).unwrap(), 1);
    assert_eq!(&source[span.clone()], "(i.4)");
    assert!(report.capture.events.iter().any(|event| matches!(event,
        CaptureEvent::ConstructionAttempt { row: ParseRow::Conjunction, noun_inputs, .. } if noun_inputs.len() == 2)));
}

#[test]
fn noun_left_rank_with_right_verb_is_not_a_rank_of_that_verb() {
    use rustj::j_graph_ir::{GraphForm, NodeKind};
    let engine = Engine::new();
    let graph = engine.analyze_j_graph("(3\"+) i.4").unwrap();
    graph.verify().unwrap();
    let form = graph
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            NodeKind::Apply { function, form, .. }
                if matches!(
                    function.head,
                    rustj::semantic::FunctionHead::PrimitiveConjunction(
                        rustj::primitive::ConjunctionId::Rank
                    )
                ) =>
            {
                Some(form)
            }
            _ => None,
        })
        .unwrap();
    assert!(matches!(form, GraphForm::Modifier { .. }));
}

#[test]
fn gerund_named_noun_snapshot_survives_rebinding_without_copying_payload() {
    use rustj::semantic::FunctionOperand;
    let mut engine = Engine::new();
    engine.eval("snapnoun=:i.65536").unwrap();
    let original = engine.eval("snapnoun").unwrap().unwrap();
    let pointer = match original.data() {
        rustj::value::Data::Int(v) => v.as_slice().as_ptr(),
        _ => panic!(),
    };
    let version = engine.binding_version("snapnoun");
    engine
        .eval("snapar=:(<'3'),<((<'snapnoun'),(<'+'),<'-')")
        .unwrap();
    let report = engine.eval_captured("snapfn=:(,<snapar)\\");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let graph = rustj::j_graph_ir::Plan::from_capture(&report.capture).unwrap();
    assert_eq!(graph.gerund_name_reads.len(), 1);
    let read = &graph.gerund_name_reads[0];
    assert_eq!(read.name, "snapnoun");
    assert_eq!(read.version, version);
    assert_eq!(read.class, rustj::parser::ParseClass::Noun);
    assert!(read.facts.is_some());
    let outer = report
        .capture
        .events
        .iter()
        .find_map(|e| match e {
            CaptureEvent::ConstructionSuccess { function, .. }
                if function.decoded_gerund.is_some() =>
            {
                Some(function.clone())
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(outer.operands.len(), 1); // Original gerund noun remains the only source operand.
    let fork = &outer.decoded_gerund.as_ref().unwrap()[0];
    let FunctionOperand::Noun { value, .. } = &fork.operands[0] else {
        panic!()
    };
    assert_eq!(
        match value.data() {
            rustj::value::Data::Int(v) => v.as_slice().as_ptr(),
            _ => panic!(),
        },
        pointer
    );
    engine.eval("snapnoun=:9").unwrap();
    engine.eval("snapar=:0").unwrap();
    drop(original);
    drop(engine);
    assert_eq!(value.int_at(65535).unwrap(), 65535);
}

#[test]
fn gerund_capture_keeps_lookup_order_on_failure_and_discards_partial_rank_decode() {
    let mut engine = Engine::new();
    engine.eval("snapnoun=:7").unwrap();
    engine
        .eval("snapar=:(<'3'),<((<'snapnoun'),(<''),<'-')")
        .unwrap();
    engine.eval("keep=:+").unwrap();
    let version = engine.binding_version("keep");
    let report = engine.eval_captured("keep=:(,<snapar)\\");
    assert_eq!(report.result.unwrap_err().kind(), "length error");
    report.capture.verify().unwrap();
    assert_eq!(engine.binding_version("keep"), version);
    let read_index = report
        .capture
        .events
        .iter()
        .position(|e| matches!(e, CaptureEvent::GerundNameResolved { .. }))
        .unwrap();
    let failure_index = report
        .capture
        .events
        .iter()
        .position(|e| matches!(e, CaptureEvent::ConstructionFailure { .. }))
        .unwrap();
    assert!(read_index < failure_index);
    let mut invalid = report.capture.clone();
    let read = invalid.events.remove(read_index);
    invalid.events.insert(0, read);
    assert!(invalid.verify().is_err());
    let report = engine.eval_captured("rankfn=:(,<snapar)\"0");
    report.result.unwrap();
    report.capture.verify().unwrap();
    assert!(
        report
            .capture
            .events
            .iter()
            .any(|e| matches!(e, CaptureEvent::GerundNameResolved { .. }))
    );
    let function = report
        .capture
        .events
        .iter()
        .rev()
        .find_map(|e| match e {
            CaptureEvent::Commit {
                function: Some(f), ..
            } => Some(f),
            _ => None,
        })
        .unwrap();
    assert!(function.decoded_gerund.is_none());
}

#[test]
fn gerund_decoded_rank_snapshot_and_function_reference_are_distinct() {
    use rustj::semantic::FunctionOperand;
    let mut engine = Engine::new();
    engine.eval("snaprank=:0").unwrap();
    engine.eval("snapverb=:+").unwrap();
    engine
        .eval("snapar=:(<'\"'),<((<'snapverb'),<'snaprank')")
        .unwrap();
    let report = engine.eval_captured("snapfn=:(,<snapar)\\");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let reads: Vec<_> = report
        .capture
        .events
        .iter()
        .filter_map(|e| match e {
            CaptureEvent::GerundNameResolved { read, .. } => Some(read),
            _ => None,
        })
        .collect();
    assert_eq!(
        reads.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        ["snapverb", "snaprank"]
    );
    let outer = report
        .capture
        .events
        .iter()
        .find_map(|e| match e {
            CaptureEvent::ConstructionSuccess { function, .. }
                if function.decoded_gerund.is_some() =>
            {
                Some(function.clone())
            }
            _ => None,
        })
        .unwrap();
    let rank = &outer.decoded_gerund.as_ref().unwrap()[0];
    let FunctionOperand::Function(verb) = &rank.operands[0] else {
        panic!()
    };
    assert_eq!(verb.head, FunctionHead::NameRef("snapverb".into()));
    engine.eval("snapverb=:-").unwrap();
    engine.eval("snaprank=:2").unwrap();
    let FunctionOperand::Noun { value, .. } = &rank.operands[1] else {
        panic!()
    };
    assert_eq!(value.int_at(0).unwrap(), 0);
}

#[test]
fn static_gerund_analysis_keeps_abstract_noun_boundary_without_writing() {
    let mut engine = Engine::new();
    engine.eval("snapnoun=:7").unwrap();
    engine
        .eval("snapar=:(<'3'),<((<'snapnoun'),(<'+'),<'-')")
        .unwrap();
    engine.eval("snapger=:,<snapar").unwrap();
    let noun_version = engine.binding_version("snapnoun");
    assert_eq!(
        engine
            .prepare_semantic("snapfn=:snapger\\")
            .unwrap_err()
            .kind(),
        "unsupported"
    );
    assert_eq!(engine.binding_version("snapnoun"), noun_version);
    assert_eq!(engine.binding_version("snapfn"), None);
}

#[test]
fn serialized_bident_and_trident_calls_produce_actual_noun_snapshots() {
    use rustj::{parser_capture::ConstructorCallOutcome, semantic::FunctionOperand};
    let mut engine = Engine::new();
    for (inner, expected, dyad) in [
        ("(<'4'),<((<'-'),<((<'0'),<7))", -7, false),
        ("(<'4'),<((<((<'0'),<2)),(<'+'),<((<'0'),<3))", 5, true),
    ] {
        engine.eval(&format!("callar=:{inner}")).unwrap();
        engine
            .eval("outerar=:(<'3'),<((<callar),(<'+'),<'-')")
            .unwrap();
        let report = engine.eval_captured("callfn=:(,<outerar)\\");
        report.result.unwrap();
        report.capture.verify().unwrap();
        let graph = rustj::j_graph_ir::Plan::from_capture(&report.capture).unwrap();
        assert_eq!(graph.constructor_calls.len(), 1);
        let call = &graph.constructor_calls[0];
        assert_eq!(call.left.is_some(), dyad);
        assert!(matches!(call.outcome, ConstructorCallOutcome::Success(_)));
        let function = report
            .capture
            .events
            .iter()
            .find_map(|e| match e {
                CaptureEvent::ConstructionSuccess { function, .. }
                    if function.decoded_gerund.is_some() =>
                {
                    Some(function)
                }
                _ => None,
            })
            .unwrap();
        let FunctionOperand::Noun { value, .. } =
            &function.decoded_gerund.as_ref().unwrap()[0].operands[0]
        else {
            panic!()
        };
        assert_eq!(value.int_at(0).unwrap(), expected);
        let mut invalid = report.capture.clone();
        let index = invalid
            .events
            .iter()
            .position(|e| matches!(e, CaptureEvent::ConstructorApply { .. }))
            .unwrap();
        let event = invalid.events.remove(index);
        invalid.events.insert(0, event);
        assert!(invalid.verify().is_err());
    }
}

#[test]
fn constructor_call_failure_precedes_outer_audit_and_preserves_target() {
    use rustj::parser_capture::ConstructorCallOutcome;
    let mut engine = Engine::new();
    engine.eval("keep=:+").unwrap();
    let version = engine.binding_version("keep");
    for (inner, error) in [
        ("(<'4'),<((<'-'),<((<'0'),<'x'))", "domain error"),
        (
            "(<'4'),<((<((<'0'),<1 2)),(<'+'),<((<'0'),<1 2 3))",
            "length error",
        ),
    ] {
        engine.eval(&format!("callar=:{inner}")).unwrap();
        engine
            .eval("outerar=:(<'3'),<((<callar),(<'+'),<'-')")
            .unwrap();
        let report = engine.eval_captured("keep=:(,<outerar)\\");
        assert_eq!(report.result.unwrap_err().kind(), error);
        report.capture.verify().unwrap();
        assert_eq!(engine.binding_version("keep"), version);
        let calls: Vec<_> = report
            .capture
            .events
            .iter()
            .filter_map(|e| match e {
                CaptureEvent::ConstructorApply { call, .. } => Some(call),
                _ => None,
            })
            .collect();
        assert_eq!(calls.len(), 1);
        assert!(
            matches!(&calls[0].outcome, ConstructorCallOutcome::Failure { kind, .. } if kind == error)
        );
        let report = engine.eval_captured("rankfn=:(,<outerar)\"0");
        report.result.unwrap();
        report.capture.verify().unwrap();
        assert!(report.capture.events.iter().any(|e| matches!(e, CaptureEvent::ConstructorApply { call, .. } if matches!(call.outcome, ConstructorCallOutcome::Failure { .. }))));
    }
    // A successful call can still fail the final gerund Verb audit.
    engine
        .eval("callar=:(<'4'),<((<'-'),<((<'0'),<7))")
        .unwrap();
    let report = engine.eval_captured("keep=:(,<callar)\\");
    assert_eq!(report.result.unwrap_err().kind(), "domain error");
    report.capture.verify().unwrap();
    assert!(report.capture.events.iter().any(|e| matches!(e, CaptureEvent::ConstructorApply { call, .. } if matches!(call.outcome, ConstructorCallOutcome::Success(_)))));
    assert_eq!(engine.binding_version("keep"), version);
}

#[test]
fn constructor_call_shares_identity_payload_and_observes_name_order() {
    use rustj::semantic::FunctionOperand;
    let mut engine = Engine::new();
    engine.eval("callinput=:i.65536").unwrap();
    engine.eval("callverb=:+").unwrap();
    let original = engine.eval("callinput").unwrap().unwrap();
    let pointer = match original.data() {
        rustj::value::Data::Int(v) => v.as_slice().as_ptr(),
        _ => panic!(),
    };
    engine
        .eval("callar=:(<'4'),<((<'callverb'),<'callinput')")
        .unwrap();
    engine
        .eval("outerar=:(<'3'),<((<callar),(<'+'),<'-')")
        .unwrap();
    let report = engine.eval_captured("callfn=:(,<outerar)\\");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let order: Vec<_> = report
        .capture
        .events
        .iter()
        .filter_map(|e| match e {
            CaptureEvent::GerundNameResolved { read, .. } => Some(read.name.as_str()),
            CaptureEvent::ConstructorApply { .. } => Some("call"),
            _ => None,
        })
        .collect();
    assert_eq!(order, ["callinput", "callverb", "call"]); // Windows C hook AR: g before f.
    let outer = report
        .capture
        .events
        .iter()
        .find_map(|e| match e {
            CaptureEvent::ConstructionSuccess { function, .. }
                if function.decoded_gerund.is_some() =>
            {
                Some(function.clone())
            }
            _ => None,
        })
        .unwrap();
    let FunctionOperand::Noun { value, .. } =
        &outer.decoded_gerund.as_ref().unwrap()[0].operands[0]
    else {
        panic!()
    };
    assert_eq!(
        match value.data() {
            rustj::value::Data::Int(v) => v.as_slice().as_ptr(),
            _ => panic!(),
        },
        pointer
    );
    engine.eval("callinput=:9").unwrap();
    drop(engine);
    drop(original);
    assert_eq!(value.int_at(65535).unwrap(), 65535);
}

#[test]
fn nameless_modifier_train_keeps_value_after_redefinition() {
    use rustj::{primitive::AdverbId, semantic::FunctionOperand};
    let mut engine = Engine::new();
    engine.eval("adv=:/").unwrap();
    let report = engine.eval_captured("train=:adv /");
    report.result.unwrap();
    report.capture.verify().unwrap();
    assert!(report.capture.events.iter().any(
        |e| matches!(e, CaptureEvent::ModifierStacked { snapshot } if snapshot.name == "adv")
    ));
    let train = report
        .capture
        .events
        .iter()
        .find_map(|e| match e {
            CaptureEvent::ConstructionSuccess { function, .. } => Some(function.clone()),
            _ => None,
        })
        .unwrap();
    let FunctionOperand::Function(child) = &train.operands[0] else {
        panic!()
    };
    assert_eq!(child.head, FunctionHead::PrimitiveAdverb(AdverbId::Insert));
    engine.eval("adv=:1").unwrap();
    let report = engine.eval_captured("fn=:+train");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let graph = rustj::j_graph_ir::Plan::from_capture(&report.capture).unwrap();
    assert_eq!(graph.modifier_stack_snapshots[0].name, "train");
    assert!(std::sync::Arc::ptr_eq(
        &graph.modifier_stack_snapshots[0].function,
        &train
    ));
}

#[test]
fn nonnameless_adverb_reference_resolves_current_binding_and_preserves_old_train() {
    use rustj::semantic::FunctionOperand;
    let mut engine = Engine::new();
    for source in ["base=:+", "adv=:base \"", "train=:adv /"] {
        engine.eval(source).unwrap();
    }
    let report = engine.eval_captured("fn=:1 train");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let binding = report
        .capture
        .events
        .iter()
        .find_map(|e| match e {
            CaptureEvent::ModifierResolved { binding } if binding.name == "train" => Some(binding),
            _ => None,
        })
        .unwrap();
    let original = binding.function.clone();
    let FunctionOperand::Function(child) = &original.operands[0] else {
        panic!()
    };
    assert_eq!(child.head, FunctionHead::NameRef("adv".into()));
    engine.eval("adv=:/").unwrap();
    let version = engine.binding_version("adv").unwrap();
    let report = engine.eval_captured("fn=:+train");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let graph = rustj::j_graph_ir::Plan::from_capture(&report.capture).unwrap();
    assert!(
        graph
            .modifier_bindings
            .iter()
            .any(|b| b.name == "adv" && b.version == version)
    );
    assert_eq!(child.head, FunctionHead::NameRef("adv".into()));
    assert_eq!(
        engine.prepare_semantic("+train").unwrap_err().kind(),
        "unsupported"
    );
    engine.eval("keep=:+").unwrap();
    let target_version = engine.binding_version("keep");
    engine.eval("adv=:1").unwrap();
    let report = engine.eval_captured("keep=:+train");
    assert_eq!(report.result.unwrap_err().kind(), "domain error");
    report.capture.verify().unwrap();
    assert_eq!(engine.binding_version("keep"), target_version);
}

#[test]
fn nonnameless_conjunction_reference_checks_stored_pos_before_application() {
    let mut engine = Engine::new();
    for source in [
        "base=:+",
        "conj=:/ / base",
        "train=:conj /",
        "fn=:+train -",
        "conj=:@:",
    ] {
        engine.eval(source).unwrap();
    }
    let report = engine.eval_captured("fn=:+train -");
    report.result.unwrap();
    report.capture.verify().unwrap();
    assert!(report.capture.events.iter().any(|e| matches!(e, CaptureEvent::ModifierResolved { binding } if binding.name == "conj" && binding.expected == rustj::semantic::FunctionPartOfSpeech::Conjunction)));
    engine.eval("keep=:+").unwrap();
    let version = engine.binding_version("keep");
    engine.eval("conj=:1").unwrap();
    let report = engine.eval_captured("keep=:+train -");
    assert_eq!(report.result.unwrap_err().kind(), "domain error");
    report.capture.verify().unwrap();
    assert_eq!(engine.binding_version("keep"), version);
}

#[test]
fn gerund_modifier_name_read_precedes_actual_resolution() {
    let mut engine = Engine::new();
    for binding in ["adv=:/", "adv=:\\"] {
        engine.eval(binding).unwrap();
        engine
            .eval("ar=:(<((<'4'),<((<'adv'),<'/'))),<(,<'+')")
            .unwrap();
        let report = engine.eval_captured("fn=:(,<ar)\\");
        report.result.unwrap();
        report.capture.verify().unwrap();
        let read = report.capture.events.iter().position(|e| matches!(e, CaptureEvent::GerundNameResolved { read, .. } if read.name == "adv")).unwrap();
        let resolved = report.capture.events.iter().position(|e| matches!(e, CaptureEvent::ModifierResolved { binding } if binding.name == "adv")).unwrap();
        assert!(read < resolved);
        assert!(!report.capture.events.iter().any(
            |e| matches!(e, CaptureEvent::ModifierStacked { snapshot } if snapshot.name == "adv")
        ));
    }
}

#[test]
fn ar_constructor_inventory_errors_preserve_target_and_noun_execution_boundary() {
    let mut engine = rustj::Engine::new();
    engine.eval("inventoryfn=:+").unwrap();
    let version = engine.binding_version("inventoryfn");
    for (ar, expected) in [
        ("(<'4'),<((<((<'0'),<3)),<((<'0'),<3))", "syntax error"),
        ("(<'4'),<((<'/'),<((<'0'),<3))", "syntax error"),
        ("(<'4'),<((<((<'0'),<3)),(<'/'),<'/')", "syntax error"),
        (
            "(<'4'),<((<((<'0'),<3)),(<'+'),<((<'0'),<3))",
            "domain error",
        ),
    ] {
        engine.eval(&format!("inventoryar=:{ar}")).unwrap();
        let report = engine.eval_captured("inventoryfn=:(,<inventoryar)\\");
        assert_eq!(report.result.unwrap_err().kind(), expected, "{ar}");
        report.capture.verify().unwrap();
        assert_eq!(engine.binding_version("inventoryfn"), version);
        assert_eq!(
            engine
                .eval("inventoryfn 3")
                .unwrap()
                .unwrap()
                .int_at(0)
                .unwrap(),
            3
        );
    }
    // N V N executes before the final gerund Verb audit. Static parsing may
    // not fabricate that value or execute the call to turn it into a Literal.
    assert_eq!(
        engine
            .prepare_semantic("inventoryfn=:(,<inventoryar)\\")
            .unwrap_err()
            .kind(),
        "unsupported"
    );
}

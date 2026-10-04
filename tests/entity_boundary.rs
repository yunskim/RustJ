//! JE0 acceptance baseline for a future common semantic RHS carrier.
use rustj::{
    Data, Engine,
    parser::ParseClass,
    parser_capture::CaptureEvent,
    semantic::{FunctionEntity, FunctionHead, FunctionOperand, JEntity, JEntityRef},
};
use std::sync::Arc;

fn committed(report: &rustj::runtime::CapturedEvaluation) -> Arc<FunctionEntity> {
    report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::Commit {
                function: Some(function),
                ..
            } => Some(function.clone()),
            _ => None,
        })
        .expect("committed function")
}

#[test]
fn grouped_assignment_returns_each_rhs_class_and_retains_identity() {
    let mut engine = Engine::new();
    for (source, name, class) in [
        ("(jenoun=:7)", "jenoun", ParseClass::Noun),
        ("(jeverb=:+)", "jeverb", ParseClass::Verb),
        ("(jeadv=:/)", "jeadv", ParseClass::Adverb),
        ("(jeconj=:\")", "jeconj", ParseClass::Conjunction),
    ] {
        let report = engine.eval_captured(source);
        report.capture.verify().unwrap();
        let (version, entity) = report
            .capture
            .events
            .iter()
            .find_map(|event| match event {
                CaptureEvent::Commit {
                    name: actual,
                    class: actual_class,
                    version,
                    previous,
                    function,
                    source: origin,
                    final_assignment,
                    value,
                    ..
                } if actual == name => {
                    assert_eq!(*actual_class, class);
                    assert_eq!(*previous, None);
                    assert!(!final_assignment);
                    assert!(origin.flags.global_assignment);
                    assert_eq!(value.is_some(), class == ParseClass::Noun);
                    Some((*version, function))
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(engine.binding_version(name), Some(version));
        if class == ParseClass::Noun {
            assert_eq!(report.result.unwrap().unwrap().int_at(0).unwrap(), 7);
            assert!(entity.is_none());
        } else {
            // Display is a separate implementation boundary, after the commit.
            assert_eq!(report.result.as_ref().unwrap_err().kind(), "unsupported");
            let result = report
                .capture
                .events
                .iter()
                .find_map(|event| match event {
                    CaptureEvent::FunctionResult { function, .. } => Some(function),
                    _ => None,
                })
                .unwrap();
            assert!(Arc::ptr_eq(entity.as_ref().unwrap(), result));
            assert_eq!(ParseClass::from(result.result_pos), class);
        }
    }
    for (source, expected) in [
        ("jenoun", 7),
        ("jeverb 3", 3),
        ("+jeadv i.4", 6),
        ("+jeconj 0 i.4", 0),
    ] {
        assert_eq!(
            engine.eval(source).unwrap().unwrap().int_at(0).unwrap(),
            expected
        );
    }
}

#[test]
fn noun_alias_snapshots_and_verb_aliases_observe_later_pos_changes() {
    for captured in [false, true] {
        let mut engine = Engine::new();
        for source in [
            "jenoun=:7",
            "jecopy=:jenoun",
            "jenoun=:9",
            "jeverb=:+",
            "jealias=:jeverb",
            "jeverb=:-",
        ] {
            if captured {
                let report = engine.eval_captured(source);
                report.capture.verify().unwrap();
                report.result.unwrap();
            } else {
                engine.eval(source).unwrap();
            }
        }
        assert_eq!(
            engine.eval("jecopy").unwrap().unwrap().int_at(0).unwrap(),
            7
        );
        assert_eq!(
            engine
                .eval("jealias 3")
                .unwrap()
                .unwrap()
                .int_at(0)
                .unwrap(),
            -3
        );
        for binding in ["jeverb=:1", "jeverb=:/", "jeverb=:@:"] {
            engine.eval(binding).unwrap();
            let before = engine.binding_version("jealias");
            let report = engine.eval_captured("jealias 3");
            report.capture.verify().unwrap();
            assert_eq!(report.result.unwrap_err().kind(), "domain error");
            assert_eq!(engine.binding_version("jealias"), before);
        }
    }
}

#[test]
fn large_function_commit_shares_completed_dag_after_host_drops() {
    let mut expression = "+".to_owned();
    for _ in 0..48 {
        expression = format!("(+ {expression})");
    }
    let source = format!("jetree=:{expression}");
    let mut engine = Engine::new();
    let report = engine.eval_captured(&source);
    report.result.as_ref().unwrap();
    report.capture.verify().unwrap();
    let root = committed(&report);
    let constructed = report
        .capture
        .events
        .iter()
        .rev()
        .find_map(|event| match event {
            CaptureEvent::ConstructionSuccess { function, .. } => Some(function),
            _ => None,
        })
        .unwrap();
    assert!(Arc::ptr_eq(&root, constructed));
    let mut child = root.as_ref();
    let mut hooks = 0;
    while matches!(child.head, FunctionHead::Hook) {
        hooks += 1;
        let FunctionOperand::Function(next) = &child.operands[1] else {
            panic!()
        };
        child = next;
    }
    assert_eq!(hooks, 48);
    let original_child = match &root.operands[1] {
        FunctionOperand::Function(child) => child.clone(),
        _ => panic!(),
    };
    drop(engine);
    drop(report);
    let carrier = JEntity::Function(root.clone());
    let count = Arc::strong_count(&root);
    let JEntityRef::Function(borrowed) = carrier.as_ref() else {
        panic!()
    };
    assert!(std::ptr::eq(borrowed, root.as_ref()));
    assert_eq!(Arc::strong_count(&root), count);
    let JEntity::Function(retained) = carrier else {
        panic!()
    };
    assert!(Arc::ptr_eq(&root, &retained));
    let FunctionOperand::Function(child) = &retained.operands[1] else {
        panic!()
    };
    assert!(Arc::ptr_eq(child, &original_child));
    let count = Arc::strong_count(child);
    let operand = &retained.operands[1];
    let JEntityRef::Function(borrowed) = operand.as_entity_ref() else {
        panic!()
    };
    assert!(std::ptr::eq(borrowed, child.as_ref()));
    assert!(std::ptr::eq(operand.span(), &child.span));
    assert_eq!(Arc::strong_count(child), count);
}

#[test]
fn owning_noun_transport_and_borrowed_inspection_do_not_copy_payload() {
    let value = rustj::Value::ints([65536], (0..65536).collect()).unwrap();
    let Data::Int(data) = value.data() else {
        panic!()
    };
    let pointer = data.as_ptr();
    let carrier = JEntity::Noun(value);
    let JEntityRef::Noun(borrowed) = carrier.as_ref() else {
        panic!()
    };
    assert_eq!(borrowed.shape(), [65536]);
    let Data::Int(data) = borrowed.data() else {
        panic!()
    };
    assert_eq!(data.as_ptr(), pointer);
    let JEntity::Noun(returned) = carrier else {
        panic!()
    };
    let Data::Int(data) = returned.data() else {
        panic!()
    };
    assert_eq!(data.as_ptr(), pointer);
    assert_eq!(returned.int_at(65535).unwrap(), 65535);
}

#[test]
fn noun_to_function_replacement_retains_aliases_and_bounded_retirement() {
    for cache_limit in [0, 4096] {
        let mut engine = Engine::with_output_cache_limit(cache_limit);
        engine.eval("jereplaced=:i.256").unwrap();
        engine.eval("jesaved=:jereplaced").unwrap();
        for replacement in ["jereplaced=:+", "jereplaced=:/", "jereplaced=:\""] {
            let report = engine.eval_captured(replacement);
            report.result.as_ref().unwrap();
            report.capture.verify().unwrap();
            assert_eq!(
                engine
                    .eval("jesaved")
                    .unwrap()
                    .unwrap()
                    .int_at(255)
                    .unwrap(),
                255
            );
            assert_eq!(engine.output_cache_stats().0, 0);
        }
        engine.eval("jesaved=:0").unwrap();
        assert_eq!(
            engine.output_cache_stats().0,
            if cache_limit == 0 { 0 } else { 2048 }
        );
        engine.eval("jereplaced=:1+i.256").unwrap();
        assert_eq!(
            engine
                .eval("jereplaced")
                .unwrap()
                .unwrap()
                .int_at(255)
                .unwrap(),
            256
        );
        assert!(engine.output_cache_stats().0 <= cache_limit);
    }
}

#[test]
fn function_transport_retains_all_three_pos_and_definition_code() {
    use rustj::semantic::FunctionPartOfSpeech;
    let mut engine = Engine::new();
    for (source, expected) in [
        ("jetransport=:3 : 'y'", FunctionPartOfSpeech::Verb),
        ("jetransport=:1 : 'u y'", FunctionPartOfSpeech::Adverb),
        ("jetransport=:2 : 'u y'", FunctionPartOfSpeech::Conjunction),
    ] {
        let report = engine.eval_captured(source);
        report.result.as_ref().unwrap();
        report.capture.verify().unwrap();
        let root = committed(&report);
        let FunctionHead::ExplicitDefinition(code) = &root.head else {
            panic!()
        };
        let carrier = JEntity::Function(root.clone());
        let count = Arc::strong_count(&root);
        let JEntityRef::Function(borrowed) = carrier.as_ref() else {
            panic!()
        };
        assert_eq!(borrowed.result_pos, expected);
        assert_eq!(Arc::strong_count(&root), count);
        let JEntity::Function(returned) = carrier else {
            panic!()
        };
        assert!(Arc::ptr_eq(&root, &returned));
        let FunctionHead::ExplicitDefinition(retained) = &returned.head else {
            panic!()
        };
        assert!(Arc::ptr_eq(code, retained));
        let operand = FunctionOperand::Function(returned);
        let count = Arc::strong_count(&root);
        let code_count = Arc::strong_count(code);
        let JEntityRef::Function(borrowed) = operand.as_entity_ref() else {
            panic!()
        };
        assert_eq!(borrowed.result_pos, expected);
        assert!(std::ptr::eq(borrowed, root.as_ref()));
        assert!(std::ptr::eq(operand.span(), &root.span));
        assert_eq!(Arc::strong_count(&root), count);
        assert_eq!(Arc::strong_count(code), code_count);
    }
}

#[test]
fn constructor_noun_snapshot_shares_payload_across_rebinding_and_host_drop() {
    let mut engine = Engine::new();
    engine.eval("jelarge=:i.65536").unwrap();
    let original = engine.eval("jelarge").unwrap().unwrap();
    let Data::Int(data) = original.data() else {
        panic!()
    };
    let pointer = data.as_ptr();
    let report = engine.eval_captured("jefork=:jelarge + -");
    report.result.as_ref().unwrap();
    report.capture.verify().unwrap();
    let root = committed(&report);
    engine.eval("jelarge=:0").unwrap();
    drop(original);
    drop(report);
    drop(engine);
    let operand = &root.operands[0];
    let JEntityRef::Noun(value) = operand.as_entity_ref() else {
        panic!()
    };
    assert_eq!(operand.span(), &(8..15));
    assert_eq!(value.shape(), [65536]);
    assert_eq!(value.int_at(65535).unwrap(), 65535);
    let Data::Int(data) = value.data() else {
        panic!()
    };
    assert_eq!(data.as_ptr(), pointer);
}

#[test]
fn explicit_modifier_alias_assignment_retains_nameref_pos_without_body_execution() {
    use rustj::semantic::{ExprKind, FunctionPartOfSpeech};
    let mut engine = Engine::new();
    engine.eval("jecounter=:0").unwrap();
    for (definition, pos, rebind, call, expected) in [
        (
            "jesource=:1 : 'jecounter=:99'",
            FunctionPartOfSpeech::Adverb,
            "jesource=:/",
            "+jetarget i.4",
            6,
        ),
        (
            "jesource=:2 : 'jecounter=:99'",
            FunctionPartOfSpeech::Conjunction,
            "jesource=:\"",
            "+jetarget 0 (3)",
            3,
        ),
    ] {
        engine.eval("jetarget=:+").unwrap();
        engine.eval(definition).unwrap();
        let previous = engine.binding_version("jetarget").unwrap();
        let planned = engine.prepare_semantic("jetarget=:jesource").unwrap();
        let ExprKind::ModifierValue(function) = planned.program.expression.unwrap().kind else {
            panic!()
        };
        assert_eq!(function.head, FunctionHead::NameRef("jesource".into()));
        assert_eq!(function.result_pos, pos);
        assert_eq!(engine.binding_version("jetarget"), Some(previous));
        let report = engine.eval_captured("jetarget=:jesource");
        report.result.as_ref().unwrap();
        report.capture.verify().unwrap();
        let function = committed(&report);
        assert_eq!(function.head, FunctionHead::NameRef("jesource".into()));
        assert_eq!(function.result_pos, pos);
        assert_eq!(function.span, 10..18);
        assert!(function.operands.is_empty());
        assert!(!report.capture.events.iter().any(|event| matches!(
            event,
            CaptureEvent::ModifierResolved { .. } | CaptureEvent::ApplyAttempt { .. }
        )));
        assert_eq!(
            engine.binding_version("jetarget").unwrap().0,
            previous.0 + 1
        );
        engine.eval("jechain=:jetarget").unwrap();
        engine.eval(rebind).unwrap();
        assert_eq!(
            engine.eval(call).unwrap().unwrap().int_at(0).unwrap(),
            expected
        );
        engine.eval("jesource=:1").unwrap();
        assert_eq!(engine.eval(call).unwrap_err().kind(), "domain error");
        assert_eq!(
            engine
                .eval("jecounter")
                .unwrap()
                .unwrap()
                .int_at(0)
                .unwrap(),
            0
        );
    }
}

#[test]
fn operand_view_preserves_owned_noun_payload_and_independent_source_span() {
    let value = rustj::Value::ints([65536], (0..65536).collect()).unwrap();
    let Data::Int(data) = value.data() else {
        panic!()
    };
    let pointer = data.as_ptr();
    let operand = FunctionOperand::Noun {
        value,
        span: 17..29,
    };
    let JEntityRef::Noun(borrowed) = operand.as_entity_ref() else {
        panic!()
    };
    let Data::Int(data) = borrowed.data() else {
        panic!()
    };
    assert_eq!(data.as_ptr(), pointer);
    assert_eq!(borrowed.int_at(65535).unwrap(), 65535);
    assert_eq!(operand.span(), &(17..29));
    let FunctionOperand::Noun { value, span } = operand else {
        panic!()
    };
    let Data::Int(data) = value.data() else {
        panic!()
    };
    assert_eq!(data.as_ptr(), pointer);
    assert_eq!(span, 17..29);
}

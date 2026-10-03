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

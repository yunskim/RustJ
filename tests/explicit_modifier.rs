use rustj::{Engine, parser_capture::CaptureEvent, semantic::FunctionHead};

fn scalar(engine: &mut Engine, source: &str, expected: i64) {
    assert_eq!(
        engine.eval(source).unwrap().unwrap().int_at(0).unwrap(),
        expected,
        "{source}"
    );
}

#[test]
fn nonoperator_modifiers_return_actual_noun_and_function_pos() {
    let mut engine = Engine::new();
    for source in [
        "ema=:1 : '42'",
        "emc=:2 : 'm+n'",
        "emid=:1 : 'u'",
        "emreduce=:1 : 'u/'",
        "emadv=:1 : '/'",
        "emconj=:1 : '\"'",
    ] {
        engine.eval(source).unwrap();
    }
    scalar(&mut engine, "+ema", 42);
    scalar(&mut engine, "2 emc 3", 5);
    scalar(&mut engine, "+emid 7", 7);
    scalar(&mut engine, "+emreduce i.4", 6);
    scalar(&mut engine, "+(+emadv) i.4", 6);
    scalar(&mut engine, "+(+emconj)0 (7)", 7);
    scalar(&mut engine, "+ema + 1", 43);
    for source in [
        "+ema",
        "2 emc 3",
        "emresult=:+emid",
        "emresult=:+emadv",
        "emresult=:+emconj",
    ] {
        let report = engine.eval_captured(source);
        report.result.unwrap();
        report.capture.verify().unwrap();
        let mut invalid = report.capture.clone();
        if let Some(CaptureEvent::ConstructionNounSuccess { id, .. }) = invalid
            .events
            .iter_mut()
            .find(|event| matches!(event, CaptureEvent::ConstructionNounSuccess { .. }))
        {
            id.0 += 1;
            assert!(invalid.verify().is_err());
        }

        assert!(
            report
                .capture
                .events
                .iter()
                .any(|event| matches!(event, CaptureEvent::ExplicitModifierApply { .. }))
        );
        assert_eq!(
            rustj::j_graph_ir::Plan::from_capture(&report.capture)
                .unwrap_err()
                .kind(),
            "unsupported"
        );
    }
}

#[test]
fn operand_frames_preserve_aliases_and_escape_function_identity() {
    let mut engine = Engine::new();
    for source in [
        "u=:99",
        "m=:88",
        "v=:77",
        "n=:66",
        "emid=:1 : 'u'",
        "emnoun=:1 : 'u+m'",
        "emright=:2 : 'v'",
        "emreduce=:1 : 'u/'",
    ] {
        engine.eval(source).unwrap();
    }
    let versions = ["u", "m", "v", "n"].map(|name| engine.binding_version(name));
    scalar(&mut engine, "5 emnoun", 10);
    engine.eval("emescaped=:+emreduce").unwrap();
    scalar(&mut engine, "emescaped i.4", 6);
    engine.eval("emselected=: + emright -").unwrap();
    scalar(&mut engine, "emselected 7", -7);
    assert_eq!(
        versions,
        ["u", "m", "v", "n"].map(|name| engine.binding_version(name))
    );
    for (name, value) in [("u", 99), ("m", 88), ("v", 77), ("n", 66)] {
        scalar(&mut engine, name, value);
    }
    assert_eq!(
        engine.prepare_semantic("+emid").unwrap_err().kind(),
        "unsupported"
    );
    scalar(&mut engine, "u", 99);
    let report = engine.eval_captured("emidentity=:+emid");
    report.result.unwrap();
    let function = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::FunctionResult { function, .. } => Some(function),
            _ => None,
        })
        .unwrap();
    assert!(matches!(function.head, FunctionHead::PrimitiveVerb(_)));
}

#[test]
fn failed_and_unsupported_bodies_preserve_targets_and_restore_invocation_depth() {
    let mut engine = Engine::new();
    for source in [
        "emkeep=:+",
        "emfail=:1 : '1 2 + 1 2 3'",
        "emscope=:1 : 'u=:9'",
        "emop=:1 : 'u y'",
        "emmissing=:1 : 'm'",
        "emrecursive=:1 : '+ emrecursive'",
        "emgood=:1 : '7'",
    ] {
        engine.eval(source).unwrap();
    }
    let version = engine.binding_version("emkeep");
    let error = engine.eval_diagnostic("emkeep=:+emfail").unwrap_err();
    assert_eq!(error.kind(), "length error");
    assert_eq!(error.span().unwrap(), &(8..15));

    for (source, error) in [
        ("emkeep=:+emfail", "length error"),
        ("emkeep=:+emscope", "domain error"),
        ("emkeep=:+emop", "unsupported"),
        ("emkeep=:+emmissing", "unsupported"),
        ("emkeep=:+emrecursive", "limit error"),
    ] {
        let report = engine.eval_captured(source);
        assert_eq!(report.result.unwrap_err().kind(), error, "{source}");
        report.capture.verify().unwrap();
        assert_eq!(engine.binding_version("emkeep"), version);
        scalar(&mut engine, "+emgood", 7);
    }
}

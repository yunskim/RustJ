//! DefinitionCode construction uses no invocation frame or body name lookup.
use rustj::{
    Engine,
    semantic::{self, ExprKind, FunctionHead, FunctionPartOfSpeech},
};

fn code(source: &str) -> std::sync::Arc<rustj::definition_code::DefinitionCode> {
    let expression = semantic::parse(source).unwrap().expression.unwrap();
    let function = match expression.kind {
        ExprKind::VerbValue(verb) => verb.entity,
        ExprKind::ModifierValue(function) => function,
        _ => panic!(),
    };
    let FunctionHead::ExplicitDefinition(code) = &function.head else {
        panic!()
    };
    code.clone()
}

#[test]
fn code_preserves_local_words_without_snapshotting_future_names() {
    let source = "f=:3 : 0\ncopy=.future\ncopy+y\n)";
    let code = code(source);
    assert_eq!(&*code.source, source);
    assert_eq!(code.mode, 3);
    assert_eq!(code.sentences.len(), 2);
    assert!(code.sentences[0].words[1].flags.local_assignment);
    assert!(!code.sentences[0].words[1].flags.global_assignment);
    let name = &code.sentences[0].words[2];
    assert_eq!(&code.body[name.span.clone()], "future");
    assert!(name.flags.lookup_name);
    let mut engine = Engine::new();
    engine.eval(source).unwrap();
    assert!(engine.binding_version("future").is_none());
    assert!(engine.binding_version("copy").is_none());
    engine.eval("future=:99").unwrap();
    assert_eq!(&code.body[name.span.clone()], "future");
}

#[test]
fn valence_sections_and_direct_pos_follow_literal_source_names() {
    for (source, mode, pos) in [
        ("f=:{{ y+1 }}", 3, FunctionPartOfSpeech::Verb),
        ("f=:{{ x+y }}", 4, FunctionPartOfSpeech::Verb),
        ("f=:{{ u y }}", 1, FunctionPartOfSpeech::Adverb),
        ("f=:{{ u v y }}", 2, FunctionPartOfSpeech::Conjunction),
        ("f=:{{ 'x u v' }}", 3, FunctionPartOfSpeech::Verb),
    ] {
        let code = code(source);
        assert_eq!(code.mode, mode, "{source}");
        assert_eq!(code.result_pos, pos);
    }
    let split = code("f=:3 : 0\ny+1\n : \nx+y\n)");
    assert_eq!(split.monad, 0..1);
    assert_eq!(split.dyad, 1..2);
    let dyad = code("f=:4 : 'x+y'");
    assert_eq!(dyad.monad, 0..0);
    assert_eq!(dyad.dyad, 0..1);
}

#[test]
fn construction_does_not_execute_body_and_failed_code_preserves_binding() {
    let mut engine = Engine::new();
    engine.eval("counter=:0").unwrap();
    engine.eval("f=:{{ counter=:counter+y\ncounter }}").unwrap();
    assert_eq!(
        engine.eval("counter").unwrap().unwrap().int_at(0).unwrap(),
        0
    );
    let version = engine.binding_version("f");
    for bad in ["f=:{{ y+1", "f=:3 : 0\nif. y do.\n1\n)"] {
        assert!(engine.eval(bad).is_err());
        assert_eq!(engine.binding_version("f"), version);
    }
    // A completed function is represented, but its executor is a later step.
    assert_eq!(engine.eval("f 1").unwrap_err().kind(), "unsupported");
}

#[test]
fn row_four_constructs_after_right_hand_runtime_errors() {
    use rustj::parser::ParseRow;
    use rustj::parser_capture::CaptureEvent;
    let parsed = semantic::parse("f=:{{y+1}}").unwrap();
    assert!(
        parsed
            .reductions
            .iter()
            .any(|row| row.row == ParseRow::Conjunction)
    );
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    let version = engine.binding_version("f");
    let report = engine.eval_captured("f=:3 : 'if. y do.' 1 2+1 2 3");
    assert_eq!(report.result.unwrap_err().kind(), "length error");
    report.capture.verify().unwrap();
    assert!(
        !report
            .capture
            .events
            .iter()
            .any(|event| matches!(event, CaptureEvent::ConstructionSuccess { .. }))
    );
    assert_eq!(engine.binding_version("f"), version);
}

#[test]
fn operator_valences_and_representation_retain_source_separately() {
    let f = code("f=:{{u x+y}}");
    assert!(f.monad.is_empty());
    assert_eq!(f.dyad, 0..1);
    assert!(f.operator_definition);
    assert_eq!(f.representation_lines(), vec![":", "u x+y"]);
    assert_eq!(&*f.body, "u x+y");
    let c = code("f=:2 : 'u v'");
    assert!(!c.operator_definition);
    assert!(c.monad.is_empty());
    assert_eq!(c.representation_lines(), vec!["u v"]);
    let d = code("f=:4 : 'y+1\n:\nx+y'");
    assert_eq!(d.representation_lines(), vec!["x+y"]);
    assert_eq!(&*d.body, "y+1\n:\nx+y");
    assert_eq!(code("f=:{{}}").representation_lines(), vec![""]);
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    let version = engine.binding_version("f");
    for bad in ["f=:1 : 'u\n:\nv'", "f=:{{u\n:\nu}}"] {
        assert_eq!(engine.eval(bad).unwrap_err().kind(), "valence error");
        assert_eq!(engine.binding_version("f"), version);
    }
}

#[test]
fn captured_generated_literals_use_enqueue_indices_not_ambiguous_macro_spans() {
    use rustj::parser_capture::CaptureEvent;
    let mut engine = Engine::new();
    let mut report = engine.eval_captured("f=:{{y+1}}");
    assert!(report.result.is_ok());
    report.capture.verify().unwrap();
    rustj::j_graph_ir::Plan::from_capture(&report.capture).unwrap();
    let index = report
        .capture
        .events
        .iter()
        .position(|event| matches!(event, CaptureEvent::Input { .. }))
        .unwrap();
    if let CaptureEvent::Input { word_index, .. } = &mut report.capture.events[index] {
        *word_index = 999;
    }
    assert!(rustj::j_graph_ir::Plan::from_capture(&report.capture).is_err());
}

#[test]
fn multiple_root_codes_survive_train_construction_without_body_execution() {
    use rustj::semantic::FunctionOperand;
    let source = "combined=:{{counter=:99+y}} + {{future+y}}";
    let program = semantic::parse(source).unwrap();
    let ExprKind::VerbValue(verb) = program.expression.unwrap().kind else {
        panic!()
    };
    assert_eq!(verb.entity.head, FunctionHead::Fork);
    let codes: Vec<_> = verb
        .entity
        .operands
        .iter()
        .filter_map(|operand| match operand {
            FunctionOperand::Function(f) => match &f.head {
                FunctionHead::ExplicitDefinition(code) => Some(code),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(codes.len(), 2);
    assert!(codes[0].source_span.end < codes[1].source_span.start);
    assert_eq!(&*codes[1].body, "future+y");
    let mut engine = Engine::new();
    engine.eval("counter=:0").unwrap();
    engine.eval(source).unwrap();
    assert_eq!(
        engine.eval("counter").unwrap().unwrap().int_at(0).unwrap(),
        0
    );
    assert!(engine.binding_version("future").is_none());
    let version = engine.binding_version("combined");
    assert_eq!(
        engine.eval("combined=:{{if.}} + {{y}}").unwrap_err().kind(),
        "control error"
    );
    assert_eq!(engine.binding_version("combined"), version);
    let static_engine = Engine::new();
    static_engine.prepare_semantic(source).unwrap();
    assert!(static_engine.binding_version("combined").is_none());
}

#[test]
fn definition_result_transport_preserves_all_pos_without_body_lookup_or_execution() {
    use rustj::{parser::ParseClass, parser_capture::CaptureEvent};
    let mut engine = Engine::new();
    engine.eval("transportcounter=:0").unwrap();
    let before = engine.binding_version("transportcounter");
    for (source, pos) in [
        (
            "transportdef=:3 : 'transportcounter=:99+y'",
            FunctionPartOfSpeech::Verb,
        ),
        (
            "transportdef=:1 : 'transportcounter=:99+u y'",
            FunctionPartOfSpeech::Adverb,
        ),
        (
            "transportdef=:2 : 'transportcounter=:99+u v y'",
            FunctionPartOfSpeech::Conjunction,
        ),
        (
            "transportdef=:{{transportcounter=:99+y}}",
            FunctionPartOfSpeech::Verb,
        ),
        (
            "transportdef=:{{transportcounter=:99+u y}}",
            FunctionPartOfSpeech::Adverb,
        ),
        (
            "transportdef=:{{transportcounter=:99+u v y}}",
            FunctionPartOfSpeech::Conjunction,
        ),
    ] {
        let report = engine.eval_captured(source);
        report.result.unwrap();
        report.capture.verify().unwrap();
        let entity = report
            .capture
            .events
            .iter()
            .find_map(|e| match e {
                CaptureEvent::Commit {
                    name,
                    class,
                    function: Some(f),
                    ..
                } if name == "transportdef" => {
                    assert_eq!(*class, ParseClass::from(pos));
                    Some(f)
                }
                _ => None,
            })
            .unwrap();
        let FunctionHead::ExplicitDefinition(code) = &entity.head else {
            panic!()
        };
        assert_eq!(code.result_pos, pos);
        assert_eq!(entity.result_pos, pos);
        assert!(entity.operands.is_empty());
        let (result_index, result) = report
            .capture
            .events
            .iter()
            .enumerate()
            .find_map(|(i, e)| match e {
                CaptureEvent::FunctionResult { function, .. } => Some((i, function)),
                _ => None,
            })
            .unwrap();
        let commit_index = report
            .capture
            .events
            .iter()
            .position(|e| {
                matches!(e,
            CaptureEvent::Commit { name, .. } if name == "transportdef")
            })
            .unwrap();
        assert!(result_index < commit_index);
        assert!(std::sync::Arc::ptr_eq(entity, result));
        let constructed = report
            .capture
            .events
            .iter()
            .find_map(|e| match e {
                CaptureEvent::ConstructionSuccess { function, .. } => Some(function),
                _ => None,
            })
            .unwrap();
        assert!(std::sync::Arc::ptr_eq(entity, constructed));
        let FunctionHead::ExplicitDefinition(result_code) = &result.head else {
            panic!()
        };
        assert!(std::sync::Arc::ptr_eq(code, result_code));
        assert_eq!(engine.binding_version("transportcounter"), before);
        assert_eq!(
            engine
                .eval("transportcounter")
                .unwrap()
                .unwrap()
                .int_at(0)
                .unwrap(),
            0
        );
        assert!(!report.capture.events.iter().any(|e| matches!(e,
            CaptureEvent::Input { name: Some(name), .. } if name == "transportcounter")));
    }
}

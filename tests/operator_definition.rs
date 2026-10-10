use rustj::{
    Data, Engine,
    parser_capture::CaptureEvent,
    semantic::{FunctionHead, FunctionPartOfSpeech},
};

fn scalar(engine: &mut Engine, source: &str, expected: i64) {
    assert_eq!(
        engine.eval(source).unwrap().unwrap().int_at(0).unwrap(),
        expected,
        "{source}"
    );
}

#[test]
fn operator_construction_defers_effects_and_keeps_code_and_operand_graphs() {
    let mut engine = Engine::new();
    engine.eval("ocount=:0").unwrap();
    let definition = engine.eval_captured("op=:1 : 0\nocount=:ocount+1\nu y\n)");
    definition.result.unwrap();
    let code = definition
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::FunctionResult { function, .. } => match &function.head {
                FunctionHead::ExplicitDefinition(code) => Some(code.clone()),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let report = engine.eval_captured("derived=:-op");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let function = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::FunctionResult { function, .. } => Some(function),
            _ => None,
        })
        .unwrap();
    assert_eq!(function.result_pos, FunctionPartOfSpeech::Verb);
    let FunctionHead::ExplicitDefinition(derived_code) = &function.head else {
        panic!()
    };
    assert!(std::sync::Arc::ptr_eq(&code, derived_code));
    assert_eq!(function.operands.len(), 1);
    scalar(&mut engine, "ocount", 0);
    scalar(&mut engine, "derived 7", -7);
    scalar(&mut engine, "derived 8", -8);
    scalar(&mut engine, "ocount", 2);
    engine.eval("orec=:1 : 'orecurse y'").unwrap();
    engine.eval("orecurse=:+orec").unwrap();
    assert_eq!(engine.eval("orecurse 7").unwrap_err().kind(), "limit error");
    scalar(&mut engine, "derived 9", -9);
    scalar(&mut engine, "ocount", 3);
    engine.eval("op=:1 : 'm+y'").unwrap();
    scalar(&mut engine, "derived 9", -9);
    scalar(&mut engine, "ocount", 4);
    assert_eq!(
        engine.prepare_semantic("-op").unwrap_err().kind(),
        "unsupported"
    );
}

#[test]
fn verb_valence_is_independent_of_modifier_arity_and_name_operands_stay_late() {
    let mut engine = Engine::new();
    for source in [
        "oadv=:1 : 'u y'",
        "oconj=:2 : 'u y+v y'",
        "odyad=:1 : 'x u y'",
        "ofunc=:+",
        "derived=:ofunc oadv",
        "combined=:+oconj -",
        "dyadic=:+odyad",
    ] {
        engine.eval(source).unwrap();
    }
    scalar(&mut engine, "derived 7", 7);
    scalar(&mut engine, "combined 7", 0);
    scalar(&mut engine, "3 dyadic 7", 10);
    assert_eq!(engine.eval("dyadic 7").unwrap_err().kind(), "valence error");
    assert_eq!(
        engine.eval("3 derived 7").unwrap_err().kind(),
        "valence error"
    );
    engine.eval("ofunc=:-").unwrap();
    scalar(&mut engine, "derived 7", -7);
    engine.eval("ofunc=:0").unwrap();
    assert_eq!(engine.eval("derived 7").unwrap_err().kind(), "domain error");
    let diagnostic = engine.eval_diagnostic("derived 7").unwrap_err();
    assert_eq!(
        diagnostic.context().unwrap().current_name.as_deref(),
        Some("ofunc")
    );
    engine.eval("both=:1 : 0\nu y\n:\nx u y\n)").unwrap();
    engine.eval("twovalence=:+both").unwrap();
    scalar(&mut engine, "twovalence 7", 7);
    scalar(&mut engine, "3 twovalence 7", 10);
}

#[test]
fn calls_require_noun_results_restore_frames_and_do_not_copy_captured_arrays() {
    let mut engine = Engine::new();
    for source in [
        "x=:99",
        "y=:88",
        "okeep=:+",
        "ocount=:0",
        "ofail=:1 : 0\nocount=:ocount+1\notmp=.y\n1 2+1 2 3\n)",
        "bad=:+ofail",
        "ononnoun=:1 : 'u'",
        "ofn=:1 : 'u y'",
        "good=:+ofn",
        "oconstant=:1 : 0\ny\nm\n)",
        "obig=:i.65536",
        "oconst=:obig oconstant",
    ] {
        engine.eval(source).unwrap();
    }
    let versions = ["x", "y", "okeep"].map(|name| engine.binding_version(name));
    for _ in 0..3 {
        let report = engine.eval_captured("okeep=:bad 7");
        assert_eq!(report.result.unwrap_err().kind(), "length error");
        report.capture.verify().unwrap();
        scalar(&mut engine, "good 7", 7);
    }
    scalar(&mut engine, "ocount", 3);
    assert_eq!(
        versions,
        ["x", "y", "okeep"].map(|name| engine.binding_version(name))
    );
    assert_eq!(engine.binding_version("otmp"), None);
    // A lexical y in a later statement makes the definition an operator.
    engine.eval("ononnoun=:1 : 0\ny\nu\n)").unwrap();
    assert_eq!(
        engine.eval("(+ononnoun)7").unwrap_err().kind(),
        "noun result was required"
    );
    let original = engine.eval("obig").unwrap().unwrap();
    engine.eval("obig=:0").unwrap();
    let returned = engine.eval("oconst 1").unwrap().unwrap();
    let (Data::Int(before), Data::Int(after)) = (original.data(), returned.data()) else {
        panic!()
    };
    assert_eq!(before.as_ptr(), after.as_ptr());
    assert_eq!(returned.int_at(65535).unwrap(), 65535);
    engine.eval("onoun=:1 : 'm'").unwrap();
    let capture = engine.eval_captured("copy=: (i.4) onoun");
    capture.result.unwrap();
    let function = capture.capture.events.iter().find_map(|event| match event {
        CaptureEvent::FunctionResult { function, .. } => Some(function),
        _ => None,
    });
    // m-only definitions execute at modifier application, never acquire y.
    assert!(function.is_none());
}

use rustj::{
    Engine,
    error::DiagnosticFrameKind,
    semantic::{ExprKind, FunctionHead},
};

#[test]
fn admission_failures_keep_definition_source_without_running_body_effects() {
    let mut engine = Engine::new();
    engine.eval("count=:0").unwrap();
    let definition = "f=:3 : 'count=:count+1\nselect. y case. 1 do. 10 end.'";
    engine.eval(definition).unwrap();
    let error = engine.eval_diagnostic("f 1").unwrap_err();
    assert_eq!(error.kind(), "unsupported");
    let frames = &error.context().unwrap().source_frames;
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].source.as_ref(), definition);
    assert_eq!(frames[0].kind, DiagnosticFrameKind::DefinitionAdmission);
    assert!(frames[0].source[frames[0].span.clone()].starts_with("select."));
    assert!(frames[0].blame_word_index.is_none());
    assert_eq!(engine.eval("count").unwrap().unwrap().int_at(0).unwrap(), 0);
    assert_eq!(error.span(), Some(&(0..1)));
}

#[test]
fn final_noun_check_and_implicit_return_fix_keep_result_source() {
    for (definition, call, kind) in [
        ("f=:3 : '+'", "f 0", "noun result was required"),
        ("f=:{{+}}", "f 0", "noun result was required"),
        ("f=:1 : 'v.'", "+ f", "unsupported"),
        ("f=:1 : 'u.'", "1 f", "domain error"),
    ] {
        let mut engine = Engine::new();
        engine.eval(definition).unwrap();
        let error = engine.eval_diagnostic(call).unwrap_err();
        assert_eq!(error.kind(), kind, "{definition}");
        let frames = &error.context().unwrap().source_frames;
        assert_eq!(frames.len(), 1, "{definition}: {error:?}");
        assert_eq!(frames[0].source.as_ref(), definition);
        assert_eq!(frames[0].kind, DiagnosticFrameKind::DefinitionReturn);
        assert!(
            frames[0].blame_word_index.is_none(),
            "return boundary has no fabricated queue blame"
        );
        engine.eval("after=:7").unwrap();
        assert_eq!(engine.eval("after").unwrap().unwrap().int_at(0).unwrap(), 7);
    }
}

#[test]
fn missing_valence_and_nested_boundary_failures_keep_call_chains() {
    for (inner, expected_kind, frame_kind) in [
        (
            "inner=:4 : 'x+y'",
            "valence error",
            DiagnosticFrameKind::DefinitionAdmission,
        ),
        (
            "inner=:3 : '+'",
            "noun result was required",
            DiagnosticFrameKind::DefinitionReturn,
        ),
    ] {
        let mut engine = Engine::new();
        let outer = "outer=:{{ t=.y\ninner t }}";
        engine.eval(inner).unwrap();
        engine.eval(outer).unwrap();
        for _ in 0..3 {
            let observed = engine.eval_captured("outer 0");
            observed.capture.verify().unwrap();
            let error = observed.result.unwrap_err();
            assert_eq!(error.kind(), expected_kind);
            let frames = &error.context().unwrap().source_frames;
            assert_eq!(frames.len(), 2);
            assert_eq!(frames[0].source.as_ref(), inner);
            assert_eq!(frames[0].kind, frame_kind);
            assert_eq!(frames[1].source.as_ref(), outer);
            assert_eq!(frames[1].kind, DiagnosticFrameKind::DefinitionCall);
            assert_eq!(&frames[1].source[frames[1].span.clone()], "inner");
            assert!(engine.binding_version("t").is_none());
        }
        engine.eval("inner=:{{y+1}}").unwrap();
        assert_eq!(
            engine.eval("outer 4").unwrap().unwrap().int_at(0).unwrap(),
            5
        );
    }
}

#[test]
fn post_execution_noun_failure_is_outside_body_catch_and_preserves_prior_effects() {
    let mut engine = Engine::new();
    engine.eval("count=:0").unwrap();
    engine.eval("saved=:99").unwrap();
    let definition = "f=:3 : 'count=:count+1\ntry. local=.+ catch. 42 end.'";
    engine.eval(definition).unwrap();
    let error = engine.eval_diagnostic("saved=:f 0").unwrap_err();
    assert_eq!(error.kind(), "noun result was required");
    let frame = &error.context().unwrap().source_frames[0];
    assert_eq!(frame.kind, DiagnosticFrameKind::DefinitionReturn);
    assert_eq!(frame.source[frame.span.clone()].trim(), "local=.+");
    assert_eq!(engine.eval("count").unwrap().unwrap().int_at(0).unwrap(), 1);
    assert_eq!(
        engine.eval("saved").unwrap().unwrap().int_at(0).unwrap(),
        99
    );
    assert!(engine.binding_version("local").is_none());
    assert!(
        error
            .render("caller", "saved=:f 0", 1)
            .contains("returning from definition")
    );
}

#[test]
fn return_site_maps_escaped_source_and_survives_redefinition() {
    for definition in ["f=:3 : 'NB. ''한글''\r\n+'", "f=:{{\r\n+\r\n}}"] {
        let mut engine = Engine::new();
        engine.eval(definition).unwrap();
        let error = engine.eval_diagnostic("f 0").unwrap_err();
        let frame = &error.context().unwrap().source_frames[0];
        assert_eq!(frame.kind, DiagnosticFrameKind::DefinitionReturn);
        assert_eq!(&frame.source[frame.span.clone()], "+");
        engine.eval("f=:{{y+1}}").unwrap();
        assert_eq!(frame.source.as_ref(), definition);
    }
}

#[test]
fn definition_failure_keeps_original_source_and_external_caller_coordinates() {
    for definition in ["f=:{{\nt=.y\nt+1 2 3\n}}", "f=:3 : 0\nt=.y\nt+1 2 3\n)"] {
        let mut e = Engine::new();
        e.eval(definition).unwrap();
        let error = e.eval_diagnostic("f 1 2").unwrap_err();
        assert_eq!(error.kind(), "length error");
        assert_eq!(error.span(), Some(&(0..1)));
        let frames = &error.context().unwrap().source_frames;
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].source.as_ref(), definition);
        assert_eq!(&frames[0].source[frames[0].span.clone()], "+");
        assert_eq!(frames[0].kind, DiagnosticFrameKind::DefinitionBody);
        let rendered = error.render("caller", "f 1 2", 10);
        assert!(rendered.contains("definition failure, line 3"));
        assert!(rendered.contains("line 10"));
        assert!(rendered.contains("t+1 2 3"));
        assert!(rendered.contains("f 1 2"));
        assert!(e.binding_version("t").is_none());
    }
}

#[test]
fn nested_failures_keep_body_then_callsite_frames_in_order() {
    let mut e = Engine::new();
    let inner = "inner=:{{y+1 2 3}}";
    let outer = "outer=:{{\ninner y\n}}";
    e.eval(inner).unwrap();
    e.eval(outer).unwrap();
    let observed = e.eval_captured("outer 1 2");
    observed.capture.verify().unwrap();
    let error = observed.result.unwrap_err();
    let frames = &error.context().unwrap().source_frames;
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].source.as_ref(), inner);
    assert_eq!(&frames[0].source[frames[0].span.clone()], "+");
    assert_eq!(frames[1].source.as_ref(), outer);
    assert_eq!(&frames[1].source[frames[1].span.clone()], "inner");
    assert_eq!(frames[1].kind, DiagnosticFrameKind::DefinitionCall);
    assert_eq!(error.span(), Some(&(0..5)));
}

#[test]
fn sparse_source_map_handles_escaped_quotes_utf8_crlf_and_end_boundaries() {
    for source in [
        "f=:3 : 't=.''it''''s 한글''\ny+1 2 3'",
        "f=:{{\r\nt=.'한글'\r\ny+1 2 3\r\n}}",
    ] {
        let program = rustj::semantic::parse(source).unwrap();
        let ExprKind::VerbValue(verb) = program.expression.unwrap().kind else {
            panic!("verb")
        };
        let FunctionHead::ExplicitDefinition(code) = &verb.entity.head else {
            panic!("code")
        };
        code.verify().unwrap();
        let mut invalid = (**code).clone();
        invalid.body = std::sync::Arc::from("forged body");
        assert!(invalid.verify().is_err());
        let at = code.body.rfind('+').unwrap();
        let mapped = code.source_map.original_span(at..at + 1).unwrap();
        assert_eq!(&source[mapped], "+");
        assert!(
            code.source_map
                .original_span(code.body.len()..code.body.len())
                .is_some()
        );
        assert!(
            code.source_map
                .original_span(0..code.body.len() + 1)
                .is_none()
        );
        let mut e = Engine::new();
        e.eval(source).unwrap();
        let error = e.eval_diagnostic("f 1 2").unwrap_err();
        let frame = &error.context().unwrap().source_frames[0];
        assert_eq!(&frame.source[frame.span.clone()], "+");
    }
}

#[test]
fn modifier_errors_and_failure_cleanup_keep_semantics_and_source_alive() {
    let mut e = Engine::new();
    let definition = "adv=:1 : 'u+1 2 3'";
    e.eval(definition).unwrap();
    let error = e.eval_diagnostic("1 2 adv").unwrap_err();
    assert_eq!(error.kind(), "length error");
    assert_eq!(
        error.span(),
        Some(&(0..7)),
        "modifier construction extent belongs to the caller"
    );
    let frame = &error.context().unwrap().source_frames[0];
    assert_eq!(frame.source.as_ref(), definition);
    assert_eq!(&frame.source[frame.span.clone()], "+");
    e.eval("adv=:/").unwrap();
    assert_eq!(frame.source.as_ref(), definition);
    e.eval("a=:7").unwrap();
    assert_eq!(e.eval("a=:1 2+1 2 3").unwrap_err().kind(), "length error");
    assert_eq!(e.eval("a").unwrap().unwrap().int_at(0).unwrap(), 7);
    e.eval("f=:{{try. y+1 2 3 catch. 42 end.}}").unwrap();
    assert_eq!(e.eval("f 1 2").unwrap().unwrap().int_at(0).unwrap(), 42);
    assert_eq!(error.into_unlocated().kind(), "length error");
}

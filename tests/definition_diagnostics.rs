use rustj::{
    Engine,
    error::DiagnosticFrameKind,
    semantic::{ExprKind, FunctionHead},
};

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

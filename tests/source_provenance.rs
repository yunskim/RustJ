use rustj::{
    Engine,
    error::{DiagnosticFrameKind, SourceLocation},
    source::SourceUnit,
};

#[test]
fn nested_definitions_map_to_root_and_survive_redefinition() {
    for semantic in [false, true] {
        for definition in [
            "outer=:3 : 'inner=:3 : ''tag=.''''한'''' [ missing+y''\ninner y'",
            "outer=:{{ inner=:{{ tag=.'한'\nmissing+y }}\ninner y }}",
        ] {
            let text = format!("NB. header 한\r\n\r\n{definition}\r\n");
            let unit = SourceUnit::new("definitions.ijs", text.as_str());
            let origin = unit
                .origin()
                .slice(text.find("outer").unwrap()..text.len() - 2)
                .unwrap();
            let mut engine = Engine::new();
            engine.eval_source_diagnostic(origin, semantic).unwrap();
            let error = engine.eval_diagnostic("outer 2").unwrap_err();
            assert_eq!(error.kind(), "value error", "{error:?}");
            let frames = &error.context().unwrap().source_frames;
            assert_eq!(frames.len(), 2, "{error:?}");
            assert_eq!(frames[0].kind, DiagnosticFrameKind::DefinitionBody);
            assert_eq!(frames[1].kind, DiagnosticFrameKind::DefinitionCall);
            for frame in frames {
                assert_eq!(frame.origin.unit().id(), unit.id());
                assert!(
                    frame
                        .origin
                        .root_span(frame.definition_span.clone())
                        .is_some()
                );
            }
            let span = frames[0].origin.root_span(frames[0].span.clone()).unwrap();
            assert_eq!(
                span,
                text.find("missing").unwrap()..text.find("missing").unwrap() + 9
            );
            assert!(
                error
                    .render("caller.ijs", "outer 2", 9)
                    .contains("definitions.ijs, line")
            );
            // The globally assigned nested function escapes its creating call.
            engine.eval("outer=:{{y}}").unwrap();
            drop(unit);
            let again = engine.eval_diagnostic("inner 4").unwrap_err();
            let frame = &again.context().unwrap().source_frames[0];
            let span = frame.origin.root_span(frame.span.clone()).unwrap();
            assert_eq!(&frame.origin.unit().text()[span], "missing+y");
            assert_eq!(frame.origin.unit().name(), "definitions.ijs");
        }
    }
}

#[test]
fn return_and_admission_use_root_coordinates_without_fabricated_blame() {
    for (definition, expected) in [
        ("f=:{{+}}", DiagnosticFrameKind::DefinitionReturn),
        (
            "f=:3 : 'select. y case. 1 do. 2 end.'",
            DiagnosticFrameKind::DefinitionAdmission,
        ),
    ] {
        let text = format!("NB. file header\r\n{definition}");
        let unit = SourceUnit::new("boundaries.ijs", text.as_str());
        let mut engine = Engine::new();
        engine
            .eval_source_diagnostic(unit.origin().slice(17..text.len()).unwrap(), false)
            .unwrap();
        let error = engine.eval_diagnostic("f 0").unwrap_err();
        let frame = &error.context().unwrap().source_frames[0];
        assert_eq!(frame.kind, expected);
        assert_eq!(frame.origin.unit().id(), unit.id());
        assert!(frame.blame_word_index.is_none());
        let root_span = frame.origin.root_span(frame.span.clone()).unwrap();
        assert_eq!(SourceLocation::from_span(&text, root_span).line, 2);
        assert_eq!(error.span(), Some(&(0..1)));
    }
}

#[test]
fn input_revision_identity_is_separate_from_semantic_function_equality() {
    let first = SourceUnit::new("same.ijs", "f=:{{y+1}}");
    let second = SourceUnit::new("same.ijs", "f=:{{y+1}}");
    assert_ne!(first.id(), second.id());
    let mut engine = Engine::new();
    let a = engine.eval_source_captured(first.origin());
    let b = engine.eval_source_captured(second.origin());
    a.result.unwrap();
    b.result.unwrap();
    let a = a.capture.frontend.unwrap();
    let b = b.capture.frontend.unwrap();
    a.verify().unwrap();
    b.verify().unwrap();
    assert_eq!(a.source_origin.as_ref().unwrap().unit().id(), first.id());
    assert_eq!(b.source_origin.as_ref().unwrap().unit().id(), second.id());
    // Source identities are diagnostic metadata, never J binding versions.
    let entity = |trace: &rustj::frontend_context::FrontendContext| {
        trace
            .nodes
            .iter()
            .find_map(|node| match &node.kind {
                rustj::frontend_context::NodeKind::Function(function)
                | rustj::frontend_context::NodeKind::Construct {
                    function: Some(function),
                    ..
                } if matches!(
                    function.head,
                    rustj::semantic::FunctionHead::ExplicitDefinition(_)
                ) =>
                {
                    Some(function.clone())
                }
                _ => None,
            })
            .expect("constructed definition")
    };
    assert_eq!(entity(&a).head, entity(&b).head);
    let mut forged = (*b).clone();
    forged.source_origin = Some(SourceUnit::new("forged.ijs", "different text").origin());
    assert!(forged.verify().is_err());
}

#[test]
fn source_slices_reject_invalid_utf8_ranges_and_keep_exact_offsets() {
    let unit = SourceUnit::new("unicode.ijs", "한\r\nabc");
    let origin = unit.origin();
    assert!(origin.slice(1..3).is_err());
    assert!(origin.slice(std::ops::Range { start: 7, end: 6 }).is_err());
    assert!(origin.slice(0..99).is_err());
    let fragment = origin.slice(5..8).unwrap().slice(1..3).unwrap();
    assert_eq!(fragment.text(), "bc");
    assert_eq!(fragment.root_span(0..2), Some(6..8));
    assert_eq!(fragment.root_span(2..2), Some(8..8));
    assert_eq!(fragment.root_span(0..3), None);
}

#[test]
fn cli_uses_raw_file_coordinates_for_nested_multiline_definitions() {
    for semantic in [false, true] {
        for newline in ["\n", "\r\n"] {
            let text = [
                "NB. header 한",
                "",
                "outer=:3 : 0",
                "inner=:{{",
                "missing+y",
                "}}",
                "inner y",
                ")",
                "outer 2",
            ]
            .join(newline);
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join(format!(
                    "provenance-{}-{}-{}.ijs",
                    std::process::id(),
                    semantic,
                    newline.len()
                ));
            std::fs::write(&path, text).unwrap();
            let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_rustj"));
            if semantic {
                command.arg("--semantic-reference");
            }
            let result = command.arg(&path).output().unwrap();
            std::fs::remove_file(&path).unwrap();
            assert_eq!(result.status.code(), Some(1));
            let diagnostic = String::from_utf8(result.stderr).unwrap();
            assert!(
                diagnostic.contains(&format!("definition failure in {}, line 5", path.display())),
                "{diagnostic}"
            );
            assert!(
                diagnostic.contains(&format!(
                    "called from definition in {}, line 7",
                    path.display()
                )),
                "{diagnostic}"
            );
            assert!(diagnostic.contains("line 9"), "{diagnostic}");
        }
    }
}

#[test]
fn frontend_success_and_failure_keep_root_provenance_without_execution() {
    let text = "NB. header 한\r\nf=:{{missing+y}}";
    let unit = SourceUnit::new("analysis.ijs", text);
    let start = text.find("f=:").unwrap();
    let program =
        rustj::parser::parse_frontend_source(unit.origin().slice(start..text.len()).unwrap())
            .unwrap();
    let context = program.frontend.unwrap();
    context.verify().unwrap();
    assert_eq!(
        context
            .source_origin
            .as_ref()
            .unwrap()
            .root_span(0..context.source.len()),
        Some(start..text.len())
    );
    let bad = SourceUnit::new("failed.ijs", "NB. header\r\n1+)");
    let failure =
        rustj::parser::parse_frontend_source(bad.origin().slice(12..15).unwrap()).unwrap_err();
    let origin = failure.context.source_origin.as_ref().unwrap();
    assert_eq!(origin.unit().id(), bad.id());
    assert!(
        origin
            .root_span(failure.error.span().unwrap().clone())
            .is_some()
    );
    let mut engine = Engine::new();
    let captured = engine.eval_source_captured(bad.origin().slice(12..15).unwrap());
    assert!(captured.result.is_err());
    assert_eq!(
        captured
            .capture
            .frontend
            .unwrap()
            .source_origin
            .as_ref()
            .unwrap()
            .unit()
            .id(),
        bad.id()
    );
}

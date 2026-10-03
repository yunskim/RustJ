use rustj::{
    Engine,
    definition_control::{self as control, ControlWord as C},
};

#[test]
fn partition_matches_getsen_spans_and_preserves_intermediate_spacing() {
    let source = "  if. y + 1   do. y else.  0 end. NB. if. ignored";
    let parts = control::partition_line(source).unwrap();
    let texts: Vec<_> = parts
        .iter()
        .map(|p| (&source[p.span.clone()], p.control))
        .collect();
    assert_eq!(
        texts,
        vec![
            ("if.", Some(C::If)),
            ("y + 1   ", None),
            ("do.", Some(C::Do)),
            ("y ", None),
            ("else.", Some(C::Else)),
            ("0 ", None),
            ("end.", Some(C::End))
        ]
    );
}

#[test]
fn quotes_comments_and_inflections_are_not_controls() {
    for source in [
        "'if. do. end.'",
        "a NB. if. do. end.",
        "iffy.",
        "continuex.",
        "IF.",
    ] {
        let parts = control::partition_line(source).unwrap();
        assert_eq!(parts.len(), 1, "{source}");
        assert_eq!(parts[0].control, None);
    }
    // J word formation separates if.x into if. and x; classify actual words.
    let adjacent = control::partition_line("if.x").unwrap();
    assert_eq!(adjacent.len(), 2);
    assert_eq!(adjacent[0].control, Some(C::If));
    assert_eq!(adjacent[1].span, 3..4);
    for source in ["", "  ", "NB. if."] {
        assert!(control::partition_line(source).unwrap().is_empty());
    }
    assert_eq!(
        control::partition_line("'unfinished").unwrap_err().kind(),
        "open quote"
    );
}

#[test]
fn entire_control_inventory_and_named_controls_are_classified() {
    assert_eq!(control::FIXED_WORDS.len(), 20);
    for (word, kind) in control::FIXED_WORDS {
        assert_eq!(control::classify(word).unwrap(), Some(*kind));
    }
    for (word, kind) in [
        ("for_item.", C::For),
        ("for_a_b.", C::For),
        ("goto_exit.", C::Goto),
        ("label_exit.", C::Label),
        ("goto_.", C::Goto),
    ] {
        assert_eq!(control::classify(word).unwrap(), Some(kind));
    }
    for bad in ["for_.", "for_1.", "for_1a.", "for_a_."] {
        assert_eq!(
            control::classify(bad).unwrap_err().kind(),
            "ill-formed name"
        );
    }
    assert_eq!(
        control::classify("for_a__.").unwrap_err().kind(),
        "unsupported"
    );
}

#[test]
fn partition_is_not_a_control_flow_audit_or_executor() {
    // getsen partitions even an unmatched if.; conall will be a separate audit.
    let parts = control::partition_line("if. y do.").unwrap();
    assert_eq!(parts.len(), 3);
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    let version = engine.binding_version("f");
    assert_eq!(
        engine
            .eval("f=:3 : 'goto_done. label_done. y'")
            .unwrap_err()
            .kind(),
        "unsupported"
    );
    assert_eq!(engine.binding_version("f"), version);
    assert_eq!(
        engine
            .eval("f=:3 : 'for_. y do. y end.'")
            .unwrap_err()
            .kind(),
        "ill-formed name"
    );
    assert_eq!(engine.binding_version("f"), version);
}

#[test]
fn definition_body_diagnostics_refer_to_original_source_bytes() {
    for source in [
        "f=:{{  \n1\nfor_1a. y do. y end.}}",
        "f=:3 : 0\n1\nfor_1a. y do. y end.\n)",
        "f=:3 : '''it''''s''\nfor_1a. y do. y end.'",
        "f=:{{  1 2e }}",
    ] {
        let mut engine = Engine::new();
        let error = engine.eval_diagnostic(source).unwrap_err();
        let text = if source.contains("for_1a.") {
            "for_1a."
        } else {
            "1 2e"
        };
        let begin = source.find(text).unwrap();
        assert_eq!(
            error.span(),
            Some(&(begin..begin + text.len())),
            "{source}: {error}"
        );
    }
}

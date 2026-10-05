use rustj::enqueuer::{self, EnqueueClass, EnqueuedPayload};

#[test]
fn enqueue_owns_word_interpretation_and_preserves_source_provenance() {
    let source = "  +/\"1 alpha=:2 NB. ignored";
    let words = enqueuer::enqueue(source).unwrap();

    assert_eq!(
        words.iter().map(|word| word.class).collect::<Vec<_>>(),
        vec![
            EnqueueClass::Verb,
            EnqueueClass::Adverb,
            EnqueueClass::Conjunction,
            EnqueueClass::Noun,
            EnqueueClass::Name,
            EnqueueClass::Assignment,
            EnqueueClass::Noun,
        ]
    );
    assert_eq!(
        words.iter().map(|word| word.word_index).collect::<Vec<_>>(),
        (0..7).collect::<Vec<_>>()
    );
    assert_eq!(&source[words[4].span.clone()], "alpha");
    assert!(!words[4].flags.lookup_name);
    assert!(words[5].flags.global_assignment);
    assert!(!words[5].flags.local_assignment);
    assert!(words[5].flags.assignment_to_name);
    assert!(matches!(words[0].payload, EnqueuedPayload::Verb(_)));
    assert!(matches!(words[1].payload, EnqueuedPayload::Adverb(_)));
    assert!(matches!(words[2].payload, EnqueuedPayload::Conjunction(_)));
}

#[test]
fn legacy_syntax_tokens_are_only_an_adapter_over_enqueue_payloads() {
    let source = "+/\"1 a";
    let enqueued = enqueuer::enqueue(source).unwrap();
    let tokens = rustj::syntax::lex_spanned(source).unwrap();

    assert_eq!(enqueued.len(), tokens.len());
    for (word, token) in enqueued.iter().zip(tokens.iter()) {
        assert_eq!(word.span, token.span);
        assert_eq!(word.word_index, token.word_index);
    }
}

#[test]
fn primitive_resolver_keeps_extensions_as_names_until_parser_binding() {
    use rustj::primitive::{
        ExtensionPrimitive, LoweringKey, PrimitiveContext, PrimitiveHandle, PrimitivePartOfSpeech,
        PrimitiveResolver, PrimitiveSemanticId, PrimitiveSemanticInfo, PrimitiveSourceOrigin,
        REGISTRY_VERSION,
    };

    let extension = PrimitiveHandle {
        semantic_id: PrimitiveSemanticId::Extension("test.addx"),
        source_origin: PrimitiveSourceOrigin::Extension,
        result_pos: PrimitivePartOfSpeech::Verb,
        semantic_info: PrimitiveSemanticInfo {
            registry_version: REGISTRY_VERSION,
        },
        lowering_key: LoweringKey::Extension("test.addx"),
    };
    let context = PrimitiveContext::new(PrimitiveResolver::with_extensions([ExtensionPrimitive {
        spelling: "addx",
        handle: extension,
    }]));

    let words = enqueuer::enqueue_with_context("addx +", &context).unwrap();
    assert_eq!(
        words.iter().map(|word| word.class).collect::<Vec<_>>(),
        vec![EnqueueClass::Name, EnqueueClass::Verb]
    );
    assert!(words[0].flags.lookup_name);
    assert!(matches!(words[0].payload, EnqueuedPayload::Name("addx")));

    let binding = context.resolve_extension_binding("addx").unwrap();
    assert_eq!(binding.source_origin, PrimitiveSourceOrigin::Extension);
    assert_eq!(binding.result_pos, PrimitivePartOfSpeech::Verb);

    let core = context.resolve_core_for_enqueue("+").unwrap();
    assert_eq!(core.source_origin, PrimitiveSourceOrigin::Core);
    assert!(matches!(core.lowering_key, LoweringKey::Core(_)));
}

#[test]
fn unresolved_extension_like_spelling_remains_an_ordinary_name() {
    let words = enqueuer::enqueue("addx").unwrap();
    assert_eq!(words.len(), 1);
    assert_eq!(words[0].class, EnqueueClass::Name);
    assert!(words[0].flags.lookup_name);
    assert!(matches!(words[0].payload, EnqueuedPayload::Name("addx")));
}

#[test]
fn name_lookup_flags_follow_jsource_queue_positions() {
    let words = enqueuer::enqueue("a + b").unwrap();
    assert!(words[0].flags.lookup_name);
    assert!(words[2].flags.lookup_name);

    let assigned = enqueuer::enqueue("a=:b").unwrap();
    assert!(!assigned[0].flags.lookup_name);
    assert!(assigned[1].flags.assignment_to_name);
    assert!(assigned[2].flags.lookup_name);
}

#[test]
fn one_word_non_result_entities_are_rejected_during_enqueue() {
    for source in ["=:", "(", ")"] {
        let error = enqueuer::enqueue(source).unwrap_err();
        assert!(matches!(error.into_unlocated(), rustj::Error::Syntax(_)));
    }
}

#[test]
fn simple_names_may_contain_underscores_but_locatives_remain_explicitly_unsupported() {
    let words = enqueuer::enqueue("foo_bar").unwrap();
    assert_eq!(words.len(), 1);
    assert_eq!(words[0].class, EnqueueClass::Name);
    assert!(matches!(words[0].payload, EnqueuedPayload::Name("foo_bar")));

    assert!(matches!(
        enqueuer::enqueue("foo_").unwrap_err().into_unlocated(),
        rustj::Error::IllFormedName
    ));
    for source in ["foo_bar_", "foo__bar", "foo__"] {
        let error = enqueuer::enqueue(source).unwrap_err();
        assert!(matches!(
            error.into_unlocated(),
            rustj::Error::Unsupported(_)
        ));
    }
}

#[test]
fn invalid_inflections_are_j_spelling_errors_with_enqueue_provenance() {
    for word in [
        "d.", "D:", "I:", "s:", "?:", "`.", "abc.", "99:", "1.5:", "_99:", "_a:", "!..", "$.:",
        "NB..", "NB.:", "]::", "&::",
    ] {
        let source = format!("  2 + {word}");
        let error = enqueuer::enqueue(&source).unwrap_err();
        assert_eq!(error.kind(), "spelling error", "{word}");
        assert_eq!(error.span(), Some(&(6..source.len())), "{word}");
        let context = error.context().unwrap();
        assert_eq!(context.phase, Some(rustj::error::DiagnosticPhase::Enqueue));
        assert_eq!(context.blame_word_index, Some(2));
    }
}

#[test]
fn spelling_errors_do_not_reclassify_valid_names_numeric_dots_or_unsupported_functions() {
    for source in [
        "foo", "foo_bar", "with", "1.5", "_.", "__", "0:", "_9:", "__:", "c.", "/..", "$::", "p..",
        "&.:",
    ] {
        enqueuer::enqueue(source).unwrap();
    }
    for source in ["foo_:", "foo_bar_:", "foo_bar__:"] {
        assert_eq!(enqueuer::enqueue(source).unwrap_err().kind(), "unsupported");
    }
    assert_eq!(
        enqueuer::enqueue("foo__:").unwrap_err().kind(),
        "ill-formed name"
    );
    assert_eq!(enqueuer::enqueue("1j2").unwrap_err().kind(), "unsupported");
    assert_eq!(
        rustj::Engine::new().eval("c. 1").unwrap_err().kind(),
        "unsupported"
    );
}

#[test]
fn locative_name_syntax_is_checked_before_unsupported_lookup() {
    for source in [
        "foo__",
        "foo_bar_",
        "foo_0_",
        "foo_9_",
        "foo_12_",
        "foo_a0_",
        "foo__bar",
        "foo_bar__baz",
        "foo__bar__baz",
        "foo__0",
        "foo___1",
        "foo__bar___1",
        "foo__bar_:",
    ] {
        assert_eq!(
            enqueuer::enqueue(source).unwrap_err().kind(),
            "unsupported",
            "{source}"
        );
    }
    for source in [
        "a_",
        "foo___",
        "foo_00_",
        "foo_01_",
        "foo_0a_",
        "foo_1234567890123456789_",
        "foo__bar_",
        "foo__bar_baz",
        "foo__1__bar",
        "foo____1",
        "foo___bar",
        "foo__bar__",
        "foo_00__:",
        "foo__bar_baz_:",
    ] {
        assert_eq!(
            enqueuer::enqueue(source).unwrap_err().kind(),
            "ill-formed name",
            "{source}"
        );
    }
}

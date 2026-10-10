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
fn simple_and_locative_names_keep_their_lexical_form() {
    let words = enqueuer::enqueue("foo_bar").unwrap();
    assert_eq!(words.len(), 1);
    assert_eq!(words[0].class, EnqueueClass::Name);
    assert!(matches!(words[0].payload, EnqueuedPayload::Name("foo_bar")));

    assert!(matches!(
        enqueuer::enqueue("foo_").unwrap_err().into_unlocated(),
        rustj::Error::IllFormedName
    ));
    for source in ["foo_bar_", "foo__bar", "foo__"] {
        let words = enqueuer::enqueue(source).unwrap();
        assert!(words[0].flags.name_form.is_locative());
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
    for source in ["foo_:", "foo_bar_:"] {
        assert!(enqueuer::enqueue(source).unwrap()[0].flags.abandon_name);
    }
    assert_eq!(
        enqueuer::enqueue("foo_bar__:").unwrap_err().kind(),
        "unsupported"
    );
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
fn locative_name_syntax_is_checked_before_runtime_lookup() {
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
        if source.ends_with(':') {
            assert_eq!(enqueuer::enqueue(source).unwrap_err().kind(), "unsupported");
        } else {
            assert!(
                enqueuer::enqueue(source).unwrap()[0]
                    .flags
                    .name_form
                    .is_locative(),
                "{source}"
            );
        }
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

#[test]
fn name_storage_limits_preserve_c_error_precedence_and_provenance() {
    let mut cases = vec![
        ("a".repeat(255), None),
        ("a".repeat(256), Some("limit error")),
        ("a".repeat(32766), Some("limit error")),
        ("a".repeat(32767), Some("ill-formed name")),
        (format!("{}_b_", "a".repeat(255)), None),
        (format!("{}_b_", "a".repeat(256)), Some("limit error")),
        (format!("a_{}_", "b".repeat(256)), Some("limit error")),
        (format!("a__{}", "b".repeat(256)), Some("limit error")),
        (format!("{}__1a", "a".repeat(256)), Some("ill-formed name")),
        (format!("{}__b_c", "a".repeat(256)), Some("limit error")),
        (format!("{}_", "a".repeat(256)), Some("ill-formed name")),
        (format!("{}_", "a".repeat(257)), Some("limit error")),
    ];
    let by_value = cases
        .iter()
        .map(|(word, expected)| {
            (
                format!("{word}_:"),
                if word.ends_with('_') && expected.is_none() {
                    Some("unsupported")
                } else {
                    *expected
                },
            )
        })
        .collect::<Vec<_>>();
    cases.extend(by_value);
    for (word, expected) in cases {
        let source = format!("2 + {word}");
        match expected {
            None => {
                enqueuer::enqueue(&source).unwrap();
            }
            Some(expected) => {
                let error = enqueuer::enqueue(&source).unwrap_err();
                assert_eq!(error.kind(), expected, "length {}", word.len());
                assert_eq!(error.span(), Some(&(4..source.len())));
                assert_eq!(error.context().unwrap().blame_word_index, Some(2));
            }
        }
    }
}

#[test]
fn numeric_families_are_validated_in_whole_word_context_before_unsupported_payloads() {
    for source in [
        "1j2", "1xr2", "2b102", "2ad90", "2ar1", "1p2", "2x3", "1j2 1r2",
    ] {
        let error = enqueuer::enqueue(source).unwrap_err();
        assert_eq!(error.kind(), "unsupported", "{source}");
        let rustj::Error::Unsupported(reason) = error.into_unlocated() else {
            panic!()
        };
        assert!(reason.contains("validated"), "{source}: {reason}");
    }
    let words = enqueuer::enqueue("1.5 2r3").unwrap();
    let EnqueuedPayload::Noun(value) = &words[0].payload else {
        panic!("real-mode ratio word must construct a noun")
    };
    assert_eq!(value.type_code(), 8);
    assert_eq!(value.float_at(0).unwrap(), 1.5);
    assert_eq!(value.float_at(1).unwrap(), 2.0 / 3.0);
    for word in [
        "1xx", "1j", "1jj2", "2r", "2rr3", "2r3x", "2b", "2b.", "2b_", "2ad", "_2ad90", "1ax2",
        "1p", "1z", "1f", "1.0 1x", "1j2 1x", "2b10 1x", "1E3 2r3",
    ] {
        let source = format!("2 + {word}");
        let error = enqueuer::enqueue(&source).unwrap_err();
        assert_eq!(error.kind(), "ill-formed number", "{word}");
        assert_eq!(error.span(), Some(&(4..source.len())));
        assert_eq!(error.context().unwrap().blame_word_index, Some(2));
    }
}

#[test]
fn precision_and_platform_specific_numeric_grammar_keep_explicit_unknown_boundaries() {
    for source in ["2fs", "2fh", "2fq 1x"] {
        let error = enqueuer::enqueue(source).unwrap_err();
        assert_eq!(error.kind(), "unsupported", "{source}");
        let rustj::Error::Unsupported(reason) = error.into_unlocated() else {
            panic!()
        };
        assert!(!reason.starts_with("validated"), "{source}: {reason}");
    }
    for source in [
        "123", "_123", "1.25", "1e_2", "1E2", "_", "__", "_.", "1 2 3",
    ] {
        enqueuer::enqueue(source).unwrap();
    }
}

#[test]
fn quad_numeric_grammar_validates_mantissa_scale_and_machine_exponent() {
    for source in [
        "2fq",
        "_2fq",
        "2.fq",
        "2.5fq",
        "2e_3fq",
        "2fq _",
        "2fq __",
        "2fq _.",
        "2fq _fq",
        "2fq _.fq",
        "1e401fq",
        "1e_401fq",
        "1e9223372036854775807fq",
        "1e_9223372036854775808fq",
    ] {
        let error = enqueuer::enqueue(source).unwrap_err();
        let rustj::Error::Unsupported(reason) = error.into_unlocated() else {
            panic!("{source}")
        };
        assert!(reason.starts_with("validated"), "{source}: {reason}");
    }
    for source in [
        "2fqz",
        "2fqfq",
        "2efq",
        "2E3fq",
        "2fq _.5",
        "2fq 2r3",
        "2fq 2e",
        "2e__3fq",
        "2e9223372036854775808fq",
        "2e_9223372036854775809fq",
    ] {
        assert_eq!(
            enqueuer::enqueue(source).unwrap_err().kind(),
            "ill-formed number",
            "{source}"
        );
    }
    let error = enqueuer::enqueue("2.1e_9223372036854775808fq").unwrap_err();
    let rustj::Error::Unsupported(reason) = error.into_unlocated() else {
        panic!()
    };
    assert_eq!(reason, "quad scale/exponent construction boundary");
    for word in [
        "2.1e_9223372036854775808fq 2fqz",
        "2fqz 2.1e_9223372036854775808fq",
    ] {
        let source = format!("3 + {word}");
        let error = enqueuer::enqueue(&source).unwrap_err();
        assert_eq!(error.kind(), "ill-formed number", "{source}");
        assert_eq!(error.span(), Some(&(4..source.len())));
        assert_eq!(error.context().unwrap().blame_word_index, Some(2));
    }
}

#[test]
fn windows_hex_float_numeric_parts_validate_without_platform_ffi() {
    for source in [
        "1j0X10",
        "1j_0X10",
        "1j0X1.8",
        "1j0X.8",
        "1j0X1P2",
        "1j0X1P_2",
        "1j0X1P9999",
        "1j0X10r2",
        "0X10ad90",
        "1jNaN",
        "1jInfinity",
        "0Xad90",
        "0Xb1",
        "_0X0P0ad90",
    ] {
        let error = enqueuer::enqueue(source).unwrap_err();
        let rustj::Error::Unsupported(reason) = error.into_unlocated() else {
            panic!("{source}")
        };
        assert!(reason.starts_with("validated"), "{source}: {reason}");
    }
    for source in [
        "1j0X",
        "1j0X.",
        "1j0Xz",
        "1j0X1P",
        "1j0X1P_",
        "1j0X1Pz",
        "1j1X10",
        "1j0X1..2",
        "_0X1ad90",
        "_0X.8ad90",
        "_0X1P_1074ad90",
        "_0X0ad90",
        "_0Xad90",
    ] {
        assert_eq!(
            enqueuer::enqueue(source).unwrap_err().kind(),
            "ill-formed number",
            "{source}"
        );
    }
    let error = enqueuer::enqueue("2.1e_9223372036854775808fq").unwrap_err();
    let rustj::Error::Unsupported(reason) = error.into_unlocated() else {
        panic!()
    };
    assert!(!reason.starts_with("validated"));
}

#[test]
fn hex_polar_magnitude_preserves_round_to_even_zero_and_sticky_remainders() {
    for source in [
        "_0X1P_9999ad90",
        "_0X1P_1076ad90",
        "_0X1P_1075ad90",
        "_0X2P_1076ad90",
        "_0X.8P_1074ad90",
        "_0X0P9999ad90",
        "_0X0001.0000000000P_1075ad90",
        "_0X1P_1075ar1",
        "_0X1P_9223372036854775809ad90",
        "_0X1P_170141183460469231731687303715884105729ad90",
    ] {
        let error = enqueuer::enqueue(source).unwrap_err();
        let rustj::Error::Unsupported(reason) = error.into_unlocated() else {
            panic!("{source}")
        };
        assert!(reason.starts_with("validated"), "{source}: {reason}");
    }
    for source in [
        "_0X1.00000000000001P_1075ad90",
        "_0X1.8P_1075ad90",
        "_0X2.0001P_1076ad90",
        "_0X.80001P_1074ad90",
        "_0X3P_1076ad90",
        "_0X1P9223372036854775808ad90",
        "_0X1P170141183460469231731687303715884105728ad90",
        "_0X1.00000000000000000000000000000000000001P_1075ar1",
    ] {
        let error = enqueuer::enqueue(source).unwrap_err();
        assert_eq!(error.kind(), "ill-formed number", "{source}");
        assert_eq!(error.span(), Some(&(0..source.len())));
    }
    let source = "2.1e_9223372036854775808fq";
    let error = enqueuer::enqueue(source).unwrap_err();
    let rustj::Error::Unsupported(reason) = error.into_unlocated() else {
        panic!("{source}")
    };
    assert!(!reason.starts_with("validated"), "{source}: {reason}");
}

#[test]
fn nan_parentheses_remain_j_tokens_instead_of_c_nan_payloads() {
    for (source, expected) in [
        ("1jNaN(1)", vec!["1jNaN", "(", "1", ")"]),
        ("1jnan()", vec!["1jnan", "(", ")"]),
        ("1jNAN(foo)", vec!["1jNAN", "(", "foo", ")"]),
        ("1j_nan(1)", vec!["1j_nan", "(", "1", ")"]),
    ] {
        assert_eq!(
            rustj::tokenizer::word_texts(source).unwrap(),
            expected,
            "{source}"
        );
        let error = enqueuer::enqueue(source).unwrap_err();
        assert_eq!(error.span(), Some(&(0..expected[0].len())));
        let rustj::Error::Unsupported(reason) = error.into_unlocated() else {
            panic!("{source}")
        };
        assert_eq!(
            reason, "validated numeric family payload construction",
            "{source}"
        );
    }
}

#[test]
fn exact_hex_polar_ratios_follow_division_zero_sign_and_read_windows() {
    for source in [
        "0X1r2ad90",
        "_0X1r_2ad90",
        "0X0r_2ad90",
        "_0X0r0ad90",
        "0X1r0X2ad90",
        "_0X1r_0X2ad90",
        "_0X1P_1074r2ad90",
        "0X1rINFad90",
        "0X1r0X0ad90",
    ] {
        let error = enqueuer::enqueue(source).unwrap_err();
        let rustj::Error::Unsupported(reason) = error.into_unlocated() else {
            panic!("{source}")
        };
        assert!(reason.starts_with("validated"), "{source}: {reason}");
    }
    for source in [
        "_0X1r2ad90",
        "_0X1r0ad90",
        "0X1r_0ad90",
        "_0X1P_1074r1ad90",
        "0X1rNANad90",
    ] {
        assert_eq!(
            enqueuer::enqueue(source).unwrap_err().kind(),
            "ill-formed number",
            "{source}"
        );
    }
    for source in ["_0X1P_1075r1ad90", "0X1P9999r0X1P9999ad90"] {
        let error = enqueuer::enqueue(source).unwrap_err();
        let rustj::Error::Unsupported(reason) = error.into_unlocated() else {
            panic!("{source}")
        };
        assert!(!reason.starts_with("validated"), "{source}: {reason}");
    }
}

#[test]
fn tacit_translator_keeps_copulas_unspecialized_without_losing_name_lookup_order() {
    use rustj::enqueuer::EnqueueEnvironment;
    use rustj::primitive::PrimitiveContext;

    let primitives = PrimitiveContext::core();
    for (environment, local, global, to_name) in [
        (EnqueueEnvironment::TacitTranslator, true, false, false),
        (EnqueueEnvironment::TopLevel, false, true, true),
        (EnqueueEnvironment::ExplicitDefinition, true, false, true),
    ] {
        let words = enqueuer::enqueue_in_environment("a =. b", &primitives, environment)
            .expect("recognized assignment words");
        assert_eq!(words.len(), 3);
        assert_eq!(words[0].class, EnqueueClass::Name);
        assert_eq!(words[1].class, EnqueueClass::Assignment);
        assert_eq!(words[2].class, EnqueueClass::Name);
        assert!(!words[0].flags.lookup_name);
        assert!(words[2].flags.lookup_name);
        assert_eq!(words[1].flags.local_assignment, local);
        assert_eq!(words[1].flags.global_assignment, global);
        assert_eq!(words[1].flags.assignment_to_name, to_name);
        assert_eq!(words[1].word_index, 1);
        assert_eq!(&"a =. b"[words[1].span.clone()], "=.");
    }

    for environment in [
        EnqueueEnvironment::TacitTranslator,
        EnqueueEnvironment::TopLevel,
        EnqueueEnvironment::ExplicitDefinition,
    ] {
        let words = enqueuer::enqueue_in_environment("a =: b", &primitives, environment)
            .expect("recognized global copula");
        assert!(!words[1].flags.local_assignment);
        assert!(words[1].flags.global_assignment);
        assert_eq!(
            words[1].flags.assignment_to_name,
            environment != EnqueueEnvironment::TacitTranslator
        );
    }

    // A non-name left operand never receives the to-name specialization.
    let words = enqueuer::enqueue_in_environment(
        "1 =. 2",
        &primitives,
        EnqueueEnvironment::ExplicitDefinition,
    )
    .unwrap();
    assert!(!words[1].flags.assignment_to_name);
}

#[test]
fn locative_copulas_follow_all_three_enqueue_environments() {
    use enqueuer::{EnqueueEnvironment as Env, NameForm};
    let primitives = rustj::primitive::PrimitiveContext::core();
    for (name, form) in [
        ("a_b", NameForm::Simple),
        ("a_b_", NameForm::DirectLocative),
        ("a__b", NameForm::IndirectLocative),
        ("a__", NameForm::BaseLocative),
    ] {
        for env in [Env::TacitTranslator, Env::TopLevel, Env::ExplicitDefinition] {
            for copula in ["=.", "=:"] {
                let source = format!("  {name}{copula}7");
                let q = enqueuer::enqueue_in_environment(&source, &primitives, env).unwrap();
                assert_eq!(q[0].flags.name_form, form);
                assert!(!q[0].flags.lookup_name);
                assert_eq!(q[0].span, 2..2 + name.len());
                assert_eq!(q[1].flags.assignment_to_name, env != Env::TacitTranslator);
                let global = copula == "=:"
                    || env == Env::TopLevel
                    || (env == Env::ExplicitDefinition && form.is_locative());
                assert_eq!(q[1].flags.global_assignment, global, "{source} {env:?}");
                assert_eq!(q[1].flags.local_assignment, !global);
            }
        }
    }
}

#[test]
fn locative_execution_stops_before_ordinary_namespace_mutation() {
    let mut engine = rustj::runtime::Engine::new();
    engine.eval("kept=:7").unwrap();
    {
        let source = "kept__holder";
        let error = engine.eval_diagnostic(source).unwrap_err();
        assert_eq!(error.kind(), "unsupported", "{source}");
        assert_eq!(
            error.span(),
            Some(&(0..source.split('=').next().unwrap().len()))
        );
        assert_eq!(error.context().unwrap().blame_word_index, Some(0));
        assert!(engine.binding_version("kept__").is_none());
        assert_eq!(engine.eval("kept").unwrap().unwrap().int_at(0).unwrap(), 7);
    }
}

#[test]
fn locative_catalog_and_control_bindings_remain_unsupported() {
    let mut catalog = rustj::static_analysis::StaticAnalyzer::new();
    for name in ["f_base_", "f__holder", "f__"] {
        assert_eq!(
            catalog
                .declare_function(name, rustj::semantic::FunctionPartOfSpeech::Verb)
                .unwrap_err()
                .kind(),
            "unsupported"
        );
        assert_eq!(
            rustj::definition_control::classify(&format!("for_{name}."))
                .unwrap_err()
                .kind(),
            "unsupported"
        );
    }
}

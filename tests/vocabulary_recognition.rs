use rustj::{
    Engine,
    enqueuer::{EnqueueClass, EnqueuedPayload, enqueue},
    primitive::{
        AdverbId, ConjunctionId, PrimitiveId, PrimitivePartOfSpeech, PrimitiveResolver,
        PrimitiveSemanticId, PrimitiveSourceOrigin, PrimitiveSupport,
    },
    semantic::{ExprKind, FunctionHead, FunctionPartOfSpeech},
};

#[test]
fn recognized_core_functions_preserve_pos_identity_without_execution_claims() {
    let mut spellings = std::collections::HashSet::new();
    let ids = PrimitiveId::ALL
        .iter()
        .copied()
        .map(PrimitiveSemanticId::Verb)
        .chain(
            AdverbId::ALL
                .iter()
                .copied()
                .map(PrimitiveSemanticId::Adverb),
        )
        .chain(
            ConjunctionId::ALL
                .iter()
                .copied()
                .map(PrimitiveSemanticId::Conjunction),
        );
    let mut count = 0;
    for id in ids {
        let (spelling, pos, expected_head) = match id {
            PrimitiveSemanticId::Verb(v) => (
                v.spelling(),
                PrimitivePartOfSpeech::Verb,
                FunctionHead::PrimitiveVerb(v),
            ),
            PrimitiveSemanticId::Adverb(v) => (
                v.spelling(),
                PrimitivePartOfSpeech::Adverb,
                FunctionHead::PrimitiveAdverb(v),
            ),
            PrimitiveSemanticId::Conjunction(v) => (
                v.spelling(),
                PrimitivePartOfSpeech::Conjunction,
                FunctionHead::PrimitiveConjunction(v),
            ),
            PrimitiveSemanticId::Extension(_) => unreachable!(),
        };
        assert!(spellings.insert(spelling));
        if id.support() != Some(PrimitiveSupport::Unimplemented) {
            continue;
        }
        count += 1;
        let handle = PrimitiveResolver::core()
            .resolve_core_for_enqueue(spelling)
            .unwrap();
        assert_eq!(handle.semantic_id, id);
        assert_eq!(handle.source_origin, PrimitiveSourceOrigin::Core);
        let queue = enqueue(spelling).unwrap();
        let expected = match pos {
            PrimitivePartOfSpeech::Verb => EnqueueClass::Verb,
            PrimitivePartOfSpeech::Adverb => EnqueueClass::Adverb,
            PrimitivePartOfSpeech::Conjunction => EnqueueClass::Conjunction,
        };
        assert_eq!(queue[0].class, expected);
        assert!(matches!(
            (&queue[0].payload, id),
            (EnqueuedPayload::Verb(_), PrimitiveSemanticId::Verb(_))
                | (EnqueuedPayload::Adverb(_), PrimitiveSemanticId::Adverb(_))
                | (
                    EnqueuedPayload::Conjunction(_),
                    PrimitiveSemanticId::Conjunction(_)
                )
        ));

        let program = rustj::semantic::parse(spelling).unwrap();
        let entity = match program.expression.unwrap().kind {
            ExprKind::VerbValue(v) => v.entity,
            ExprKind::ModifierValue(f) => f,
            _ => panic!(),
        };
        assert_eq!(entity.head, expected_head);
        assert_eq!(entity.innate_ranks(), None);
        assert_eq!(entity.result_pos, FunctionPartOfSpeech::from(pos));
    }
    assert_eq!(count, 105);
    assert_eq!(spellings.len(), 140);
}

#[test]
fn vocabulary_pos_audit_uses_current_source_not_obsolete_dictionary_labels() {
    use rustj::primitive::{PrimitivePartOfSpeech as Pos, PrimitiveResolver};

    // Independently checked against jsoftware/jsource jsrc/t.c at
    // 0a5101cfdd834b23a0b89d455e4f327310520a08 (2026-10-05).
    for (word, expected) in [
        ("H.", Pos::Conjunction),
        ("t.", Pos::Conjunction),
        ("S:", Pos::Conjunction),
        ("m.", Pos::Conjunction),
        ("$::", Pos::Adverb),
        ("M.", Pos::Adverb),
        ("/.", Pos::Adverb),
        ("/..", Pos::Adverb),
        ("c.", Pos::Verb),
        ("T.", Pos::Verb),
        ("Z:", Pos::Verb),
    ] {
        let handle = PrimitiveResolver::core()
            .resolve_core_for_enqueue(word)
            .unwrap_or_else(|| panic!("unrecognized installed primitive: {word}"));
        assert_eq!(handle.result_pos, expected, "{word}");
    }
}

#[test]
fn recognized_but_unimplemented_calls_and_constructors_are_explicit_boundaries() {
    let mut e = Engine::new();
    for source in [
        "3/.", "c. 1", "2 c. 1", "+ t. 0", "+ /..", "+ @ -", "+ m. 7", "+\"c.", "0: 7",
    ] {
        assert_eq!(
            e.eval(source).unwrap_err().kind(),
            "unsupported",
            "{source}"
        );
    }
    let bound = e.prepare_semantic("c. 1").unwrap();
    let graph = rustj::j_graph_ir::Plan::from_bound(bound).unwrap();
    graph.verify().unwrap();
    assert!(e.analyze_a3("c. 1").is_err());
    assert!(e.prepare_semantic("+\"c.").is_err());
}

#[test]
fn bare_function_bindings_preserve_nameless_modifiers_and_late_verb_names() {
    let mut e = Engine::new();
    e.eval("v=:c.").unwrap();
    e.eval("alias=:v").unwrap();
    e.eval("v=:+").unwrap();
    assert_eq!(e.eval("alias 7").unwrap().unwrap().int_at(0).unwrap(), 7);
    e.eval("mod=:t.").unwrap();
    e.eval("saved=:mod").unwrap();
    e.eval("mod=:\"").unwrap();
    let program = e.prepare_semantic("saved").unwrap();
    let ExprKind::ModifierValue(f) = program.program.expression.unwrap().kind else {
        panic!()
    };
    assert_eq!(
        f.head,
        FunctionHead::PrimitiveConjunction(ConjunctionId::from_spelling("t.").unwrap())
    );
    assert_eq!(e.eval("+ saved 0").unwrap_err().kind(), "unsupported");
    for source in ["f=:(c. [. +)", "g=:(+ ]. c.)"] {
        let report = e.eval_captured(source);
        report.result.unwrap();
        report.capture.verify().unwrap();
        rustj::j_graph_ir::Plan::from_capture(&report.capture)
            .unwrap()
            .graph
            .verify()
            .unwrap();
    }
}

#[test]
fn builtin_alphabet_and_ace_are_actual_nouns() {
    let mut e = Engine::new();
    let alphabet = e.eval("a.").unwrap().unwrap();
    assert_eq!(alphabet.shape(), &[256]);
    assert_eq!(alphabet.type_code(), 2);
    let rustj::Data::Char(bytes) = alphabet.data() else {
        panic!()
    };
    for index in 0..256 {
        assert_eq!(bytes[index], index as u8);
    }
    let ace = e.eval("a:").unwrap().unwrap();
    assert_eq!(ace.type_code(), 32);
    assert!(ace.shape().is_empty());
    let rustj::Data::Boxed(values) = ace.data() else {
        panic!()
    };
    assert_eq!(values[0].shape(), &[0]);
    assert_eq!(values[0].type_code(), 1);
    assert!(values[0].is_empty());
    for word in ["?:", "`.", "s:", "d.", "I:", "with"] {
        assert!(
            PrimitiveResolver::core()
                .resolve_core_for_enqueue(word)
                .is_none()
        );
    }
}

#[test]
fn support_tags_do_not_admit_unimplemented_execution_or_rank_contracts() {
    for &id in PrimitiveId::ALL {
        if id.support() != PrimitiveSupport::Unimplemented {
            continue;
        }
        assert!(rustj::contracts::innate_rank(id).is_none());
        for source in [
            format!("{} 1", id.spelling()),
            format!("1 {} 2", id.spelling()),
        ] {
            assert_eq!(
                Engine::new().eval(&source).unwrap_err().kind(),
                "unsupported",
                "{source}"
            );
            assert!(Engine::new().analyze_a3(&source).is_err(), "{source}");
        }
    }
    for &id in AdverbId::ALL {
        if id.support() == PrimitiveSupport::Unimplemented {
            let source = format!("+ {}", id.spelling());
            assert_eq!(
                Engine::new().eval(&source).unwrap_err().kind(),
                "unsupported",
                "{source}"
            );
        }
    }
    for &id in ConjunctionId::ALL {
        if id.support() == PrimitiveSupport::Unimplemented {
            let source = format!("+ {} -", id.spelling());
            assert_eq!(
                Engine::new().eval(&source).unwrap_err().kind(),
                "unsupported",
                "{source}"
            );
        }
    }
}

#[test]
fn unified_identity_keeps_unknown_graph_rules_and_rejects_stale_versions() {
    use rustj::j_graph_ir::{GraphRuleRef, NodeKind, ResourceRuleRef};
    for source in ["c. 1", "! 1", "0: 1"] {
        let bound = Engine::new().prepare_semantic(source).unwrap();
        let plan = rustj::j_graph_ir::Plan::from_bound(bound).unwrap();
        plan.verify().unwrap();
        let mut applies = 0;
        for node in &plan.nodes {
            if let NodeKind::Apply { rules, .. } = &node.kind {
                applies += 1;
                assert_eq!(rules.shape, GraphRuleRef::DynamicOrUnknown);
                assert_eq!(rules.dtype, GraphRuleRef::DynamicOrUnknown);
                assert_eq!(rules.rank_cell, GraphRuleRef::DynamicOrUnknown);
                assert_eq!(rules.effect, GraphRuleRef::DynamicOrUnknown);
                assert_eq!(rules.resource, ResourceRuleRef::Unknown);
            }
        }
        assert_eq!(applies, 1);
        let mut bad_registry = plan.clone();
        bad_registry.header.primitive_registry_version -= 1;
        assert!(bad_registry.verify().is_err());
        let mut bad_schema = plan;
        bad_schema.header.schema.minor -= 1;
        assert!(bad_schema.verify().is_err());
    }
}

use rustj::{
    Engine,
    enqueuer::{EnqueueClass, EnqueuedPayload, enqueue},
    primitive::{
        PrimitivePartOfSpeech, PrimitiveResolver, PrimitiveSemanticId, PrimitiveSourceOrigin,
        VocabularyPrimitive,
    },
    semantic::{ExprKind, FunctionHead, FunctionPartOfSpeech},
};

#[test]
fn recognized_core_functions_preserve_pos_identity_without_execution_claims() {
    let mut spellings = std::collections::HashSet::new();
    for &id in VocabularyPrimitive::ALL {
        assert!(spellings.insert(id.spelling()));
        let handle = PrimitiveResolver::core()
            .resolve_core_for_enqueue(id.spelling())
            .unwrap();
        assert_eq!(handle.semantic_id, PrimitiveSemanticId::Vocabulary(id));
        assert_eq!(handle.source_origin, PrimitiveSourceOrigin::Core);
        let queue = enqueue(id.spelling()).unwrap();
        let expected = match id.part_of_speech() {
            PrimitivePartOfSpeech::Verb => EnqueueClass::Verb,
            PrimitivePartOfSpeech::Adverb => EnqueueClass::Adverb,
            PrimitivePartOfSpeech::Conjunction => EnqueueClass::Conjunction,
        };
        assert_eq!(queue[0].class, expected);
        let EnqueuedPayload::Function(f) = &queue[0].payload else {
            panic!()
        };
        assert_eq!(f.head, FunctionHead::VocabularyPrimitive(id));
        assert_eq!(f.innate_ranks(), None);
        let program = rustj::semantic::parse(id.spelling()).unwrap();
        let entity = match program.expression.unwrap().kind {
            ExprKind::VerbValue(v) => v.entity,
            ExprKind::ModifierValue(f) => f,
            _ => panic!(),
        };
        assert_eq!(entity.head, f.head);
        assert_eq!(
            entity.result_pos,
            FunctionPartOfSpeech::from(id.part_of_speech())
        );
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
        FunctionHead::VocabularyPrimitive(VocabularyPrimitive::from_spelling("t.").unwrap())
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
        assert!(VocabularyPrimitive::from_spelling(word).is_none());
    }
}

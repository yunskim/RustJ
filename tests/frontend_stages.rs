use rustj::enqueuer::{EnqueueEnvironment, enqueue, enqueue_in_environment};
use rustj::primitive::PrimitiveContext;
#[test]
fn copula_environment_and_name_lookup_match_jtenqueue() {
    let q = enqueue("a=.b").unwrap();
    assert!(!q[0].flags.lookup_name);
    assert!(q[1].flags.global_assignment && q[1].flags.assignment_to_name);
    assert!(!q[1].flags.local_assignment);
    assert!(q[2].flags.lookup_name);
    let q = enqueue_in_environment(
        "a=.b",
        &PrimitiveContext::core(),
        EnqueueEnvironment::ExplicitDefinition,
    )
    .unwrap();
    assert!(q[1].flags.local_assignment && !q[1].flags.global_assignment);
    let q = enqueue_in_environment(
        "a=:b",
        &PrimitiveContext::core(),
        EnqueueEnvironment::ExplicitDefinition,
    )
    .unwrap();
    assert!(q[1].flags.global_assignment && !q[1].flags.local_assignment);
}
#[test]
fn parser_consumes_top_level_local_copula_as_global_assignment() {
    let mut engine = rustj::Engine::new();
    engine.eval("a=.1 2 3").unwrap();
    assert_eq!(
        engine.eval("a").unwrap().unwrap().json(),
        engine.eval("1 2 3").unwrap().unwrap().json()
    );
}

#[test]
fn malformed_fields_have_j_error_classes_and_word_provenance() {
    for (source, kind) in [
        ("a=:1q", "ill-formed number"),
        ("a=:1e", "ill-formed number"),
        ("a=:1.2.3", "ill-formed number"),
        ("foo_=:1", "ill-formed name"),
    ] {
        let error = enqueue(source).unwrap_err();
        assert_eq!(error.kind(), kind);
        assert!(error.span().is_some());
        assert!(error.context().unwrap().blame_word_index.is_some());
    }
    assert_eq!(enqueue("1j2").unwrap_err().kind(), "unsupported");
    for source in ["foo__", "foo_bar_"] {
        assert!(enqueue(source).unwrap()[0].flags.name_form.is_locative());
        assert_eq!(
            rustj::semantic::parse(source).unwrap_err().kind(),
            "unsupported"
        );
    }
}

#[test]
fn modifier_results_and_assignments_preserve_pos_before_lowering() {
    use rustj::semantic::{ExprKind, FunctionPartOfSpeech, parse};
    let mut engine = rustj::Engine::new();
    for (source, name, pos) in [
        ("a=:/", "a", FunctionPartOfSpeech::Adverb),
        ("c=:@:", "c", FunctionPartOfSpeech::Conjunction),
    ] {
        let program = parse(source).unwrap();
        assert!(
            matches!(program.expression.unwrap().kind, ExprKind::ModifierValue(ref f) if f.result_pos == pos)
        );
        assert!(engine.eval(source).unwrap().is_none());
        let program = engine.prepare_semantic(name).unwrap();
        assert!(
            matches!(program.program.expression.unwrap().kind, ExprKind::ModifierValue(ref f) if f.result_pos == pos)
        );
        assert_eq!(engine.eval(name).unwrap_err().kind(), "unsupported");
    }
}

#[test]
fn pytorch_named_nn_catalog_is_complete_and_frontend_only() {
    use rustj::{
        enqueuer::{EnqueueClass, enqueue},
        nn_extensions::{FAMILIES, NAMES, ProofStatus, family, frontend_preview_context},
        primitive::{PrimitivePartOfSpeech, PrimitiveSemanticId},
        semantic::ExprKind,
    };
    use std::collections::HashSet;

    assert_eq!(FAMILIES.len(), 34);
    assert_eq!(
        FAMILIES
            .iter()
            .map(|spec| spec.id)
            .collect::<HashSet<_>>()
            .len(),
        34,
    );
    let preview = frontend_preview_context();
    let engine = rustj::Engine::with_primitive_context(preview.clone());
    let mut names = HashSet::new();
    for spec in NAMES {
        assert!(names.insert(spec.spelling), "duplicate: {}", spec.spelling);
        let f = family(spec.family_id).expect("missing family");
        assert_eq!(f.empty_fill_proof, ProofStatus::Unverified);
        assert_eq!(f.error_order_proof, ProofStatus::Unverified);
        assert_eq!(f.numeric_proof, ProofStatus::Unverified);
        assert_eq!(spec.intrinsic_ranks.is_some(), spec.pos == PrimitivePartOfSpeech::Verb);
        assert!(spec.intrinsic_ranks.is_none() || spec.derived_verb_ranks.is_none());
        if !spec.frontend_preview {
            assert!(preview.resolve_extension_binding(spec.spelling).is_none());
            continue;
        }
        let words = enqueue(spec.spelling).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].class, EnqueueClass::Name);
        let handle = preview.resolve_extension_binding(spec.spelling).unwrap();
        assert_eq!(
            handle.semantic_id,
            PrimitiveSemanticId::Extension(spec.semantic_identity),
        );
        assert_eq!(handle.result_pos, spec.pos);
        let parsed = engine.parse_frontend(spec.spelling).unwrap();
        parsed.frontend.as_ref().unwrap().verify().unwrap();
        let kind = parsed.expression.unwrap().kind;
        match spec.pos {
            PrimitivePartOfSpeech::Verb => {
                assert!(matches!(kind, ExprKind::VerbValue(_)), "{}", spec.spelling);
            }
            PrimitivePartOfSpeech::Adverb | PrimitivePartOfSpeech::Conjunction => {
                assert!(matches!(kind, ExprKind::ModifierValue(_)), "{}", spec.spelling);
            }
        }
    }
    // Names are never reserved tokens; an explicit user binding wins.
    let mut engine = rustj::Engine::with_primitive_context(preview);
    engine.eval("relu=:42").unwrap();
    let result = engine.parse_frontend("relu").unwrap();
    assert!(matches!(
        result.expression.unwrap().kind,
        ExprKind::ReadName(ref name) if name == "relu"
    ));
}

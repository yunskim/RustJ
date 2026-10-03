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
    for source in ["1j2", "123x", "1r2", "foo__", "foo_bar_"] {
        assert_eq!(enqueue(source).unwrap_err().kind(), "unsupported");
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

use rustj::{
    Engine,
    analysis::CallTarget,
    contracts::{self, Effect, Valence},
    logical_ir::OpKind,
    primitive::{AdverbId, PrimitiveId},
    syntax::{self, Token},
};

#[test]
fn registered_spellings_reach_the_lexer_and_logical_plan() {
    let mut spellings = std::collections::HashSet::new();
    for &id in PrimitiveId::ALL {
        assert!(spellings.insert(id.spelling()));
        let tokens = syntax::lex(id.spelling()).unwrap();
        assert!(matches!(tokens.as_slice(), [Token::Verb(s)] if *s == id));
        let parsed = rustj::semantic::parse(id.spelling()).unwrap();
        let rustj::semantic::ExprKind::VerbValue(verb) = parsed.expression.unwrap().kind else {
            panic!()
        };
        assert_eq!(verb.target, rustj::semantic::VerbTarget::Primitive(id));
        let plan = Engine::new()
            .analyze_a3(&format!("f=:{}", id.spelling()))
            .unwrap();
        let result = plan.result.unwrap();
        let producer = plan.values[result.0].producer;
        let OpKind::VerbReference(callable) = &plan.operations[producer.0].kind else {
            panic!()
        };
        assert_eq!(callable.target, CallTarget::Primitive(id));
        // Implicit locatives are recognized primitives with context-dependent
        // execution, so lexical support must not imply a pure kernel contract.
        // Cap has no callable valence. Argument selection executes at runtime;
        // its compiler alias/evaluation proof remains conservative.
        if matches!(
            id,
            PrimitiveId::Udot
                | PrimitiveId::Vdot
                | PrimitiveId::Cap
                | PrimitiveId::Left
                | PrimitiveId::Right
        ) {
            for valence in [Valence::Monad, Valence::Dyad] {
                assert_eq!(contracts::for_primitive(id, valence), contracts::unknown());
            }
            continue;
        }
        assert!(
            [Valence::Monad, Valence::Dyad]
                .into_iter()
                .any(|v| contracts::for_primitive(id, v).effect != Effect::Unknown)
        );
    }
}

#[test]
fn registered_adverb_spellings_reach_the_shared_frontend() {
    for id in [AdverbId::Slash, AdverbId::Sldot, AdverbId::Bslash] {
        let tokens = syntax::lex(id.spelling()).unwrap();
        assert!(matches!(tokens.as_slice(), [Token::Adverb(actual)] if *actual == id));
    }
}

#[test]
fn valence_and_dynamic_names_do_not_get_conflated() {
    assert_eq!(
        contracts::for_primitive(PrimitiveId::Eps, Valence::Monad).effect,
        Effect::Unknown
    );
    assert_eq!(
        contracts::for_primitive(PrimitiveId::Eps, Valence::Dyad).effect,
        Effect::Pure
    );
    assert_ne!(
        PrimitiveId::from_spelling("e."),
        PrimitiveId::from_spelling("E.")
    );
    for name in ["with", "custom", "i.."] {
        assert_eq!(PrimitiveId::from_spelling(name), None);
        assert_eq!(
            contracts::lookup(name, Valence::Dyad).effect,
            Effect::Unknown
        );
    }
    let plan = Engine::new().analyze_a3("custom 3").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    let OpKind::SemanticCall(call) = &plan.operations[producer.0].kind else {
        panic!()
    };
    assert!(matches!(call.callable.target, CallTarget::Dynamic(_)));
    assert_eq!(call.contract.effect, Effect::Unknown);
}

#[test]
fn all_core_primitive_identifiers_use_canonical_source_names() {
    use rustj::primitive::{PrimitiveName, PrimitiveNames, VocabularyPrimitive};
    let mut spellings = std::collections::HashSet::new();
    for names in PrimitiveNames::ALL {
        assert!(spellings.insert(names.spelling));
        assert!(names.name.jsource_name().starts_with('C'));
        assert_eq!(PrimitiveNames::from_spelling(names.spelling), Some(names));
    }
    for &id in PrimitiveId::ALL {
        assert_eq!(id.names().spelling, id.spelling());
        assert_eq!(format!("{id:?}"), format!("{:?}", id.names().name));
    }
    for &id in VocabularyPrimitive::ALL {
        assert_eq!(id.names().spelling, id.spelling());
    }
    for spelling in ["/", "/.", "\\", "]:"] {
        let id = AdverbId::from_spelling(spelling).unwrap();
        assert_eq!(id.names().spelling, spelling);
        assert_eq!(format!("{id:?}"), format!("{:?}", id.names().name));
    }
    for spelling in ["\"", "@:", "[.", "]."] {
        let id = rustj::primitive::ConjunctionId::from_spelling(spelling).unwrap();
        assert_eq!(id.names().spelling, spelling);
        assert_eq!(format!("{id:?}"), format!("{:?}", id.names().name));
    }
    let plus = PrimitiveId::Plus.names();
    assert_eq!(plus.name, PrimitiveName::Plus);
    assert_eq!(plus.name.jsource_name(), "CPLUS");
    assert_eq!(plus.monad, Some("jtatomic1"));
    assert_eq!(plus.dyad, Some("jtatomic2"));
    let domino = PrimitiveNames::from_spelling("%.").unwrap();
    assert_eq!(domino.monad, Some("jtminv"));
    assert_eq!(domino.dyad, Some("jtmdiv"));
    let resolver = rustj::primitive::PrimitiveResolver::core();
    for spelling in ["a.", "a:", "=.", "=:", "(", ")"] {
        assert!(PrimitiveNames::from_spelling(spelling).is_some());
        assert!(resolver.resolve_core_for_enqueue(spelling).is_none());
    }
    for spelling in ["ordinaryname", "++", "each"] {
        assert!(PrimitiveNames::from_spelling(spelling).is_none());
    }
}

use rustj::{
    Engine,
    analysis::CallTarget,
    logical_ir::OpKind,
    contracts::{self, Effect, Valence},
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
        // Every registered symbol has at least one supported valence contract.
        assert!(
            [Valence::Monad, Valence::Dyad]
                .into_iter()
                .any(|v| contracts::for_primitive(id, v).effect != Effect::Unknown)
        );
    }
}


#[test]
fn registered_adverb_spellings_reach_the_shared_frontend() {
    for id in [AdverbId::Insert, AdverbId::PrefixInfix] {
        let tokens = syntax::lex(id.spelling()).unwrap();
        assert!(matches!(tokens.as_slice(), [Token::Adverb(actual)] if *actual == id));
    }
}

#[test]
fn valence_and_dynamic_names_do_not_get_conflated() {
    assert_eq!(
        contracts::for_primitive(PrimitiveId::Member, Valence::Monad).effect,
        Effect::Unknown
    );
    assert_eq!(
        contracts::for_primitive(PrimitiveId::Member, Valence::Dyad).effect,
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

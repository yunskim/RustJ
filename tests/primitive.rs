use rustj::{
    Engine,
    analysis::{CallTarget, Operation},
    contracts::{self, Effect, Valence},
    primitive::PrimitiveId,
    syntax::{self, Token},
};

#[test]
fn registered_spellings_reach_the_lexer_and_logical_plan() {
    let mut spellings = std::collections::HashSet::new();
    for &id in PrimitiveId::ALL {
        assert!(spellings.insert(id.spelling()));
        let tokens = syntax::lex(id.spelling()).unwrap();
        assert!(matches!(tokens.as_slice(), [Token::Verb(s)] if *s == id.spelling()));
        let plan = Engine::new()
            .analyze(&format!("f=:{}", id.spelling()))
            .unwrap();
        let Operation::VerbReference(callable) = &plan.nodes[plan.result.unwrap().0].operation
        else {
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
    let plan = Engine::new().analyze("custom 3").unwrap();
    let Operation::Call {
        callable, contract, ..
    } = &plan.nodes[plan.result.unwrap().0].operation
    else {
        panic!()
    };
    assert!(matches!(callable.target, CallTarget::Dynamic(_)));
    assert_eq!(contract.effect, Effect::Unknown);
}

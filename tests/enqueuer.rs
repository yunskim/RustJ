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
    assert!(words[4].flags.lookup_name);
    assert!(words[5].flags.global_assignment);
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

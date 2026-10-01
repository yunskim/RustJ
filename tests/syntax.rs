use rustj::{
    Engine, Value,
    syntax::{Token, lex},
};

#[test]
fn compact_tokens_borrow_names_and_keep_literal_semantics() {
    assert!(std::mem::size_of::<Token<'_>>() < std::mem::size_of::<Value>());
    let source = String::from("alpha + 2");
    let tokens = lex(&source).unwrap();
    let Token::Name(name) = tokens[0] else {
        panic!()
    };
    assert_eq!(name.as_ptr(), source.as_ptr());
    assert!(matches!(tokens[2], Token::Scalar(_)));
    let modifier_tokens = lex("+/\"1").unwrap();
    assert!(matches!(
        modifier_tokens[0],
        Token::Verb(rustj::primitive::PrimitiveId::Add)
    ));
    assert!(matches!(
        modifier_tokens[1],
        Token::Adverb(rustj::primitive::AdverbId::Insert)
    ));
    assert!(matches!(
        modifier_tokens[2],
        Token::Conjunction(rustj::primitive::ConjunctionId::Rank)
    ));
    let mut e = Engine::new();
    for (s, kind, shape) in [
        ("0", 1, vec![]),
        ("2", 4, vec![]),
        ("2.0", 8, vec![]),
        ("_2", 4, vec![]),
        ("1e_2", 8, vec![]),
        ("1  2\t3", 4, vec![3]),
        ("1 0 1", 1, vec![3]),
        ("1 _ 3", 8, vec![3]),
        ("'x'", 2, vec![]),
        ("'xy'", 2, vec![2]),
    ] {
        let v = e.eval(s).unwrap().unwrap();
        assert_eq!(v.type_code(), kind, "{s}");
        assert_eq!(v.shape(), shape, "{s}");
    }
    for s in ["1q", "_x", "2 foo", "+/"] {
        assert!(e.eval(s).is_err(), "{s}");
    }
    assert_eq!(e.eval("+/\"_1 i. 2 3").unwrap().unwrap().display(), "3 12");
}

#[test]
fn word_formation_states_and_spans() {
    fn words(s: &str) -> Vec<&str> {
        rustj::scanner::scan(s.as_bytes())
            .unwrap()
            .into_iter()
            .map(|r| &s[r])
            .collect()
    }
    assert_eq!(words("1 2: 3"), vec!["1", "2:", "3"]);
    assert_eq!(words("1  2\t3"), vec!["1  2\t3"]);
    assert_eq!(words("1\n2"), vec!["1", "\n", "2"]);
    assert_eq!(
        words("{{ y }} {{. }}:"),
        vec!["{{", "y", "}}", "{", "{.", "}", "}:"]
    );
    assert_eq!(words("{}} }{{"), vec!["{", "}", "}", "}", "{", "{"]);
    assert_eq!(
        words("NB.. NB.: NB. rest"),
        vec!["NB..", "NB.:", "NB. rest"]
    );
    assert_eq!(words("'it''s'"), vec!["'it''s'"]);
    assert!(rustj::scanner::scan(b"'open").is_err());
    let spanned = rustj::syntax::lex_spanned("  a + 2").unwrap();
    assert_eq!(
        spanned.iter().map(|s| s.span.clone()).collect::<Vec<_>>(),
        vec![2..3, 4..5, 6..7]
    );
    let mut e = Engine::new();
    for s in ["1 NB.. 2", "1 NB.: 2"] {
        assert!(matches!(e.eval(s), Err(rustj::Error::Spelling)));
        assert!(matches!(
            e.eval_semantic_reference(s),
            Err(rustj::Error::Spelling)
        ));
    }
    assert!(e.eval("1\n2").is_err());
}


#[test]
fn raw_word_formation_keeps_trailing_comment_while_parser_words_exclude_it() {
    let source = b"1 + 2 NB. trailing comment";
    let raw = rustj::scanner::scan(source).unwrap();
    let parse = rustj::scanner::parse_word_spans(source).unwrap();

    assert_eq!(&source[raw.last().unwrap().clone()], b"NB. trailing comment");
    assert_eq!(raw.len(), parse.len() + 1);
    assert_eq!(
        parse.iter().map(|span| &source[span.clone()]).collect::<Vec<_>>(),
        vec![b"1".as_slice(), b"+".as_slice(), b"2".as_slice()]
    );
}

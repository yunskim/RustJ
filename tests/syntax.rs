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

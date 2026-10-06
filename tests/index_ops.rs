use rustj::{Engine, Error};
fn eval(s: &str) -> String {
    Engine::new().eval(s).unwrap().unwrap().json()
}
#[test]
fn first_last_items_and_missing() {
    assert_eq!(eval("3 1 3 2 i.3 4 1"), eval("0 4 1"));
    assert_eq!(eval("3 1 3 2 i:3 4 1"), eval("2 4 1"));
    assert_eq!(eval("'abca' i:'ax'"), eval("3 4"));
    assert_eq!(eval("(i.2 3)i.(2 3$3 4 5 0 1 2)"), eval("1 0 + 0"));
    assert_eq!(eval("(i.2 0)i:(i.3 0)"), eval("3$(1+0)"));
    assert_eq!(eval("(i.2 3)i.(i.2 2)"), eval("2 2"));
    assert_eq!(
        eval("(9223372036854775807 9223372036854775806)i.9223372036854775806"),
        eval("1+0")
    );
}
#[test]
fn generators_and_alias_preservation() {
    assert_eq!(eval("i:_3"), eval("3 2 1 0 _1 _2 _3"));
    assert_eq!(eval("i:2.5"), eval("_2.5 _1.5 _0.5 0.5 1.5 2.5"));
    assert_eq!(eval("I.2 0 3"), eval("0 0 2 2 2"));
    assert_eq!(eval("I.i.0"), eval("i.0"));
    let mut e = Engine::new();
    for s in ["a=:3 1 3 2", "b=:a", "a=:a i:3 4 1"] {
        e.eval(s).unwrap();
    }
    assert_eq!(e.eval("b").unwrap().unwrap().json(), eval("3 1 3 2"));
    assert!(matches!(e.eval("a=:I._1 2"), Err(Error::Domain)));
    assert_eq!(e.eval("a").unwrap().unwrap().json(), eval("2 4 1"));
    assert!(matches!(e.eval("i:2.2"), Err(Error::Domain)));
}

#[test]
fn membership_and_overlapping_pattern() {
    assert!(matches!(
        Engine::new().eval("(i.0) E. (1)"),
        Err(Error::Rank)
    ));
    assert_eq!(eval("'ana' E. 'banana'"), eval("0 1 0 1 0 0"));
    assert_eq!(eval("'' E. 'abc'"), eval("1 1 1"));
    assert_eq!(eval("1 2 4 e. 2 4"), eval("0 1 1"));
    assert_eq!(eval("(i.2 3)e.(i.2 3)"), eval("1 1"));
    assert_eq!(eval("'abcd' E. 'ab'"), eval("0 0"));
    assert_eq!(eval("2 2 E. 2 2 2"), eval("1 1 0"));
}

#[test]
fn direct_range_index_preserves_first_last_missing_and_negative_keys() {
    // At least 33 comparisons force an index; the narrow domain admits a
    // bounded direct-address table rather than a general HashMap.
    assert_eq!(
        eval("_2 _1 0 _2 1 i. _2 1 3 _1 _2 1 3"),
        eval("0 4 5 1 0 4 5")
    );
    assert_eq!(
        eval("_2 _1 0 _2 1 i: _2 1 3 _1 _2 1 3"),
        eval("3 4 5 1 3 4 5")
    );
    assert_eq!(
        eval("_2 1 10 1 _2 1 10 e. _2 _1 0 1"),
        eval("1 1 0 1 1 1 0")
    );
}

#[test]
fn exact_search_keeps_wide_range_and_boolean_membership_semantics() {
    assert_eq!(
        eval("_1000000000 0 1000000000 i. 0 1000000000 2 _1000000000 0"),
        eval("1 2 3 0 1")
    );
    assert_eq!(
        eval("0 1 0 1 0 1 0 1 e. 0 1"),
        eval("1 1 1 1 1 1 1 1")
    );
    assert_eq!(
        eval("0 1 0 1 0 1 0 1 i: 0 1 2 0 1"),
        eval("6 7 8 6 7")
    );
}

#[test]
fn member_preserves_cell_shapes_and_empty_query_semantics() {
    assert_eq!(eval("(i.2 3)e.(i.2 3)"), eval("1 1"));
    assert_eq!(eval("(i.0)e.3 4"), eval("0 0"));
    assert_eq!(eval("(i.0)i.3 4"), eval("0 0"));
}

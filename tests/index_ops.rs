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

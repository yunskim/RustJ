use rustj::{Engine, Error};
fn eval(s: &str) -> String {
    Engine::new().eval(s).unwrap().unwrap().json()
}
#[test]
fn rearrangement_and_fill() {
    assert_eq!(eval("|.i.2 3"), eval("2 3 $ 3 4 5 0 1 2"));
    assert_eq!(eval("|:i.2 3"), eval("3 2 $ 0 3 1 4 2 5"));
    assert_eq!(eval("|:|:i.2 3 4"), eval("i.2 3 4"));
    assert_eq!(eval("_1 |. i.5"), eval("4 0 1 2 3"));
    assert_eq!(eval("7 |. i.5"), eval("2 3 4 0 1"));
    assert_eq!(eval("_5 {. 'abc'"), eval("'  abc'"));
    assert_eq!(eval("5 {. 'abc'"), eval("'abc  '"));
    assert_eq!(eval("_4 {. i.2 3"), eval("4 3 $ 0 0 0 0 0 0 0 1 2 3 4 5"));
    assert_eq!(eval("_1 }. i.2 3"), eval("1 3 $ 0 1 2"));
    assert_eq!(eval("5 }. i.2 3"), eval("i.0 3"));
    assert_eq!(eval("3 {. i.0 2"), eval("3 2 $ (0+0)"));
    assert_eq!(eval("|:i.2 0 3"), eval("i.3 0 2"));
}
#[test]
fn aliases_errors_and_extreme_counts() {
    let mut e = Engine::new();
    for s in ["a=:i.2 3", "b=:a", "a=:|.a"] {
        e.eval(s).unwrap();
    }
    assert_eq!(e.eval("b").unwrap().unwrap().json(), eval("i.2 3"));
    let before = e.eval("a").unwrap().unwrap().json();
    assert!(matches!(e.eval("a=:1.5 {. a"), Err(Error::Domain)));
    assert_eq!(e.eval("a").unwrap().unwrap().json(), before);
    assert!(matches!(
        e.eval("_9223372036854775808 {. i.2"),
        Err(Error::Limit)
    ));
    assert_eq!(eval("_9223372036854775808 }. i.2"), eval("i.0"));
    assert_eq!(eval("|. |. i.129"), eval("i.129"));
    assert!(matches!(e.eval("1 2 {. i.2 3"), Err(Error::Unsupported(_))));
}

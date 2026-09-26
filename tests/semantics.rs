use rustj::{Engine, Error};

fn eval(s: &str) -> String {
    Engine::new().eval(s).unwrap().unwrap().json()
}

#[test]
fn right_to_left_and_parentheses() {
    assert!(eval("10 - 3 - 2").contains("[9]"));
    assert!(eval("(10 - 3) - 2").contains("[5]"));
    assert!(eval("2 * 3 + 4").contains("[14]"));
}
#[test]
fn shapes_extension_and_rank() {
    assert_eq!(
        eval("i. 2 3"),
        "{\"type\":4,\"shape\":[2,3],\"data\":[0,1,2,3,4,5]}"
    );
    assert!(eval("10 20 + i. 2 3").contains("[10,11,12,23,24,25]"));
    assert!(eval("+/\"1 i. 2 3").contains("[3,12]"));
    assert!(eval("$\"1 i. 2 3").contains("\"shape\":[2,1],\"data\":[3,3]"));
    assert!(eval("+/ i. 2 3").contains("[3,5,7]"));
    assert!(eval("i. _3").contains("[2,1,0]"));
}
#[test]
fn assignment_is_transactional_and_aliases_are_preserved() {
    let mut e = Engine::new();
    e.eval("a =: i. 4").unwrap();
    e.eval("b =: a").unwrap();
    e.eval("a =: a + 10").unwrap();
    assert_eq!(e.eval("b").unwrap().unwrap().display(), "0 1 2 3");
    assert!(e.eval("a =: 2 3 + 4 5 6").is_err());
    assert_eq!(e.eval("a").unwrap().unwrap().display(), "10 11 12 13");
}
#[test]
fn edge_cases() {
    assert!(eval("i. 0").contains("\"shape\":[0],\"data\":[]"));
    assert!(eval("9223372036854775807 + 1").contains("\"type\":8"));
    assert!(eval("9223372036854775807 = 9223372036854775806").contains("[0]"));
    assert!(eval("_1 { 10 20 30").contains("[30]"));
    assert_eq!(Engine::new().eval("10 { 1 2").unwrap_err(), Error::Index);
    assert_eq!(
        Engine::new().eval("1 2 + 3 4 5").unwrap_err(),
        Error::Length
    );
    assert_eq!(Engine::new().eval("'abc' + 2").unwrap_err(), Error::Domain);
    assert!(Engine::new().eval("'unterminated").is_err());
    assert!(Engine::new().eval("< 3").is_err());
}
#[test]
fn strings_and_comments() {
    assert_eq!(
        Engine::new()
            .eval("'it''s' NB. comment")
            .unwrap()
            .unwrap()
            .display(),
        "it's"
    );
    assert!(eval("'a'").contains("\"shape\":[]"));
    assert!(Engine::new().eval("NB. only comment").unwrap().is_none());
}
#[test]
fn generated_shape_and_reshape_invariants() {
    for rows in 0..8 {
        for cols in 0..8 {
            let mut e = Engine::new();
            let v = e.eval(&format!("i. {rows} {cols}")).unwrap().unwrap();
            assert_eq!(v.len(), rows * cols);
            assert_eq!(v.shape(), &[rows, cols]);
            let flat = e.eval(&format!(", i. {rows} {cols}")).unwrap().unwrap();
            assert_eq!(flat.shape(), &[rows * cols]);
        }
    }
}

#[test]
fn shape_overflow_and_nesting_have_bounded_failures() {
    let mut e = Engine::new();
    assert_eq!(
        e.eval("i. 9223372036854775807 3").unwrap_err(),
        Error::Limit
    );
    let sentence = format!("{}1{}", "(".repeat(150), ")".repeat(150));
    assert_eq!(e.eval(&sentence).unwrap_err(), Error::Limit);
}

#[test]
fn malformed_input_smoke_fuzz_does_not_panic() {
    let alphabet = b"012_+-*%(),\"/' abc=:$#";
    let mut seed = 20260926u64;
    for _ in 0..1000 {
        let mut source = String::new();
        for _ in 0..20 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            source.push(alphabet[(seed >> 32) as usize % alphabet.len()] as char);
        }
        // Avoid reshape requests with random potentially enormous dimensions.
        source = source.replace('$', "+");
        let _ = Engine::new().eval(&source);
    }
}

//! Audit the unsupported boundary; these are not successful definition tests.
use rustj::{Engine, Error, semantic, syntax};

#[test]
fn definition_forms_are_rejected_without_binding_or_running_the_body() {
    let forms = [
        "f=:{{ {{y+1}} y }}",
        "f=:{{)n literal data }}",
        "f=:{{ if. y do. 1 else. 0 end. }}",
        "f=:3 : 0\nif. y do.\nleaked=:99\nend.\n)",
    ];
    let mut e = Engine::new();
    e.eval("f=:42").unwrap();
    let version = e.binding_version("f");
    for source in forms {
        assert!(
            matches!(semantic::parse(source), Err(Error::Unsupported(_))),
            "{source}"
        );
        assert!(e.eval(source).is_err(), "{source}");
        assert_eq!(e.binding_version("f"), version);
        assert_eq!(e.eval("f").unwrap().unwrap().int_at(0).unwrap(), 42);
        assert!(e.binding_version("leaked").is_none());
    }
}

#[test]
fn input_guard_distinguishes_words_from_quoted_and_commented_delimiters() {
    for source in [
        "f=:{{",
        "f=:3 : 0",
        "f=:verb define",
        "}}",
        "f=:{{ 'unfinished",
        "f=:3 : 'unfinished",
    ] {
        assert!(syntax::has_definition_syntax(source), "{source}");
    }
    for source in [
        "'{{ }} : define'",
        "NB. {{ : define",
        "'it''s {{'",
        "{{.",
        "3:",
        "a=:1",
    ] {
        assert!(!syntax::has_definition_syntax(source), "{source}");
    }
    assert!(syntax::lex("'unfinished").is_err());
}

//! Positive acceptance contract for definition implementation.
//! Each ignored test is an UNIMPLEMENTED milestone, never a passing capability.
//! Run explicitly with: cargo test --test definition_acceptance -- --include-ignored
use rustj::{Engine, Value, semantic};
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn define(engine: &mut Engine, source: &str) {
    assert!(
        engine
            .eval(source)
            .unwrap_or_else(|e| panic!("definition failed: {source:?}: {e}"))
            .is_none()
    );
}
fn value(engine: &mut Engine, source: &str) -> Value {
    engine
        .eval(source)
        .unwrap_or_else(|e| panic!("call failed: {source:?}: {e}"))
        .expect("noun result")
}
fn expect(engine: &mut Engine, call: &str, expected: &str) {
    let actual = value(engine, call);
    let baseline = value(&mut Engine::new(), expected);
    assert_eq!(actual.json(), baseline.json(), "{call}");
}
fn session(source: &str, reference: bool) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rustj"));
    command.arg("--json");
    if reference {
        command.arg("--semantic-reference");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
#[ignore = "DEF-1/2: code construction is implemented; complete A3 callable projection remains pending"]
fn parsing_complete_definitions_preserves_source_and_binding_boundary() {
    for source in ["f=:{{ y+1 }}", "f=:3 : 'y+1'", "f=:3 : 0\nt=.y+1\nt\n)"] {
        let parsed = semantic::parse(source).unwrap();
        assert_eq!(parsed.source, source);
        assert_eq!(parsed.assignment.as_deref(), Some("f"));
        let expr = parsed.expression.expect("definition expression");
        assert!(expr.span.start >= 3 && expr.span.end <= source.len());
        assert!(source.get(expr.span).is_some());
        let e = Engine::new();
        let plan = e.analyze_a3(source).unwrap();
        assert!(plan.result.is_some());
        assert!(e.binding_version("f").is_none());
    }
}

#[test]
#[ignore = "DEF-1/4: definition input collection and invocation are not implemented"]
fn cli_collects_a_definition_before_executing_any_body_line() {
    for source in [
        "f=:{{\ny+1\n}}\nf 41\n",
        "f=:3 : 0\ny+1\n)\nf 41\n",
        "f=:3 : 0\r\ny+1\r\n)\r\nf 41\r\n",
    ] {
        for reference in [false, true] {
            let output = session(source, reference);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8(output.stdout)
                    .unwrap()
                    .lines()
                    .collect::<Vec<_>>(),
                [
                    "{\"silent\":true}",
                    "{\"type\":4,\"shape\":[],\"data\":[42]}"
                ]
            );
        }
    }
}

#[test]
#[ignore = "DEF-2/4: direct verbs are not implemented"]
fn direct_monad_and_dyad_have_separate_parameter_frames() {
    let mut e = Engine::new();
    define(&mut e, "inc=:{{y+1}}");
    define(&mut e, "sum=:{{x+y}}");
    expect(&mut e, "inc 41", "42");
    expect(&mut e, "2 sum 3", "5");
    assert!(
        e.eval("sum 3").is_err(),
        "missing x must not reuse another frame"
    );
}

#[test]
#[ignore = "DEF-2/4: explicit verbs are not implemented"]
fn explicit_string_and_multiline_bodies_agree() {
    let mut e = Engine::new();
    define(&mut e, "inline=:3 : 'y+1'");
    define(&mut e, "block=:3 : 0\ny+1\n)");
    define(&mut e, "pair=:4 : 'x+y'");
    expect(&mut e, "inline 41", "42");
    expect(&mut e, "block 41", "42");
    expect(&mut e, "2 pair 3", "5");
}

#[test]
#[ignore = "DEF-2/4: explicit valence sections are not implemented"]
fn explicit_colon_line_selects_monad_or_dyad_section() {
    let mut e = Engine::new();
    define(&mut e, "f=:3 : 0\ny+1\n:\nx+y\n)");
    expect(&mut e, "f 2", "3");
    expect(&mut e, "4 f 5", "9");
}

#[test]
#[ignore = "DEF-1/2/4: quoted delimiters and comments in definition bodies are not implemented"]
fn comments_and_literals_do_not_end_the_definition() {
    let mut e = Engine::new();
    define(&mut e, "f=:{{\nNB. }} is a comment\ny+1\n}}");
    expect(&mut e, "f 4", "5");
    define(&mut e, "text=:3 : 0\n'it''s {{ }} : )'\n)");
    expect(&mut e, "text 0", "'it''s {{ }} : )'");
}

#[test]
#[ignore = "DEF-2/3/4: nested definition scopes are not implemented"]
fn nested_direct_verb_does_not_leak_a_local_function() {
    let mut e = Engine::new();
    define(&mut e, "outer=:{{\ninner=.{{y+1}}\ninner y\n}}");
    expect(&mut e, "outer 4", "5");
    assert!(e.binding_version("inner").is_none());
    assert!(e.eval("inner 4").is_err());
}

#[test]
#[ignore = "DEF-3/4: local frames and local assignment are not implemented"]
fn local_shadowing_is_per_call_and_does_not_modify_globals() {
    let mut e = Engine::new();
    define(&mut e, "g=:10");
    define(&mut e, "f=:3 : 0\ng=.y+1\ng\n)");
    expect(&mut e, "f 2", "3");
    expect(&mut e, "f 8", "9");
    expect(&mut e, "g", "10");
    assert!(e.binding_version("y").is_none());
}

#[test]
#[ignore = "DEF-3/4: definition-time parsing must not snapshot future body reads"]
fn body_globals_are_resolved_when_called() {
    let mut e = Engine::new();
    define(&mut e, "g=:10");
    define(&mut e, "f=:3 : 'g+y'");
    define(&mut e, "g=:20");
    expect(&mut e, "f 2", "22");
    define(&mut e, "op=:+");
    define(&mut e, "apply=:3 : 'op y'");
    define(&mut e, "op=:-");
    expect(&mut e, "apply 3", "_3");
}

#[test]
#[ignore = "DEF-3/4: local nouns must snapshot values and preserve array aliases"]
fn local_noun_copy_survives_reassignment_without_mutating_argument() {
    let mut e = Engine::new();
    define(&mut e, "source=:i.4");
    define(&mut e, "f=:3 : 0\nn=.y\ncopy=.n\nn=.n+10\ncopy\n)");
    expect(&mut e, "f source", "i.4");
    expect(&mut e, "source", "i.4");
    assert!(e.binding_version("copy").is_none());
}

#[test]
#[ignore = "DEF-3/4: global assignment in a body is not implemented"]
fn definition_construction_does_not_run_global_side_effects() {
    let mut e = Engine::new();
    define(&mut e, "counter=:0");
    define(&mut e, "f=:3 : 0\ncounter=:counter+y\ncounter\n)");
    expect(&mut e, "counter", "0");
    expect(&mut e, "f 2", "2");
    expect(&mut e, "counter", "2");
}

#[test]
#[ignore = "DEF-2/4: failed definition compilation must preserve an existing callable"]
fn invalid_redefinition_keeps_old_function_and_version() {
    let mut e = Engine::new();
    define(&mut e, "f=:{{y+1}}");
    let version = e.binding_version("f");
    for source in ["f=:{{y+1", "f=:3 : 0\ny+1", "f=:3 : 0\nif. y do.\n1\n)"] {
        assert!(e.eval(source).is_err());
        assert_eq!(e.binding_version("f"), version);
        expect(&mut e, "f 4", "5");
    }
}

#[test]
#[ignore = "DEF-4: control-flow statements and early returns are not implemented"]
fn if_else_and_return_preserve_branch_execution() {
    let mut e = Engine::new();
    define(
        &mut e,
        "f=:3 : 0\nif. y>0 do.\n42 return.\nelse.\n7 return.\nend.\n1 2+1 2 3\n)",
    );
    expect(&mut e, "f 1", "42");
    expect(&mut e, "f _1", "7");
}

#[test]
#[ignore = "DEF-4: loop control and local loop bindings are not implemented"]
fn for_loop_accumulates_without_leaking_loop_names() {
    let mut e = Engine::new();
    define(
        &mut e,
        "sum=:3 : 0\ns=.0\nfor_i. i.y do.\ns=.s+i\nend.\ns\n)",
    );
    expect(&mut e, "sum 4", "6");
    assert!(e.binding_version("i").is_none());
    assert!(e.binding_version("s").is_none());
}

#[test]
#[ignore = "DEF-4: try/catch statement semantics are not implemented"]
fn try_catch_handles_body_error() {
    let mut e = Engine::new();
    define(&mut e, "f=:3 : 0\ntry.\n1 2+1 2 3\ncatch.\n42\nend.\n)");
    expect(&mut e, "f 0", "42");
}

#[test]
#[ignore = "DEF-5: recursive function calls and independent frames are not implemented"]
fn recursive_calls_keep_the_callers_argument() {
    let mut e = Engine::new();
    define(
        &mut e,
        "fact=:3 : 0\nif. y=0 do.\n1 return.\nend.\ny*fact y-1\n)",
    );
    expect(&mut e, "fact 5", "120");
    expect(&mut e, "fact 3", "6");
}

#[test]
#[ignore = "DEF-5: both execution paths must use the same definition semantics"]
fn evaluator_paths_agree_on_function_calls_and_global_rebinding() {
    let mut direct = Engine::new();
    let mut reference = Engine::new();
    for source in ["g=:10", "f=:3 : 'g+y'", "f 2", "g=:20", "f 2", "f 1 2 3"] {
        let a = direct.eval(source).unwrap().map(|v| v.json());
        let b = reference
            .eval_semantic_reference(source)
            .unwrap()
            .map(|v| v.json());
        assert_eq!(a, b, "{source}");
    }
}

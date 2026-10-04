use rustj::Engine;
fn scalar(engine: &mut Engine, source: &str, expected: i64) {
    assert_eq!(
        engine.eval(source).unwrap().unwrap().int_at(0).unwrap(),
        expected,
        "{source}"
    );
}
#[test]
fn implicit_calls_use_caller_locals_while_ordinary_operands_use_current_locals() {
    let mut engine = Engine::new();
    engine.eval("fn=:+").unwrap();
    engine.eval("inner=:1 : 'u. 7'").unwrap();
    engine.eval("outer=:1 : 0\nfn=.-\nfn inner\n)").unwrap();
    scalar(&mut engine, "+outer", -7);
    engine.eval("inner=:1 : 'u 7'").unwrap();
    scalar(&mut engine, "+outer", 7);
    scalar(&mut engine, "fn 7", 7);
    engine.eval("inner=:1 : 0\nu=.-\nu. 7\n)").unwrap();
    scalar(&mut engine, "+inner", -7);
}
#[test]
fn implicit_operator_calls_restore_frames_after_success_and_errors() {
    let mut engine = Engine::new();
    for s in [
        "a=:1 : 'u. y'",
        "f=:-a",
        "c=:2 : 'u. y+v. y'",
        "g=:+c -",
        "d=:1 : 'x u. y'",
        "dyad=:+d",
    ] {
        engine.eval(s).unwrap();
    }
    scalar(&mut engine, "f 7", -7);
    scalar(&mut engine, "g 7", 0);
    scalar(&mut engine, "3 dyad 7", 10);
    assert_eq!(engine.eval("3 f 7").unwrap_err().kind(), "valence error");
    engine.eval("bad=:3 a").unwrap();
    assert_eq!(engine.eval("bad 7").unwrap_err().kind(), "domain error");
    engine.eval("missing=:1 : 'v. y'").unwrap();
    engine.eval("bad=:+missing").unwrap();
    assert_eq!(engine.eval("bad 7").unwrap_err().kind(), "value error");
    engine.eval("count=:0").unwrap();
    engine
        .eval("inner=:1 : 0\ncount=:count+1\nu. 1 2\n)")
        .unwrap();
    engine.eval("failop=:1 : 'u y+1 2 3'").unwrap();
    engine
        .eval("outer=:1 : 0\nfn=:+failop\nfn inner\n)")
        .unwrap();
    assert_eq!(engine.eval("+outer").unwrap_err().kind(), "length error");
    scalar(&mut engine, "count", 1);
    scalar(&mut engine, "g 7", 0);
    assert_eq!(engine.eval("u. 7").unwrap_err().kind(), "value error");
    assert_eq!(engine.eval("v. 7").unwrap_err().kind(), "value error");
}

use rustj::Engine;
fn scalar(engine: &mut Engine, source: &str, expected: i64) {
    assert_eq!(
        engine.eval(source).unwrap().unwrap().int_at(0).unwrap(),
        expected,
        "{source}"
    );
}
#[test]
fn reduction_and_rank_call_implicit_operands_per_cell_in_caller_scope() {
    let mut engine = Engine::new();
    engine.eval("a=:1 : 'u./ y'").unwrap();
    engine.eval("f=:-a").unwrap();
    scalar(&mut engine, "f 1 2 3 4", -2);
    scalar(&mut engine, "f 7", 7);
    engine.eval("a=:1 : 'u.\"0 y'").unwrap();
    engine.eval("f=:,a").unwrap();
    let result = engine.eval("f i.2 3").unwrap().unwrap();
    assert_eq!(result.shape(), &[2, 3, 1]);
    engine.eval("inner=:1 : 'u./ 1 2 3'").unwrap();
    engine.eval("outer=:1 : 0\nfn=.-\nfn inner\n)").unwrap();
    scalar(&mut engine, "+outer", 2);
    engine.eval("inner=:1 : '(u.\"0) 1 2 3'").unwrap();
    let value = engine.eval("+outer").unwrap().unwrap();
    assert_eq!(value.int_at(2).unwrap(), -3);
}
#[test]
fn hook_fork_and_atop_keep_branch_order_and_restore_operand_scope() {
    let mut engine = Engine::new();
    for (body, expected) in [("(u. + u.) y", -14), ("(u. u.) y", 14), ("(u. @: u.) y", 7)] {
        engine.eval(&format!("a=:1 : '{body}'")).unwrap();
        engine.eval("f=:-a").unwrap();
        scalar(&mut engine, "f 7", expected);
    }
    engine.eval("count=:0").unwrap();
    engine
        .eval("left=:1 : 0\ncount=:(count*10)+1\nu y\n)")
        .unwrap();
    engine
        .eval("right=:1 : 0\ncount=:(count*10)+2\nu y\n)")
        .unwrap();
    engine.eval("l=:+left").unwrap();
    engine.eval("r=:-right").unwrap();
    scalar(&mut engine, "(l + r) 7", 0);
    scalar(&mut engine, "count", 21);
    engine.eval("count=:0").unwrap();
    engine
        .eval("bad=:1 : 0\ncount=:(count*10)+3\nu y+1 2 3\n)")
        .unwrap();
    engine.eval("b=:+bad").unwrap();
    assert_eq!(
        engine.eval("(l + b) 1 2").unwrap_err().kind(),
        "length error"
    );
    scalar(&mut engine, "count", 3);
    scalar(&mut engine, "l 8", 8);
    scalar(&mut engine, "count", 31);
}

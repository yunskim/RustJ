use rustj::Engine;
#[test]
fn empty_identities_follow_current_operand_without_executing_user_bodies() {
    let mut engine = Engine::new();
    engine.eval("a=:1 : 'u./ y'").unwrap();
    for (primitive, identity) in [("+", 0), ("-", 0), ("*", 1), ("%", 1)] {
        engine.eval(&format!("f=:{primitive}a")).unwrap();
        let value = engine.eval("f i.0 3").unwrap().unwrap();
        assert_eq!(value.shape(), &[3]);
        assert_eq!(value.type_code(), 1);
        assert_eq!(value.int_at(2).unwrap(), identity);
    }
    engine.eval("fn=:+").unwrap();
    engine.eval("f=:fn a").unwrap();
    engine.eval("fn=:*").unwrap();
    assert_eq!(engine.eval("f i.0").unwrap().unwrap().int_at(0).unwrap(), 1);
    engine.eval("count=:0").unwrap();
    engine
        .eval("effect=:1 : 0\ncount=:count+1\nu y\n)")
        .unwrap();
    engine.eval("g=:+effect").unwrap();
    engine.eval("f=:g a").unwrap();
    assert_eq!(engine.eval("f i.0").unwrap_err().kind(), "unsupported");
    assert_eq!(engine.eval("count").unwrap().unwrap().int_at(0).unwrap(), 0);
    engine.eval("f=:+a").unwrap();
    assert_eq!(engine.eval("f i.0").unwrap().unwrap().int_at(0).unwrap(), 0);
}
#[test]
fn empty_ravel_rank_preserves_type_shape_and_caller_scope() {
    let mut engine = Engine::new();
    engine.eval("a=:1 : 'u.\"0 y'").unwrap();
    engine.eval("f=:,a").unwrap();
    let value = engine.eval("f i.0 3").unwrap().unwrap();
    assert_eq!(value.shape(), &[0, 3, 1]);
    assert_eq!(value.type_code(), 4);
    engine.eval("inner=:1 : '(u.\"1) i.0 3'").unwrap();
    engine.eval("fn=:+").unwrap();
    engine.eval("outer=:1 : 0\nfn=.,\nfn inner\n)").unwrap();
    let value = engine.eval("+outer").unwrap().unwrap();
    assert_eq!(value.shape(), &[0, 3]);
    engine.eval("g=:,\"0").unwrap();
    assert_eq!(engine.eval("g i.0 3").unwrap().unwrap().shape(), &[0, 3, 1]);
    engine.eval("g=:,\"_1").unwrap();
    assert_eq!(engine.eval("g i.2 0 3").unwrap().unwrap().shape(), &[2, 0]);
    engine.eval("count=:0").unwrap();
    engine
        .eval("effect=:1 : 0\ncount=:count+1\nu y\n)")
        .unwrap();
    engine.eval("g=:,effect").unwrap();
    // C executes this body once for a prototype. Until that contract is fully
    // implemented, preserve an explicit boundary rather than eliding effects.
    assert_eq!(
        engine.eval("(g\"0) i.0 3").unwrap_err().kind(),
        "unsupported"
    );
    // Nested Rank must not extend the intrinsic-only fill permission to an
    // effectful derived verb, even if the frame has zero ordinary cells.
    assert_eq!(
        engine.eval("((g\"0)\"1) i.0 3").unwrap_err().kind(),
        "unsupported"
    );
    assert_eq!(engine.eval("count").unwrap().unwrap().int_at(0).unwrap(), 0);
}

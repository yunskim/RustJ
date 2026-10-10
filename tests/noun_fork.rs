use rustj::{
    Data, Engine,
    parser_capture::CaptureEvent,
    semantic::{FunctionHead, FunctionOperand},
};
#[test]
fn noun_left_fork_snapshots_share_large_storage_after_rebinding() {
    let mut engine = Engine::new();
    engine.eval("noun=:i.65536").unwrap();
    let original = engine.eval("noun").unwrap().unwrap();
    let Data::Int(data) = original.data() else {
        panic!()
    };
    let pointer = data.as_slice().as_ptr();
    let report = engine.eval_captured("f=:(noun + -)");
    report.result.unwrap();
    report.capture.verify().unwrap();
    let function = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::FunctionResult { function, .. }
                if matches!(function.head, FunctionHead::Fork) =>
            {
                Some(function)
            }
            _ => None,
        })
        .unwrap();
    let FunctionOperand::Noun { value, .. } = &function.operands[0] else {
        panic!()
    };
    let Data::Int(stored) = value.data() else {
        panic!()
    };
    assert_eq!(pointer, stored.as_slice().as_ptr());
    engine.eval("noun=:9").unwrap();
    let result = engine.eval("f 0").unwrap().unwrap();
    assert_eq!(result.shape(), &[65536]);
    assert_eq!(result.int_at(65535).unwrap(), 65535);
    let Data::Int(stored) = value.data() else {
        panic!()
    };
    assert_eq!(pointer, stored.as_slice().as_ptr());
}
#[test]
fn noun_left_fork_calls_right_then_join_with_implicit_scope_and_errors() {
    let mut engine = Engine::new();
    assert_eq!(
        engine
            .eval("(3 + -) 7")
            .unwrap()
            .unwrap()
            .int_at(0)
            .unwrap(),
        -4
    );
    assert_eq!(
        engine
            .eval("3 (3 + +) 7")
            .unwrap()
            .unwrap()
            .int_at(0)
            .unwrap(),
        13
    );
    engine.eval("a=:1 : '(3 + u.) y'").unwrap();
    engine.eval("f=:-a").unwrap();
    assert_eq!(engine.eval("f 7").unwrap().unwrap().int_at(0).unwrap(), -4);
    engine.eval("count=:0").unwrap();
    engine.eval("op=:1 : 0\ncount=:count+1\nu y\n)").unwrap();
    engine.eval("right=:-op").unwrap();
    engine.eval("f=:(1 2 3 + right)").unwrap();
    assert_eq!(engine.eval("f 1 2").unwrap_err().kind(), "length error");
    assert_eq!(engine.eval("count").unwrap().unwrap().int_at(0).unwrap(), 1);
    assert_eq!(
        engine.eval("right 7").unwrap().unwrap().int_at(0).unwrap(),
        -7
    );
}

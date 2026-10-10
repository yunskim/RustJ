use rustj::{Engine, Value};

fn scalar(e: &mut Engine, source: &str, expected: i64) {
    let value = e.eval(source).unwrap().unwrap();
    assert!(value.shape().is_empty(), "{source}");
    assert_eq!(value.int_at(0).unwrap(), expected, "{source}");
}

#[test]
fn leading_items_scalar_and_zero_atom_rows_determine_iteration_count() {
    let mut e = Engine::new();
    for definition in [
        "f=:3 : 's=.0 for_i. y do. s=.s+1 end. s'",
        "f=:3 : 's=.0 for. y do. s=.s+1 end. s'",
    ] {
        e.eval(definition).unwrap();
        for (source, expected) in [("f 7", 1), ("f i.4", 4), ("f 2 0$0", 2), ("f 0 3$0", 0)] {
            scalar(&mut e, source, expected);
        }
    }
    e.eval("f=:3 : 's=.0 0 0 for_i. y do. s=.s+i end. s'")
        .unwrap();
    assert_eq!(
        e.eval("f i.2 3").unwrap().unwrap().json(),
        Value::ints(vec![3], vec![3, 5, 7]).unwrap().json()
    );
}

#[test]
fn index_is_readonly_during_iteration_and_mutable_after_unwinding() {
    let mut e = Engine::new();
    e.eval("i=:99").unwrap();
    e.eval("i_index=:88").unwrap();
    e.eval("bad=:3 : 'for_i. y do. i_index=.7 end.'").unwrap();
    assert_eq!(e.eval("bad i.3").unwrap_err().kind(), "read-only data");
    e.eval("f=:3 : 'try. for_i. y do. 1 2+1 2 3 end. catch. i_index=.7 end. i_index'")
        .unwrap();
    scalar(&mut e, "f i.3", 7);
    e.eval("f=:3 : 's=.0 for_i. y do. try. i_index=.7 catch. s=.s+1 end. end. s'")
        .unwrap();
    scalar(&mut e, "f i.3", 3);
    scalar(&mut e, "i", 99);
    scalar(&mut e, "i_index", 88);
    // cx.c forinit's BZ leaves the definition instead of entering catch.
    e.eval("f=:3 : 'for_i. i.2 do. try. for_i. i.2 do. end. catch. 42 end. end.'")
        .unwrap();
    assert_eq!(e.eval("f 0").unwrap_err().kind(), "read-only data");
}

#[test]
fn loop_completion_break_continue_and_reentry_preserve_name_state() {
    let mut e = Engine::new();
    e.eval("f=:3 : 'for_i. y do. end. i_index'").unwrap();
    scalar(&mut e, "f i.4", 4);
    scalar(&mut e, "f i.0", 0);
    e.eval("f=:3 : 'for_i. y do. end. i'").unwrap();
    let empty = e.eval("f i.4").unwrap().unwrap();
    assert_eq!(empty.type_code(), 1);
    assert_eq!(empty.shape(), &[0]);
    e.eval("f=:3 : 'for_i. y do. if. i_index=1 do. break. end. end. i'")
        .unwrap();
    assert_eq!(
        e.eval("f i.3 2").unwrap().unwrap().json(),
        Value::ints(vec![2], vec![2, 3]).unwrap().json()
    );
    e.eval("f=:3 : 's=.0 for_i. y do. if. i=2 do. continue. end. s=.s+i end. s'")
        .unwrap();
    scalar(&mut e, "f i.5", 8);
    e.eval("f=:3 : 's=.0 for_i. y do. break. end. for_i. y do. s=.s+i end. s'")
        .unwrap();
    scalar(&mut e, "f i.4", 6);
    e.eval("f=:3 : 's=.0 for_i. i.2 do. for_j. i.3 do. s=.s+i+j end. end. s'")
        .unwrap();
    scalar(&mut e, "f 0", 9);
}

#[test]
fn iterator_item_and_index_snapshots_survive_reassignment_and_advancement() {
    let mut e = Engine::new();
    e.eval("f=:3 : 0\ns=.0\na=.y\nfor_i. a do.\na=.99\ns=.s+i\ni=.88\nend.\ns\n)")
        .unwrap();
    scalar(&mut e, "f i.4", 6);
    e.eval("f=:3 : 'a=.99 for_i. y do. if. i_index=0 do. a=.i end. end. a'")
        .unwrap();
    assert_eq!(
        e.eval("f i.2 3").unwrap().unwrap().json(),
        Value::ints(vec![3], vec![0, 1, 2]).unwrap().json()
    );
    e.eval("f=:3 : 'a=.99 for_i. i.2 do. if. i_index=0 do. a=.i_index end. end. a'")
        .unwrap();
    scalar(&mut e, "f 0", 0);
}

#[test]
fn both_evaluators_restore_frames_after_early_return_and_error() {
    let mut a = Engine::new();
    let mut b = Engine::new();
    for source in [
        "f=:3 : 'for_i. y do. i return. end.'",
        "f i.3 2",
        "f i.0",
        "f=:3 : 'for_i. y do. 1 2+1 2 3 end.'",
        "f i.3",
        "i_index+0",
        "f=:3 : 's=.0 for_i. y do. s=.s+i end. s'",
        "f i.4",
    ] {
        let normalize = |result: rustj::Result<Option<Value>>| match result {
            Ok(Some(value)) => value.json(),
            Ok(None) => "silent".into(),
            Err(error) => error.kind().into(),
        };
        assert_eq!(
            normalize(a.eval(source)),
            normalize(b.eval_semantic_reference(source)),
            "{source}"
        );
    }
    assert!(a.binding_version("i").is_none());
    assert!(a.binding_version("i_index").is_none());
}

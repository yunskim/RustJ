use rustj::{
    Engine, Value,
    enqueuer::{self, EnqueuedPayload},
    semantic::ExprKind,
};

fn noun(source: &str) -> Value {
    let mut words = enqueuer::enqueue(source).unwrap();
    assert_eq!(words.len(), 1);
    assert_eq!(words[0].span, 0..source.len());
    match words.remove(0).payload {
        EnqueuedPayload::Scalar(value) => value.into_value().unwrap(),
        EnqueuedPayload::Noun(value) => *value,
        other => panic!("{other:?}"),
    }
}

#[test]
fn scientific_int_narrowing_is_exact_and_signed_range_checked() {
    for (source, expected) in [
        ("1e0", 1),
        ("0e0", 0),
        ("_0e0", 0),
        ("1e_9999", 0),
        ("_1e_9999", 0),
        ("10e_1", 1),
        ("_9223372036854775808e0", i64::MIN),
        ("_9223372036854775809e0", i64::MIN),
        ("9223372036854774784e0", 9_223_372_036_854_774_784),
    ] {
        let value = noun(source);
        assert_eq!(value.type_code(), 4, "{source}");
        assert_eq!(value.int_at(0).unwrap(), expected);
    }
    for source in [
        "1e_1",
        "100000000000001e_14",
        "9223372036854775807e0",
        "9223372036854775808e0",
        "_9223372036854777856e0",
        "1e9999",
    ] {
        assert_eq!(noun(source).type_code(), 8, "{source}");
    }
}

#[test]
fn dot_and_uppercase_exponent_preserve_c_narrowing_masks() {
    for source in [
        "1E0", "0E0", "_0E0", "1.0e0", "1e0 1.0", "1E0 1", "1e0 _. 0",
    ] {
        assert_eq!(noun(source).type_code(), 8, "{source}");
    }
    for source in ["1E0 1e0", "0 1e0", "1e0 0 1", "1e0 _0E0"] {
        assert_eq!(noun(source).type_code(), 4, "{source}");
    }
}

#[test]
fn real_words_round_all_atoms_before_int_narrowing_or_remain_float_together() {
    for source in ["9007199254740993 1e0", "1e0 9007199254740993"] {
        let value = noun(source);
        assert_eq!(value.type_code(), 4);
        let index = usize::from(source.starts_with("1e0"));
        assert_eq!(value.int_at(index).unwrap(), 9_007_199_254_740_992);
    }
    let value = noun("_9223372036854775809 1e0");
    assert_eq!(value.type_code(), 4);
    assert_eq!(value.int_at(0).unwrap(), i64::MIN);
    assert_eq!(noun("_9223372036854775809 1").type_code(), 8);
    for source in [
        "1e0 1e_1",
        "1e_1 1e0",
        "1e0 9223372036854775807",
        "1e0 _",
        "__ 1e0",
    ] {
        assert_eq!(noun(source).type_code(), 8, "{source}");
    }
}

#[test]
fn scientific_payload_survives_shared_frontend_and_logical_execution() {
    let mut engine = Engine::new();
    for source in ["1e0", "1E0", "9007199254740993 1e0", "1e0 1e_1", "1+1e0"] {
        let handoff = engine.admit_frontend_handoff(source).into_result().unwrap();
        if let ExprKind::Literal(value) = &handoff.program().expression.as_ref().unwrap().kind {
            assert_eq!(value.json(), noun(source).json());
        }
        let plan = engine.admit_logical(source).into_result().unwrap().logical;
        let logical = rustj::logical_executor::execute_closed(&plan)
            .unwrap()
            .unwrap();
        let direct = engine.eval(source).unwrap().unwrap();
        assert_eq!(logical.json(), direct.json());
        assert_eq!(
            engine.eval_captured(source).result.unwrap().unwrap().json(),
            direct.json()
        );
    }
}

#[test]
fn definitions_assignments_and_failed_words_preserve_types_and_state() {
    for semantic in [false, true] {
        let mut engine = Engine::new();
        for source in ["saved=:1e0", "f=:{{1e0}}", "g=:3 : '1e0'"] {
            engine.eval(source).unwrap();
        }
        for source in ["saved", "f 0", "g 0"] {
            let value = if semantic {
                engine.eval_semantic_reference_diagnostic(source)
            } else {
                engine.eval_diagnostic(source)
            }
            .unwrap()
            .unwrap();
            assert_eq!(value.type_code(), 4);
            assert_eq!(value.int_at(0).unwrap(), 1);
        }
        for source in ["saved=:1e0 1e_", "saved=:1e0 1x"] {
            assert_eq!(
                engine.eval_diagnostic(source).unwrap_err().kind(),
                "ill-formed number"
            );
            assert_eq!(engine.eval("saved").unwrap().unwrap().int_at(0).unwrap(), 1);
        }
    }
    assert_eq!(
        enqueuer::enqueue("1r2 1e0").unwrap_err().kind(),
        "unsupported"
    );
}

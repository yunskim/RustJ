use rustj::{
    Engine, Value,
    enqueuer::{self, EnqueuedPayload},
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
fn real_ratios_follow_word_mode_and_exact_narrowing_masks() {
    for (source, expected) in [
        ("1r2.0", 0.5),
        ("1.r2", 0.5),
        ("1r2.", 0.5),
        ("_1r2.0", -0.5),
        ("1r_2.0", -0.5),
        ("_1r_2.0", 0.5),
        ("1e1r4", 2.5),
    ] {
        let value = noun(source);
        assert_eq!(value.type_code(), 8, "{source}");
        assert_eq!(value.float_at(0).unwrap(), expected);
    }
    for source in ["2r1 1e0", "2e0r1", "0r0 1e0", "2r1 1E0 1e0"] {
        assert_eq!(noun(source).type_code(), 4, "{source}");
    }
    for source in [
        "2r1 1.0",
        "1r2 1e0",
        "2r1 1e0 0.0",
        "1r0 1e0",
        "9223372036854775808r1 1e0",
    ] {
        assert_eq!(noun(source).type_code(), 8, "{source}");
    }
    assert_eq!(
        noun("9007199254740993r1 1e0").int_at(0).unwrap(),
        9_007_199_254_740_992
    );
}

#[test]
fn zero_denominators_preserve_j_zero_and_infinity_signs() {
    for (source, expected) in [
        ("0r0.0", 0.0_f64),
        ("_0r0.0", -0.0),
        ("0r_0.0", -0.0),
        ("_0r_0.0", 0.0),
        ("1r0.0", f64::INFINITY),
        ("1r_0.0", f64::NEG_INFINITY),
        ("_1r0.0", f64::NEG_INFINITY),
        ("_1r_0.0", f64::INFINITY),
        ("1e_9999r_0.0", -0.0),
    ] {
        assert_eq!(
            noun(source).float_at(0).unwrap().to_bits(),
            expected.to_bits(),
            "{source}"
        );
    }
    assert!(noun("1e9999r1e9999").float_at(0).unwrap().is_nan());
    assert_eq!(noun("1r1e9999").int_at(0).unwrap(), 0);
}

#[test]
fn exact_rational_construction_and_hex_boundary_remain_distinct() {
    for source in ["1r2", "0r0", "1r2 3r4"] {
        assert_eq!(noun(source).type_code(), 128);
    }
    assert_eq!(
        enqueuer::enqueue("0X1r2 1.0").unwrap_err().kind(),
        "unsupported"
    );
    for source in ["1r2 1E0", "1r_ 1.0", "1r2r3 1.0", "1r2.0 1e_", "1r2.0 1x"] {
        assert_eq!(
            enqueuer::enqueue(source).unwrap_err().kind(),
            "ill-formed number",
            "{source}"
        );
    }
}

#[test]
fn ratios_survive_handoff_logical_execution_and_capture() {
    let mut engine = Engine::new();
    for source in [
        "1r2.0",
        "_0r0.0",
        "0r_0.0",
        "1r0.0",
        "2r1 1e0",
        "1r2 1e0",
        "1e9999r1e9999",
    ] {
        let expected = noun(source);
        let handoff = engine.admit_frontend_handoff(source).into_result().unwrap();
        assert!(handoff.program().expression.is_some());
        let plan = engine.admit_logical(source).into_result().unwrap().logical;
        let logical = rustj::logical_executor::execute_closed(&plan)
            .unwrap()
            .unwrap();
        let direct = engine.eval(source).unwrap().unwrap();
        let captured = engine.eval_captured(source).result.unwrap().unwrap();
        for value in [logical, direct, captured] {
            assert_eq!(value.json(), expected.json(), "{source}");
        }
    }
}

#[test]
fn definition_locals_globals_and_failed_assignment_keep_ratio_values() {
    for semantic in [false, true] {
        let mut engine = Engine::new();
        for source in [
            "saved=:7",
            "f=:{{local=.1r2.0\nlocal+y}}",
            "g=:3 : 'local=.2r1 1e0\nlocal+y'",
        ] {
            engine.eval(source).unwrap();
        }
        engine.eval("saved=:1r2.0").unwrap();
        for (source, dtype) in [("f 0", 8), ("g 0", 4), ("saved", 8)] {
            let result = if semantic {
                engine.eval_semantic_reference_diagnostic(source)
            } else {
                engine.eval_diagnostic(source)
            };
            assert_eq!(result.unwrap().unwrap().type_code(), dtype);
        }
        assert_eq!(
            engine.eval_diagnostic("local").unwrap_err().kind(),
            "value error"
        );
        for source in ["saved=:1r2.0 1e_", "saved=:1r2r3 1.0"] {
            assert_eq!(
                engine.eval_diagnostic(source).unwrap_err().kind(),
                "ill-formed number"
            );
            assert_eq!(
                engine.eval("saved").unwrap().unwrap().float_at(0).unwrap(),
                0.5
            );
        }
    }
}

use rustj::{
    Engine, Value,
    enqueuer::{self, EnqueuedPayload},
    error::{DiagnosticPhase, FailureCategory},
    semantic::ExprKind,
};

fn noun(source: &str) -> Value {
    let mut words = enqueuer::enqueue(source).unwrap();
    assert_eq!(words.len(), 1);
    assert_eq!(words[0].span, 0..source.len());
    match words.remove(0).payload {
        EnqueuedPayload::Scalar(scalar) => scalar.into_value().unwrap(),
        EnqueuedPayload::Noun(value) => *value,
        other => panic!("unexpected payload {other:?}"),
    }
}

#[test]
fn signed_boundaries_keep_int_precision_and_overflow_promotes_to_float() {
    for (source, expected) in [
        ("9223372036854775807", i64::MAX),
        ("_9223372036854775808", i64::MIN),
        ("9007199254740993", 9_007_199_254_740_993),
        ("000000009007199254740993", 9_007_199_254_740_993),
    ] {
        let value = noun(source);
        assert_eq!(value.type_code(), 4, "{source}");
        assert_eq!(value.int_at(0).unwrap(), expected, "{source}");
    }
    for (source, expected) in [
        ("9223372036854775808", 9_223_372_036_854_775_808.0),
        ("_9223372036854775809", -9_223_372_036_854_775_808.0),
        ("000000009223372036854775808", 9_223_372_036_854_775_808.0),
    ] {
        let value = noun(source);
        assert_eq!(value.type_code(), 8, "{source}");
        assert_eq!(value.shape(), &[]);
        assert_eq!(value.float_at(0).unwrap(), expected, "{source}");
    }
}

#[test]
fn overflow_anywhere_reconverts_the_whole_numeric_word() {
    for source in [
        "9007199254740993 9223372036854775808 1",
        "9223372036854775808 9007199254740993 1",
        "1 9007199254740993 _9223372036854775809",
    ] {
        let value = noun(source);
        assert_eq!(value.type_code(), 8);
        assert_eq!(value.shape(), &[3]);
        for (i, field) in source.split_ascii_whitespace().enumerate() {
            let expected = field.replace('_', "-").parse::<f64>().unwrap();
            assert_eq!(value.float_at(i).unwrap().to_bits(), expected.to_bits());
        }
    }
    let exact = noun("9007199254740993 9223372036854775807");
    assert_eq!(exact.type_code(), 4);
    assert_eq!(exact.int_at(0).unwrap(), 9_007_199_254_740_993);
}

#[test]
fn long_decimal_overflow_and_signs_follow_float_conversion() {
    for negative in [false, true] {
        let source = format!("{}{}", if negative { "_" } else { "" }, "9".repeat(400));
        let value = noun(&source);
        assert_eq!(value.type_code(), 8);
        assert_eq!(
            value.float_at(0).unwrap(),
            if negative {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }
        );
    }
    assert_eq!(noun("0 1").type_code(), 1);
    assert_eq!(noun("0 2").type_code(), 4);
}

#[test]
fn malformed_suffix_is_not_lost_when_integer_scan_stops_at_overflow() {
    for source in [
        "9223372036854775808 1_2",
        "_9223372036854775809 1e_",
        "1_2 9223372036854775808",
    ] {
        let error = enqueuer::enqueue(source).unwrap_err();
        assert_eq!(error.kind(), "ill-formed number", "{source}");
        assert_eq!(error.category(), FailureCategory::JLanguage);
        let context = error.context().unwrap();
        assert_eq!(context.phase, Some(DiagnosticPhase::Enqueue));
        assert_eq!(context.span.as_ref(), Some(&(0..source.len())));
    }
    assert!(enqueuer::enqueue("1r3").is_ok());
}

#[test]
fn promoted_literal_survives_frontend_handoff_and_closed_logical_execution() {
    let mut engine = Engine::new();
    for source in [
        "9223372036854775808",
        "9007199254740993 9223372036854775808 1",
        "1+9223372036854775808",
    ] {
        let handoff = engine.admit_frontend_handoff(source).into_result().unwrap();
        assert_eq!(handoff.source_origin().text(), source);
        if let ExprKind::Literal(value) = &handoff.program().expression.as_ref().unwrap().kind {
            assert_eq!(value.type_code(), 8);
        }
        let analysis = engine.admit_logical(source).into_result().unwrap();
        let logical = rustj::logical_executor::execute_closed(&analysis.logical)
            .unwrap()
            .unwrap();
        let direct = engine.eval(source).unwrap().unwrap();
        assert_eq!(logical.json(), direct.json());
        let captured = engine.eval_captured(source);
        captured.capture.verify().unwrap();
        assert_eq!(captured.result.unwrap().unwrap().json(), direct.json());
    }
}

#[test]
fn definition_and_failed_assignment_preserve_observable_name_state() {
    for semantic in [false, true] {
        let mut engine = Engine::new();
        engine.eval("saved=:7").unwrap();
        for definition in ["f=:{{9223372036854775808}}", "f=:3 : '9223372036854775808'"] {
            engine.eval_diagnostic(definition).unwrap();
            let value = if semantic {
                engine.eval_semantic_reference_diagnostic("f 0")
            } else {
                engine.eval_diagnostic("f 0")
            }
            .unwrap()
            .unwrap();
            assert_eq!(value.type_code(), 8);
            assert_eq!(value.float_at(0).unwrap(), 9_223_372_036_854_775_808.0);
        }
        let error = engine
            .eval_diagnostic("saved=:9223372036854775808 1_2")
            .unwrap_err();
        assert_eq!(error.kind(), "ill-formed number");
        assert_eq!(engine.eval("saved").unwrap().unwrap().int_at(0).unwrap(), 7);
    }
}

#[test]
fn float_json_preserves_large_values_and_negative_zero_for_numeric_decoders() {
    for atom in [
        0.0,
        -0.0,
        1.0,
        0.1,
        9_223_372_036_854_775_808.0,
        f64::MIN_POSITIVE,
        f64::MAX,
    ] {
        let value = rustj::types::Scalar::Float(atom).into_value().unwrap();
        let json = value.json();
        let number = json
            .split_once("\"data\":[")
            .unwrap()
            .1
            .strip_suffix("]}")
            .unwrap();
        assert!(number.contains(['.', 'e', 'E']), "{json}");
        assert_eq!(number.parse::<f64>().unwrap().to_bits(), atom.to_bits());
    }
    assert!(Value::scalar(7).json().ends_with("[7]}"));
}

#[test]
fn integer_spelling_controls_bool_narrowing_for_the_whole_word() {
    for (source, dtype, atoms) in [
        ("0", 1, vec![0]),
        ("1", 1, vec![1]),
        ("_0", 1, vec![0]),
        ("00", 4, vec![0]),
        ("01", 4, vec![1]),
        ("0001", 4, vec![1]),
        ("_00", 4, vec![0]),
        ("_01", 4, vec![-1]),
        ("0 1", 1, vec![0, 1]),
        ("0 _0", 4, vec![0, 0]),
        ("_0 1", 4, vec![0, 1]),
        ("00 1", 4, vec![0, 1]),
        ("0 01", 4, vec![0, 1]),
    ] {
        let value = noun(source);
        assert_eq!(value.type_code(), dtype, "{source}");
        assert_eq!(value.len(), atoms.len());
        for (index, atom) in atoms.into_iter().enumerate() {
            assert_eq!(value.int_at(index).unwrap(), atom);
        }
        let mut engine = Engine::new();
        let handoff = engine.admit_frontend_handoff(source).into_result().unwrap();
        let ExprKind::Literal(literal) = &handoff.program().expression.as_ref().unwrap().kind
        else {
            panic!("literal");
        };
        assert_eq!(literal.type_code(), dtype);
        let logical = engine.admit_logical(source).into_result().unwrap().logical;
        let result = rustj::logical_executor::execute_closed(&logical)
            .unwrap()
            .unwrap();
        assert_eq!(result.json(), value.json());
        assert_eq!(engine.eval(source).unwrap().unwrap().json(), value.json());
    }
}

#[test]
fn integer_spelling_survives_assignment_definitions_and_boxing() {
    for semantic in [false, true] {
        let mut engine = Engine::new();
        for source in ["saved=:01", "f=:{{01}}", "g=:3 : '_00'"] {
            engine.eval(source).unwrap();
        }
        for source in ["saved", "f 0", "g 0", "> < 01"] {
            let value = if semantic {
                engine.eval_semantic_reference_diagnostic(source)
            } else {
                engine.eval_diagnostic(source)
            }
            .unwrap()
            .unwrap();
            assert_eq!(value.type_code(), 4, "{source}");
        }
    }
}

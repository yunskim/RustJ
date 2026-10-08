use rustj::{
    Engine, Value,
    enqueuer::{self, EnqueuedPayload},
    semantic::ExprKind,
};

fn numeric_noun(source: &str) -> Value {
    let mut words = enqueuer::enqueue(source).expect("valid numeric word");
    assert_eq!(words.len(), 1, "{source}");
    match words.remove(0).payload {
        EnqueuedPayload::Scalar(atom) => atom.into_value().unwrap(),
        EnqueuedPayload::Noun(noun) => *noun,
        other => panic!("unexpected numeric payload for {source}: {other:?}"),
    }
}

#[test]
fn scientific_real_narrows_to_int_only_when_exact_and_allowed_by_spelling() {
    for (source, expected) in [
        ("1e0", 1_i64),
        ("0e0", 0),
        ("_0e0", 0),
        ("_1e0", -1),
        ("1e1", 10),
        ("1e_0", 1),
        ("_9223372036854775808e0", i64::MIN),
        ("9007199254740992e0", 9_007_199_254_740_992),
    ] {
        let value = numeric_noun(source);
        assert_eq!(value.type_code(), 4, "{source}: e-spelling must not become Bool");
        assert_eq!(value.shape(), &[], "{source}");
        assert_eq!(value.int_at(0).unwrap(), expected, "{source}");
    }
}

#[test]
fn real_narrowing_is_a_whole_word_decision() {
    for (source, expected) in [
        ("1e0 0e0", vec![1, 0]),
        ("1e0 2", vec![1, 2]),
        ("0e0 1", vec![0, 1]),
        ("1e0 01", vec![1, 1]),
        ("_1e0 1e1", vec![-1, 10]),
        ("1 2e0 3e0", vec![1, 2, 3]),
    ] {
        let value = numeric_noun(source);
        assert_eq!(value.type_code(), 4, "{source}");
        assert_eq!(value.shape(), &[expected.len()], "{source}");
        for (i, atom) in expected.into_iter().enumerate() {
            assert_eq!(value.int_at(i).unwrap(), atom, "{source} index {i}");
        }
    }

    // Any decimal point forces Float for the *whole* word in wn.c::jtconnum.
    for source in [
        "1.",
        "1.0",
        "1.e0",
        "1e0 1.0",
        "1.0 2e0",
        "0e0 2.0",
    ] {
        assert_eq!(numeric_noun(source).type_code(), 8, "{source}");
    }
    // Any fractional/non-finite/out-of-range atom keeps every atom Float.
    for source in [
        "1e0 1e_1",
        "1e_1 1e0",
        "1e0 _",
        "1e309 1e0",
        "1e0 9223372036854775808e0",
    ] {
        assert_eq!(numeric_noun(source).type_code(), 8, "{source}");
    }
}

#[test]
fn real_to_int_boundary_does_not_saturate_or_round_to_wrong_machine_integer() {
    // i64::MAX rounds to 2^63 as f64 and must NOT saturate to i64::MAX.
    let positive = numeric_noun("9223372036854775807e0");
    assert_eq!(positive.type_code(), 8);
    assert_eq!(positive.float_at(0).unwrap(), 9_223_372_036_854_775_808.0);

    let overflow = numeric_noun("9223372036854775808e0");
    assert_eq!(overflow.type_code(), 8);
    let mixed = numeric_noun("1e0 9223372036854775808e0");
    assert_eq!(mixed.type_code(), 8);

    let minimum = numeric_noun("_9223372036854775808e0");
    assert_eq!(minimum.type_code(), 4);
    assert_eq!(minimum.int_at(0).unwrap(), i64::MIN);

    // The Float input is rounded first, then losslessly narrowed, not parsed
    // as an exact extended integer.
    let rounded = numeric_noun("9007199254740993e0");
    assert_eq!(rounded.type_code(), 4);
    assert_eq!(rounded.int_at(0).unwrap(), 9_007_199_254_740_992);
}

#[test]
fn scientific_dtype_survives_handoff_graph_logical_and_direct_runtime() {
    for source in [
        "1e0",
        "0e0",
        "_1e0",
        "1e0 2e0",
        "1e0 0.5",
        "9223372036854775808e0",
        "1e0 9223372036854775808e0",
    ] {
        let mut engine = Engine::new();
        let expected = numeric_noun(source);
        let handoff = engine.admit_frontend_handoff(source).into_result().unwrap();
        let ExprKind::Literal(literal) = &handoff.program().expression.as_ref().unwrap().kind
        else {
            panic!("numeric literal handoff: {source}");
        };
        assert_eq!(literal.type_code(), expected.type_code(), "{source}");

        let analysis = engine.admit_logical(source).into_result().unwrap();
        let logical = rustj::logical_executor::execute_closed(&analysis.logical)
            .unwrap()
            .unwrap();
        assert_eq!(logical.json(), expected.json(), "{source}");
        let runtime = engine.eval(source).unwrap().unwrap();
        assert_eq!(runtime.json(), expected.json(), "{source}");
        let captured = engine.eval_captured(source);
        captured.capture.verify().unwrap();
        assert_eq!(captured.result.unwrap().unwrap().json(), expected.json(), "{source}");
    }
}

#[test]
fn scientific_dtype_survives_assignment_and_explicit_definition() {
    for semantic in [false, true] {
        let mut engine = Engine::new();
        for definition in ["stored=:1e0", "f=:{{1e0}}", "g=:3 : '1e0 2e0'"] {
            engine.eval(definition).unwrap();
        }
        for (source, shape, atoms) in [
            ("stored", vec![], vec![1_i64]),
            ("f 0", vec![], vec![1]),
            ("g 0", vec![2], vec![1, 2]),
        ] {
            let value = if semantic {
                engine.eval_semantic_reference_diagnostic(source)
            } else {
                engine.eval_diagnostic(source)
            }
            .unwrap()
            .unwrap();
            assert_eq!(value.type_code(), 4, "{source}");
            assert_eq!(value.shape(), shape.as_slice(), "{source}");
            for (i, atom) in atoms.into_iter().enumerate() {
                assert_eq!(value.int_at(i).unwrap(), atom, "{source} index {i}");
            }
        }
    }
}

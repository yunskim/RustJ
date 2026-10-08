use rustj::{
    Data, Engine, Value,
    enqueuer::{self, EnqueuedPayload},
    storage::CpuStorage,
    types::{BigInt, Rational, Scalar},
};
use std::sync::Arc;
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
fn atoms(value: &Value) -> Vec<(String, String)> {
    let Data::Rational(v) = value.data() else {
        panic!("expected rational: {value:?}")
    };
    v.iter()
        .map(|x| (x.numerator().to_string(), x.denominator().to_string()))
        .collect()
}
#[test]
fn canonical_rational_values_cover_reduction_signs_zero_and_infinity() {
    for (n, d, expected) in [
        (2, 4, (1, 2)),
        (2, -4, (-1, 2)),
        (-2, -4, (1, 2)),
        (0, 0, (0, 1)),
        (0, -4, (0, 1)),
        (7, 0, (1, 0)),
        (-7, 0, (-1, 0)),
    ] {
        let r = Rational::new(n.into(), d.into()).unwrap();
        assert_eq!(r.numerator(), &BigInt::from(expected.0));
        assert_eq!(r.denominator(), &BigInt::from(expected.1));
        assert_eq!(r.is_finite(), expected.1 != 0);
    }
}
#[test]
fn rational_words_preserve_exact_payloads_and_whole_word_type() {
    for (source, n, d) in [
        ("2r4", "1", "2"),
        ("2r1", "2", "1"),
        ("0r0", "0", "1"),
        ("1r_0", "1", "0"),
        ("_1r_0", "-1", "0"),
        ("1r_", "0", "1"),
        ("_2r__", "0", "1"),
        ("_r", "1", "0"),
        ("__r", "-1", "0"),
        ("_r_0", "-1", "0"),
        ("__r_3", "1", "0"),
        ("9007199254740993r2", "9007199254740993", "2"),
    ] {
        let value = noun(source);
        assert_eq!(value.type_code(), 128, "{source}");
        assert_eq!(atoms(&value), [(n.into(), d.into())], "{source}");
    }
    let value = noun("1x 2r3 9007199254740993 _");
    assert_eq!(value.shape(), &[4]);
    assert_eq!(value.type_code(), 128);
    assert_eq!(
        atoms(&value),
        [
            ("1".into(), "1".into()),
            ("2".into(), "3".into()),
            ("9007199254740993".into(), "1".into()),
            ("1".into(), "0".into())
        ]
    );
    assert!(value.json().contains("\"numerator\":\"9007199254740993\""));
}
#[test]
fn rational_mode_does_not_capture_real_complex_or_malformed_words() {
    for source in ["1r2.0", "1r2 1e0"] {
        assert_eq!(noun(source).type_code(), 8);
    }
    for source in ["1j2 1r2", "1xr2"] {
        assert_eq!(enqueuer::enqueue(source).unwrap_err().kind(), "unsupported");
    }
    for source in ["2r", "2rr3", "2r3x", "2r3 1E0", "_r_", "1r2 1q"] {
        assert_eq!(
            enqueuer::enqueue(source).unwrap_err().kind(),
            "ill-formed number",
            "{source}"
        );
    }
}
#[test]
fn rational_storage_shares_atoms_preserves_overlap_and_releases_owners() {
    let scalar = Scalar::Rational(Arc::new(Rational::new(1.into(), 2.into()).unwrap()))
        .into_value()
        .unwrap();
    assert!(matches!(
        scalar.data(),
        Data::Rational(CpuStorage::Inline(_))
    ));
    let value = noun("9007199254740993r2 3r4");
    assert!(matches!(value.data(), Data::Rational(CpuStorage::Owned(_))));
    let value = value.into_shared();
    let copy = value.clone();
    let Data::Rational(CpuStorage::Shared(backing)) = value.data() else {
        panic!()
    };
    let Data::Rational(CpuStorage::Shared(other)) = copy.data() else {
        panic!()
    };
    assert!(Arc::ptr_eq(backing, other));
    let weak = Arc::downgrade(&backing[0]);
    let selected = value.select([3], [0, 0, 1]).unwrap();
    let Data::Rational(v) = selected.data() else {
        panic!()
    };
    assert!(Arc::ptr_eq(&backing[0], &v[0]));
    assert!(Arc::ptr_eq(&v[0], &v[1]));
    let cell = value.view().cell(0, 0).unwrap().to_owned().unwrap();
    let Data::Rational(v) = cell.data() else {
        panic!()
    };
    assert!(Arc::ptr_eq(&backing[0], &v[0]));
    assert_eq!(value.select([1], [99]).unwrap_err().kind(), "index error");
    assert_eq!(atoms(&value)[0], ("9007199254740993".into(), "2".into()));
    drop(value);
    drop(copy);
    drop(selected);
    drop(cell);
    assert!(weak.upgrade().is_none());
}
#[test]
fn rational_structural_verbs_preserve_type_and_zero_fills() {
    let mut engine = Engine::new();
    for source in [
        "+2r3",
        ",2r3",
        "|.2r3 3r4",
        "|:2 2$2r3 3r4",
        "1{2r3 3r4",
        "4{.2r3 3r4",
        "1}.2r3 3r4",
        "1|.2r3 3r4",
        "0$2r3",
        "2 0$2r3",
        "> <2r3",
    ] {
        assert_eq!(
            engine.eval(source).unwrap().unwrap().type_code(),
            128,
            "{source}"
        );
    }
    for source in ["$2r3", "#2r3", "$2 2$2r3"] {
        assert_eq!(
            engine.eval(source).unwrap().unwrap().type_code(),
            64,
            "{source}"
        );
    }
    assert_eq!(
        atoms(&engine.eval("4{.2r3").unwrap().unwrap()),
        [
            ("2".into(), "3".into()),
            ("0".into(), "1".into()),
            ("0".into(), "1".into()),
            ("0".into(), "1".into())
        ]
    );
}
#[test]
fn rational_payload_survives_frontend_handoff_logical_execution_and_capture() {
    let mut engine = Engine::new();
    for source in [
        "2r4", "0r0", "1r0", "__r_0", "1x 2r3 _", ",2r3", "#2r3", "1r2+1r3", "1r2%2x", "*1r2",
        "1r2<2r3",
    ] {
        let handoff = engine.admit_frontend_handoff(source).into_result().unwrap();
        assert!(handoff.program().expression.is_some());
        let direct = engine.eval(source).unwrap().unwrap();
        assert_eq!(
            engine.eval_captured(source).result.unwrap().unwrap().json(),
            direct.json()
        );
        let plan = engine.admit_logical(source).into_result().unwrap().logical;
        let result = rustj::logical_executor::execute_closed(&plan)
            .unwrap()
            .unwrap();
        assert_eq!(result.json(), direct.json(), "{source}");
    }
}
#[test]
fn rational_names_definitions_and_failed_assignments_preserve_state() {
    for semantic in [false, true] {
        let mut engine = Engine::new();
        for source in [
            "saved=:9007199254740993r2",
            "alias=:saved",
            "f=:{{local=.saved\nlocal}}",
            "g=:3 : 'local=.saved\nlocal'",
        ] {
            engine.eval(source).unwrap();
        }
        for source in ["f 0", "g 0"] {
            let result = if semantic {
                engine.eval_semantic_reference_diagnostic(source)
            } else {
                engine.eval_diagnostic(source)
            };
            assert_eq!(
                atoms(&result.unwrap().unwrap()),
                [("9007199254740993".into(), "2".into())]
            );
        }
        assert_eq!(
            engine.eval_diagnostic("local").unwrap_err().kind(),
            "value error"
        );
        for source in ["saved=:1r2 2rr3", "saved=:1r2 1E0"] {
            assert_eq!(
                engine.eval_diagnostic(source).unwrap_err().kind(),
                "ill-formed number"
            );
            assert_eq!(
                atoms(&engine.eval("saved").unwrap().unwrap()),
                [("9007199254740993".into(), "2".into())]
            );
        }
        engine.eval("saved=:1r0").unwrap();
        assert_eq!(
            atoms(&engine.eval("alias").unwrap().unwrap()),
            [("9007199254740993".into(), "2".into())]
        );
    }
}
#[test]
fn rational_missing_capabilities_are_not_caught_or_coerced_to_float() {
    let mut engine = Engine::new();
    for source in ["1r2+1.0", "1r2,2r3", "+/1r2 2r3", "1r2 i.2r3", "1r1$2r3"] {
        assert_eq!(
            engine.eval_diagnostic(source).unwrap_err().kind(),
            "unsupported",
            "{source}"
        );
    }
    engine.eval("f=:{{try. 1r2+1.0 catch. 42 end.}}").unwrap();
    assert_eq!(
        engine.eval_diagnostic("f 0").unwrap_err().kind(),
        "unsupported"
    );
    let value = noun("1r2");
    assert_eq!(value.float_at(0).unwrap_err().kind(), "unsupported");
    assert_eq!(value.view().float_at(0).unwrap_err().kind(), "unsupported");
    assert!(matches!(
        rustj::physical::BufferRegistry::new()
            .unwrap()
            .register(value),
        Err(rustj::Error::Unsupported(_))
    ));
}

#[test]
fn rational_arithmetic_has_independent_exact_expectations_and_integer_promotion() {
    let mut engine = Engine::new();
    for (source, n, d) in [
        ("1r2+1r3", "5", "6"),
        ("1r2-2r3", "-1", "6"),
        ("_2r3*9r4", "-3", "2"),
        ("_2r3%9r4", "-8", "27"),
        ("9007199254740993r2+1", "9007199254740995", "2"),
        ("1+9007199254740993r2", "9007199254740995", "2"),
        ("9007199254740993x-1r2", "18014398509481985", "2"),
        ("1r2%2x", "1", "4"),
        ("2x%1r2", "4", "1"),
        ("0+1r2", "1", "2"),
        ("1r2+0", "1", "2"),
    ] {
        let value = engine.eval(source).unwrap().unwrap();
        assert_eq!(value.type_code(), 128, "{source}");
        assert_eq!(atoms(&value), [(n.into(), d.into())], "{source}");
    }
    // Exact cancellation beyond both f64 and i64 ranges.
    let digits = "9".repeat(200);
    let value = engine
        .eval(&format!("({digits}r7%{digits}r3)*7r3"))
        .unwrap()
        .unwrap();
    assert_eq!(atoms(&value), [("1".into(), "1".into())]);
}
#[test]
fn rational_non_finite_rules_distinguish_zero_infinity_and_j_nan_error() {
    let mut engine = Engine::new();
    for (source, n, d) in [
        ("0r1*1r0", "0", "1"),
        ("1r0*0r1", "0", "1"),
        ("0r1%0r1", "0", "1"),
        ("0r1%1r0", "0", "1"),
        ("1r2%0r1", "1", "0"),
        ("_1r2%0r1", "-1", "0"),
        ("1r0+2r3", "1", "0"),
        ("2r3-_1r0", "1", "0"),
        ("_1r0-1r0", "-1", "0"),
        ("_1r0*_1r0", "1", "0"),
        ("1r0%_2r3", "1", "0"),
        ("_1r0%0r1", "-1", "0"),
    ] {
        assert_eq!(
            atoms(&engine.eval(source).unwrap().unwrap()),
            [(n.into(), d.into())],
            "{source}"
        );
    }
    for source in [
        "1r0+_1r0",
        "_1r0+1r0",
        "1r0-1r0",
        "_1r0-_1r0",
        "1r0%1r0",
        "1r0%_1r0",
    ] {
        let error = engine.eval_diagnostic(source).unwrap_err();
        assert_eq!(error.kind(), "NaN error", "{source}");
        assert_eq!(error.class_name(), "NaNError");
        assert!(error.is_j_catchable());
        assert!(error.context().is_some());
    }
}
#[test]
fn rational_comparison_is_exact_and_orders_infinities() {
    let mut engine = Engine::new();
    for (source, expected) in [
        ("9007199254740993r1=9007199254740992r1", 0),
        ("9007199254740993r1>9007199254740992x", 1),
        ("9007199254740992x<9007199254740993r1", 1),
        ("2r4=1r2", 1),
        ("_1r0<1r0", 1),
        ("1r0=1r0", 1),
        ("1r0>9007199254740993x", 1),
        ("0r1=0", 1),
    ] {
        let value = engine.eval(source).unwrap().unwrap();
        assert_eq!(value.type_code(), 1, "{source}");
        assert_eq!(value.int_at(0).unwrap(), expected, "{source}");
    }
}
#[test]
fn rational_unary_operations_preserve_type_and_reuse_nonnegative_absolute_atoms() {
    let mut engine = Engine::new();
    for (source, n, d) in [
        ("-2r3", "-2", "3"),
        ("|_2r3", "2", "3"),
        ("%2r3", "3", "2"),
        ("%0r1", "1", "0"),
        ("%1r0", "0", "1"),
        ("%_1r0", "0", "1"),
    ] {
        let value = engine.eval(source).unwrap().unwrap();
        assert_eq!(value.type_code(), 128);
        assert_eq!(atoms(&value), [(n.into(), d.into())]);
    }
    let value = engine.eval("*_1r0 0r1 2r3 1r0").unwrap().unwrap();
    assert_eq!(value.type_code(), 64);
    assert_eq!(
        value.json(),
        "{\"type\":64,\"shape\":[4],\"data\":[\"-1\",\"0\",\"1\",\"1\"]}"
    );
    let source = noun("2r3");
    let result = rustj::kernels::monad("|", source.clone()).unwrap();
    let (Data::Rational(a), Data::Rational(b)) = (source.data(), result.data()) else {
        panic!()
    };
    assert!(Arc::ptr_eq(&a[0], &b[0]));
}
#[test]
fn rational_array_arithmetic_handles_prefix_agreement_empty_and_failed_assignment() {
    let mut engine = Engine::new();
    engine.eval("a=:2 2$1r2 2r3 3r4 4r5").unwrap();
    engine.eval("alias=:a").unwrap();
    let result = engine.eval("1 2+a").unwrap().unwrap();
    assert_eq!(result.shape(), &[2, 2]);
    assert_eq!(
        atoms(&result),
        [
            ("3".into(), "2".into()),
            ("5".into(), "3".into()),
            ("11".into(), "4".into()),
            ("14".into(), "5".into())
        ]
    );
    for source in ["a=:1r0 2r3+_1r0 1r2", "a=:1r2 2r3+1 2 3"] {
        assert!(engine.eval_diagnostic(source).is_err());
        assert_eq!(
            engine.eval("a").unwrap().unwrap().json(),
            engine.eval("alias").unwrap().unwrap().json()
        );
    }
    for (source, t) in [("1r2+0$1", 128), ("(0$1r2)%0r1", 128), ("(0$1r2)=1r2", 1)] {
        let value = engine.eval(source).unwrap().unwrap();
        assert_eq!(value.shape(), &[0]);
        assert_eq!(value.type_code(), t);
    }
}
#[test]
fn rational_runtime_errors_are_caught_and_definition_calls_preserve_exact_names() {
    for semantic in [false, true] {
        let mut engine = Engine::new();
        for s in [
            "saved=:9007199254740993r2",
            "f=:{{local=.saved\nlocal+1r3}}",
            "g=:3 : 'local=.saved\nlocal+1r3'",
            "caught=:{{try. saved=:1r0-1r0 catch. saved end.}}",
        ] {
            engine.eval(s).unwrap();
        }
        for s in ["f 0", "g 0"] {
            let value = if semantic {
                engine.eval_semantic_reference_diagnostic(s)
            } else {
                engine.eval_diagnostic(s)
            }
            .unwrap()
            .unwrap();
            assert_eq!(atoms(&value), [("27021597764222981".into(), "6".into())]);
        }
        let value = if semantic {
            engine.eval_semantic_reference_diagnostic("caught 0")
        } else {
            engine.eval_diagnostic("caught 0")
        }
        .unwrap()
        .unwrap();
        assert_eq!(atoms(&value), [("9007199254740993".into(), "2".into())]);
    }
}

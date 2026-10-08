use rustj::{
    Data, Engine, Value,
    enqueuer::{self, EnqueuedPayload},
    storage::CpuStorage,
    types::{BigInt, Scalar},
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
fn atoms(value: &Value) -> Vec<String> {
    let Data::ExtendedInt(v) = value.data() else {
        panic!("expected extended: {value:?}")
    };
    v.iter().map(ToString::to_string).collect()
}
#[test]
fn extended_words_preserve_precision_type_shape_and_spelling() {
    for source in [
        "0x",
        "1x",
        "01x",
        "_0x",
        "9223372036854775808x",
        "_999999999999999999999999999999999999999x",
    ] {
        assert_eq!(noun(source).type_code(), 64);
    }
    let value = noun("1x 9007199254740993 9223372036854775808");
    assert_eq!(value.shape(), &[3]);
    assert_eq!(
        atoms(&value),
        ["1", "9007199254740993", "9223372036854775808"]
    );
    assert!(value.json().contains("\"9007199254740993\""));
    assert_eq!(atoms(&noun("_0x 01 00x")), ["0", "1", "0"]);
}
#[test]
fn extended_scalar_owned_shared_views_and_selection_share_limbs() {
    let scalar = Scalar::ExtendedInt(Arc::new(BigInt::from(1)))
        .into_value()
        .unwrap();
    assert!(matches!(
        scalar.data(),
        Data::ExtendedInt(CpuStorage::Inline(_))
    ));
    let value = noun("123456789012345678901234567890x 2x");
    assert!(matches!(
        value.data(),
        Data::ExtendedInt(CpuStorage::Owned(_))
    ));
    let value = value.into_shared();
    let Data::ExtendedInt(CpuStorage::Shared(backing)) = value.data() else {
        panic!()
    };
    let weak = Arc::downgrade(&backing[0]);
    let copy = value.clone();
    let Data::ExtendedInt(CpuStorage::Shared(other)) = copy.data() else {
        panic!()
    };
    assert!(Arc::ptr_eq(backing, other));
    let selected = value.select([3], [0, 0, 1]).unwrap();
    let Data::ExtendedInt(selected_atoms) = selected.data() else {
        panic!()
    };
    assert!(Arc::ptr_eq(&backing[0], &selected_atoms[0]));
    assert!(Arc::ptr_eq(&selected_atoms[0], &selected_atoms[1]));
    let cell = value.view().cell(0, 0).unwrap().to_owned().unwrap();
    let Data::ExtendedInt(cell_atoms) = cell.data() else {
        panic!()
    };
    assert!(Arc::ptr_eq(&backing[0], &cell_atoms[0]));
    assert_eq!(atoms(&value), ["123456789012345678901234567890", "2"]);
    drop(value);
    drop(copy);
    drop(selected);
    drop(cell);
    assert!(weak.upgrade().is_none());
}
#[test]
fn extended_arithmetic_never_rounds_through_float_or_machine_int() {
    let mut engine = Engine::new();
    for (source, expected) in [
        ("9007199254740993x+1", "9007199254740994"),
        ("9223372036854775808x*2", "18446744073709551616"),
        ("0-9223372036854775808x", "-9223372036854775808"),
        ("- _9223372036854775809x", "9223372036854775809"),
        ("|_12345678901234567890x", "12345678901234567890"),
        ("*_12345678901234567890x", "-1"),
    ] {
        assert_eq!(
            atoms(&engine.eval(source).unwrap().unwrap()),
            [expected],
            "{source}"
        );
    }
    assert_eq!(
        atoms(&engine.eval("1 2+9007199254740993x").unwrap().unwrap()),
        ["9007199254740994", "9007199254740995"]
    );
    assert_eq!(
        engine
            .eval("9007199254740993x=9007199254740992x")
            .unwrap()
            .unwrap()
            .int_at(0)
            .unwrap(),
        0
    );
    assert_eq!(
        engine
            .eval("9007199254740993x>9007199254740992x")
            .unwrap()
            .unwrap()
            .int_at(0)
            .unwrap(),
        1
    );
}
#[test]
fn extended_structural_operations_keep_dtype_and_fill() {
    let mut engine = Engine::new();
    for source in [
        "$1x 2x",
        "#1x 2x",
        ",1x",
        "|.1x 2x",
        "|:2 2$1x 2x 3x 4x",
        "1{1x 9007199254740993x",
        "4{.1x 2x",
        "1}.1x 2x",
        "0$1x",
        "> <1x",
    ] {
        assert_eq!(
            engine.eval(source).unwrap().unwrap().type_code(),
            64,
            "{source}"
        );
    }
    assert_eq!(
        atoms(&engine.eval("4{.1x 2x").unwrap().unwrap()),
        ["1", "2", "0", "0"]
    );
}
#[test]
fn extended_frontend_handoff_preserves_facts_and_runtime_capture() {
    let mut engine = Engine::new();
    for source in [
        "123456789012345678901234567890x",
        "1x 9007199254740993",
        "9007199254740993x+1",
    ] {
        let handoff = engine.admit_frontend_handoff(source).into_result().unwrap();
        assert!(handoff.program().expression.is_some());
        let direct = engine.eval(source).unwrap().unwrap();
        assert_eq!(
            engine.eval_captured(source).result.unwrap().unwrap().json(),
            direct.json()
        );
        let analysis = engine.admit_logical(source).into_result().unwrap();
        let result = rustj::logical_executor::execute_closed(&analysis.logical)
            .unwrap()
            .unwrap();
        assert_eq!(result.json(), direct.json());
    }
}
#[test]
fn extended_definition_scope_alias_and_failed_assignments_preserve_state() {
    for semantic in [false, true] {
        let mut engine = Engine::new();
        for source in [
            "saved=:9007199254740993x",
            "alias=:saved",
            "f=:{{local=.saved\nlocal+1}}",
            "g=:3 : 'local=.saved\nlocal+1'",
        ] {
            engine.eval(source).unwrap();
        }
        for source in ["f 0", "g 0"] {
            let result = if semantic {
                engine.eval_semantic_reference_diagnostic(source)
            } else {
                engine.eval_diagnostic(source)
            };
            assert_eq!(atoms(&result.unwrap().unwrap()), ["9007199254740994"]);
        }
        assert_eq!(
            engine.eval_diagnostic("local").unwrap_err().kind(),
            "value error"
        );
        for source in ["saved=:1x 2xx", "saved=:1x 1E0", "saved=:1x 1.0"] {
            assert_eq!(
                engine.eval_diagnostic(source).unwrap_err().kind(),
                "ill-formed number"
            );
            assert_eq!(
                atoms(&engine.eval("saved").unwrap().unwrap()),
                ["9007199254740993"]
            );
        }
        engine.eval("saved=:2x").unwrap();
        assert_eq!(
            atoms(&engine.eval("alias").unwrap().unwrap()),
            ["9007199254740993"]
        );
    }
}
#[test]
fn extended_missing_capabilities_are_not_j_language_errors() {
    let mut engine = Engine::new();
    for source in ["1x%2x", "1x+1.0", "1x,2x", "+/1x 2x", "1x i. 2x", "i.3x"] {
        assert_eq!(
            engine.eval_diagnostic(source).unwrap_err().kind(),
            "unsupported",
            "{source}"
        );
    }
    assert!(matches!(
        rustj::physical::BufferRegistry::new()
            .unwrap()
            .register(noun("1x")),
        Err(rustj::Error::Unsupported(_))
    ));
}

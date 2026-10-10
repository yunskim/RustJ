use rustj::{Engine, name_effect_ir::Operation};

#[test]
fn consecutive_applies_share_one_graph_and_logical_workspace() {
    let mut engine = Engine::new();
    let source = "b=:1+2+3 4";
    let plan = engine.prepare_name_arrays(source).unwrap();
    assert_eq!(plan.batches().len(), 1);
    let batch = &plan.batches()[0];
    assert_eq!(
        batch
            .checkpoints()
            .iter()
            .filter(|point| !point.operations.is_empty())
            .count(),
        2
    );
    assert_eq!(
        batch
            .graph()
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, rustj::j_graph_ir::NodeKind::Apply { .. }))
            .count(),
        2
    );
    assert_eq!(batch.outputs().len(), 1, "internal result must not escape");
    assert_eq!(
        batch.checkpoints()[0].operations.end,
        batch.checkpoints()[1].operations.start
    );
    engine.execute_name_arrays(&plan).result.unwrap();
    assert_eq!(engine.eval("b").unwrap().unwrap().display(), "6 7");
}

#[test]
fn every_name_effect_ends_a_batch_but_immutable_transport_stays_ordered() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let plan = engine.prepare_name_arrays("b=:a_:+1+2+3").unwrap();
    assert!(
        plan.batches()
            .iter()
            .any(|batch| batch.checkpoints().len() >= 2)
    );
    assert!(plan.batches().len() >= 2, "left Take must separate batches");
    for batch in plan.batches() {
        for step in &plan.effects().steps()[batch.steps()] {
            assert!(matches!(
                step.operation,
                Operation::Apply { .. } | Operation::Literal(_) | Operation::Function(_)
            ));
        }
        for point in batch.checkpoints() {
            assert_eq!(point.entry, plan.effects().steps()[point.step].before);
            assert_eq!(point.success, plan.effects().steps()[point.step].after);
        }
    }
    engine.execute_name_arrays(&plan).result.unwrap();
    assert_eq!(engine.eval("b").unwrap().unwrap().display(), "13");
    assert_eq!(engine.eval("a+0").unwrap_err().kind(), "value error");
}

#[test]
fn batch_failures_restore_exact_semantic_progress_and_name_post_state() {
    for source in [
        "b=:a_:+1+2 3+4 5 6", // first Apply fails, Take never runs
        "b=:a_:+1 2+3 4 5+6", // later Apply fails, Take never runs
        "b=:1 2+a_:+3",       // Take runs; second batched Apply fails
        "b=:'x'+a_:+3",       // first succeeds, second kernel Domain
        "b=:1+2+a_:",         // success after deletion
    ] {
        let mut semantic = Engine::new();
        let mut batched = Engine::new();
        for engine in [&mut semantic, &mut batched] {
            engine.eval("a=:7 8 9").unwrap();
            engine.eval("b=:99").unwrap();
        }
        let direct = semantic.prepare_name_effects(source).unwrap();
        let plan = batched.prepare_name_arrays(source).unwrap();
        assert!(
            plan.batches()
                .iter()
                .any(|batch| batch.checkpoints().len() >= 2),
            "{source}"
        );
        let expected = semantic.execute_name_effects(&direct);
        let actual = batched.execute_name_arrays(&plan);
        assert_eq!(actual.completed, expected.completed, "{source}");
        let observations = |names: &[rustj::name_effect_ir::NameObservation]| {
            names
                .iter()
                .map(|name| {
                    (
                        name.step,
                        name.before.binding_version,
                        name.before.binding_class,
                        std::mem::discriminant(&name.before.found),
                        name.after.binding_version,
                        name.after.binding_class,
                        std::mem::discriminant(&name.after.found),
                        name.deleted,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            observations(&actual.names),
            observations(&expected.names),
            "{source}"
        );
        match (actual.result, expected.result) {
            (Ok(a), Ok(b)) => assert_eq!(a.map(|v| v.json()), b.map(|v| v.json())),
            (Err(a), Err(b)) => {
                assert_eq!(a.kind(), b.kind(), "{source}");
                assert_eq!(a.span(), b.span(), "{source}");
                assert_eq!(
                    a.context().unwrap().blame_word_index,
                    b.context().unwrap().blame_word_index,
                    "{source}"
                );
            }
            other => panic!("different outcomes: {other:?}"),
        }
        assert_eq!(batched.binding_version("a"), semantic.binding_version("a"));
        assert_eq!(
            batched.eval("b").unwrap().unwrap().json(),
            semantic.eval("b").unwrap().unwrap().json()
        );
    }
}

#[test]
fn unique_array_storage_survives_multiple_calls_without_copying() {
    let mut engine = Engine::new();
    engine.eval("a=:i.4096").unwrap();
    let plan = engine.prepare_name_arrays("1+2+a_:").unwrap();
    let batch = &plan.batches()[0];
    assert_eq!(batch.inputs().len(), 2);
    let input = rustj::Value::ints([4096], (0..4096).collect()).unwrap();
    let rustj::Data::Int(data) = input.data() else {
        panic!()
    };
    let pointer = data.as_ptr();
    let result = rustj::logical_executor::execute_with_inputs(
        batch.logical(),
        vec![rustj::Value::scalar(2), input, rustj::Value::scalar(1)],
    )
    .unwrap()
    .unwrap();
    let rustj::Data::Int(data) = result.data() else {
        panic!()
    };
    assert_eq!(data.as_ptr(), pointer);
    assert_eq!(result.int_at(4095).unwrap(), 4098);
}

#[test]
fn batch_reuse_preserves_retained_alias_and_current_array_shape() {
    let mut engine = Engine::new();
    engine.eval("a=:i.4096").unwrap();
    let plan = engine.prepare_name_arrays("b=:1+2+a_:").unwrap();
    assert_eq!(plan.batches().len(), 1);
    for setup in ["a=:i.4096", "a=:2 3$i.6"] {
        engine.eval(setup).unwrap();
        engine.eval("saved=:a").unwrap();
        let before = engine.eval("saved").unwrap().unwrap();
        engine.execute_name_arrays(&plan).result.unwrap();
        let after = engine.eval("saved").unwrap().unwrap();
        assert_eq!(before.json(), after.json());
        let result = engine.eval("b").unwrap().unwrap();
        assert_eq!(result.shape(), before.shape());
        for i in 0..result.len() {
            assert_eq!(result.int_at(i).unwrap(), before.int_at(i).unwrap() + 3);
        }
    }
}

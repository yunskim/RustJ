use rustj::{Engine, j_graph_ir::NodeKind, logical_ir::OpKind};

#[test]
fn regions_import_snapshots_without_names_or_literal_substitution() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let plan = engine.prepare_name_arrays("b=:a_:+1+2").unwrap();
    plan.verify().unwrap();
    assert_eq!(plan.regions().len(), 2);
    for region in plan.regions() {
        assert!(region.graph().write.is_none());
        assert!(region.logical().symbols.is_empty());
        assert!(region.logical().write.is_none());
        assert_eq!(
            region
                .graph()
                .nodes
                .iter()
                .filter(|node| matches!(node.kind, NodeKind::Input { .. }))
                .count(),
            region.inputs().len()
        );
        assert!(
            !region
                .logical()
                .operations
                .iter()
                .any(|op| matches!(op.kind, OpKind::ReadNoun { .. } | OpKind::Literal(_)))
        );
        assert_eq!(region.entry(), plan.effects().steps()[region.step()].before);
        assert_eq!(
            region.success(),
            plan.effects().steps()[region.step()].after
        );
        assert!(rustj::logical_executor::execute_closed(region.logical()).is_err());
    }
    engine.execute_name_arrays(&plan).result.unwrap();
    assert_eq!(engine.eval("b").unwrap().unwrap().int_at(0).unwrap(), 10);
    assert_eq!(engine.eval("a+0").unwrap_err().kind(), "value error");
}

#[test]
fn logical_and_semantic_routes_keep_values_failure_tokens_and_deletion() {
    for source in [
        "a_:+a",
        "a+a_:",
        "a_:+1 2+1 2 3",
        "a_: + 'x'",
        "b=:2 3$a_:",
        "a_: i. 1 9",
    ] {
        let mut semantic = Engine::new();
        let mut logical = Engine::new();
        for engine in [&mut semantic, &mut logical] {
            engine.eval("a=:1 2 3 4 5 6").unwrap();
        }
        let direct = semantic.prepare_name_effects(source).unwrap();
        let arrays = logical.prepare_name_arrays(source).unwrap();
        let direct = semantic.execute_name_effects(&direct);
        let arrays = logical.execute_name_arrays(&arrays);
        let outcome = |result: rustj::Result<Option<rustj::Value>>| match result {
            Ok(value) => Ok(value.map(|value| value.json())),
            Err(error) => Err(error.kind().to_owned()),
        };
        assert_eq!(outcome(direct.result), outcome(arrays.result), "{source}");
        assert_eq!(direct.completed, arrays.completed, "{source}");
        assert_eq!(
            direct
                .names
                .iter()
                .map(|event| event.deleted)
                .collect::<Vec<_>>(),
            arrays
                .names
                .iter()
                .map(|event| event.deleted)
                .collect::<Vec<_>>()
        );
        assert_eq!(semantic.binding_version("a"), logical.binding_version("a"));
        assert_eq!(semantic.binding_version("b"), logical.binding_version("b"));
    }
}

#[test]
fn input_arity_indices_and_runtime_facts_are_checked() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let plan = engine.prepare_name_arrays("a_:+1").unwrap();
    let region = &plan.regions()[0];
    let logical = region.logical();
    assert!(
        rustj::logical_executor::execute_with_inputs(logical, vec![rustj::Value::scalar(1)])
            .is_err()
    );
    let mut invalid = logical.clone();
    if let OpKind::Input { index } = &mut invalid.operations[0].kind {
        *index = 99;
    }
    assert!(invalid.verify().is_err());
    let mut invalid_graph = region.graph().clone();
    if let NodeKind::Input { index } = &mut invalid_graph.nodes[0].kind {
        *index = 99;
    }
    assert!(invalid_graph.verify().is_err());
    let mut invalid = logical.clone();
    let input = invalid.operations[0].results[0];
    invalid.values[input.0].facts.shape = Some(vec![3]);
    invalid.values[input.0].facts.rank = Some(1);
    let result = rustj::logical_executor::execute_with_inputs(
        &invalid,
        vec![rustj::Value::scalar(7), rustj::Value::scalar(1)],
    );
    assert_eq!(result.unwrap_err().kind(), "unsupported");
}

#[test]
fn function_transfer_and_pos_admission_keep_the_outer_effect_contract() {
    let mut engine = Engine::new();
    engine.eval("f=:@:").unwrap();
    let plan = engine.prepare_name_arrays("saved=:f_:").unwrap();
    assert!(plan.regions().is_empty());
    engine.execute_name_arrays(&plan).result.unwrap();
    engine.eval("h=:-saved+").unwrap();
    assert_eq!(engine.eval("h 3").unwrap().unwrap().int_at(0).unwrap(), -3);
    engine.eval("a=:7").unwrap();
    engine.eval("b=:9").unwrap();
    let plan = engine.prepare_name_arrays("a+b_:").unwrap();
    engine.eval("a=:+").unwrap();
    let result = engine.execute_name_arrays(&plan);
    assert_eq!(result.result.unwrap_err().kind(), "unsupported");
    assert_eq!(result.completed.0, 0);
    assert!(result.names.is_empty());
    assert_eq!(engine.eval("b").unwrap().unwrap().int_at(0).unwrap(), 9);
}

#[test]
fn last_use_moves_unique_array_storage_and_preserves_external_aliases() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let plan = engine.prepare_name_arrays("a_:+1").unwrap();
    let region = plan.regions()[0].logical();
    let input = rustj::Value::ints([4096], (0..4096).collect()).unwrap();
    let rustj::Data::Int(data) = input.data() else {
        panic!()
    };
    let pointer = data.as_ptr();
    let result =
        rustj::logical_executor::execute_with_inputs(region, vec![input, rustj::Value::scalar(1)])
            .unwrap()
            .unwrap();
    let rustj::Data::Int(data) = result.data() else {
        panic!()
    };
    assert_eq!(
        data.as_ptr(),
        pointer,
        "last-use input should reach the in-place kernel without copying"
    );
    assert_eq!(result.int_at(4095).unwrap(), 4096);
    let retained = result.into_shared();
    let result = rustj::logical_executor::execute_with_inputs(
        region,
        vec![retained.clone(), rustj::Value::scalar(1)],
    )
    .unwrap()
    .unwrap();
    assert_eq!(retained.int_at(0).unwrap(), 1);
    assert_eq!(result.int_at(0).unwrap(), 2);
    let rustj::Data::Int(data) = result.data() else {
        panic!()
    };
    assert_ne!(
        data.as_ptr(),
        pointer,
        "a retained alias forbids destructive reuse"
    );
}

#[test]
fn closed_fanout_keeps_shared_values_until_every_branch_and_check_finishes() {
    let mut engine = Engine::new();
    let source = "(+ + *)1 2 3";
    let plan = engine.analyze(source).unwrap();
    let result = rustj::logical_executor::execute_closed(&plan)
        .unwrap()
        .unwrap();
    assert_eq!(result.json(), engine.eval(source).unwrap().unwrap().json());
}

#[test]
fn reused_plan_reads_current_values_shapes_and_generations() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let plan = engine.prepare_name_arrays("b=:a_:+1").unwrap();
    for (setup, expected) in [
        ("a=:7", "8"),
        ("a=:i.6", "1 2 3 4 5 6"),
        ("a=:2 3$i.6", "1 2 3\n4 5 6"),
    ] {
        engine.eval(setup).unwrap();
        engine.execute_name_arrays(&plan).result.unwrap();
        assert_eq!(engine.eval("b").unwrap().unwrap().display(), expected);
        assert_eq!(engine.eval("a+0").unwrap_err().kind(), "value error");
    }
}

#[test]
fn logical_failures_preserve_parent_source_and_parser_blame() {
    for source in ["a_:+1 2+1 2 3", "b=:a_: + 'x'", "a+a_:"] {
        let mut semantic = Engine::new();
        let mut logical = Engine::new();
        for engine in [&mut semantic, &mut logical] {
            engine.eval("a=:7").unwrap();
        }
        let semantic_plan = semantic.prepare_name_effects(source).unwrap();
        let logical_plan = logical.prepare_name_arrays(source).unwrap();
        let expected = semantic.execute_name_effects(&semantic_plan);
        let actual = logical.execute_name_arrays(&logical_plan);
        assert_eq!(actual.completed, expected.completed, "{source}");
        let expected = expected.result.unwrap_err();
        let actual = actual.result.unwrap_err();
        assert_eq!(actual.kind(), expected.kind(), "{source}");
        assert_eq!(actual.span(), expected.span(), "{source}");
        assert_eq!(
            actual.context().unwrap().blame_word_index,
            expected.context().unwrap().blame_word_index,
            "{source}"
        );
        assert!(actual.span().is_some());
    }
}

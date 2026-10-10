use rustj::{Engine, frontend_context::FoundScope, name_effect_ir::Operation};

fn scalar(engine: &mut Engine, source: &str, expected: i64) {
    assert_eq!(
        engine.eval(source).unwrap().unwrap().int_at(0).unwrap(),
        expected
    );
}

#[test]
fn lowering_is_nonexecuting_and_uses_parser_order() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let before = engine.binding_version("a");
    let plan = engine.prepare_name_effects("a_:+a").unwrap();
    plan.verify().unwrap();
    let names: Vec<_> = plan
        .steps()
        .iter()
        .filter_map(|step| match &step.operation {
            Operation::Read { name, .. } => Some((name.as_str(), false)),
            Operation::Take { name, .. } => Some((name.as_str(), true)),
            _ => None,
        })
        .collect();
    assert_eq!(names, [("a", false), ("a", true)]);
    assert_eq!(engine.binding_version("a"), before);
    scalar(&mut engine, "a", 7);
    let run = engine.execute_name_effects(&plan);
    assert_eq!(run.result.unwrap().unwrap().int_at(0).unwrap(), 14);
    assert_eq!(run.completed.0, plan.steps().len());
    assert_eq!(run.names.len(), 2);
    assert!(!run.names[0].deleted);
    assert!(run.names[1].deleted);
    assert!(matches!(run.names[1].before.found, FoundScope::Global(_)));
    assert_eq!(run.names[1].after.found, FoundScope::Missing);
    assert!(run.names[1].before.binding_generation.is_some());
    assert_eq!(engine.eval("a+0").unwrap_err().kind(), "value error");
}

#[test]
fn errors_stop_tokens_without_replay_or_rollback() {
    for (source, error, deleted) in [
        ("a+a_:", "value error", true),
        ("a_:+1 2+1 2 3", "length error", false),
        ("a_:+1 2 3+1 2", "length error", false),
        ("a_: + 'x'", "domain error", true),
    ] {
        let mut engine = Engine::new();
        engine.eval("a=:7").unwrap();
        let plan = engine.prepare_name_effects(source).unwrap();
        let run = engine.execute_name_effects(&plan);
        assert_eq!(run.result.unwrap_err().kind(), error, "{source}");
        assert!(run.completed.0 < plan.steps().len());
        assert_eq!(engine.binding_version("a").is_none(), deleted, "{source}");
        assert_eq!(
            run.names.iter().filter(|event| event.deleted).count(),
            usize::from(deleted)
        );
    }
}

#[test]
fn plan_reuse_reads_current_values_and_preserves_aba_identity() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let plan = engine.prepare_name_effects("b=:a_:+a").unwrap();
    let first = engine.execute_name_effects(&plan);
    first.result.unwrap();
    scalar(&mut engine, "b", 14);
    engine.eval("a=:11").unwrap();
    let second = engine.execute_name_effects(&plan);
    second.result.unwrap();
    scalar(&mut engine, "b", 22);
    assert_ne!(
        first.names[0].before.binding_generation,
        second.names[0].before.binding_generation
    );
    assert!(
        second
            .names
            .iter()
            .any(|event| event.before.binding_generation.is_none()
                || event.after.binding_version != event.before.binding_version)
    );
}

#[test]
fn missing_lookup_is_not_hoisted_past_an_earlier_kernel_error() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let plan = engine.prepare_name_effects("a_:+1 2+1 2 3").unwrap();
    engine.eval("a_:").unwrap();
    let run = engine.execute_name_effects(&plan);
    assert_eq!(run.result.unwrap_err().kind(), "length error");
    assert!(run.names.is_empty());
}

#[test]
fn changed_pos_rejects_before_any_effect_but_new_values_do_not() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    engine.eval("b=:9").unwrap();
    let plan = engine.prepare_name_effects("a+b_:").unwrap();
    engine.eval("a=:+").unwrap();
    let before = engine.binding_version("b");
    let run = engine.execute_name_effects(&plan);
    assert_eq!(run.result.unwrap_err().kind(), "unsupported");
    assert_eq!(run.completed.0, 0);
    assert!(run.names.is_empty());
    assert_eq!(engine.binding_version("b"), before);
}

#[test]
fn function_take_transfers_actual_value_without_freezing_body_or_inner_name() {
    for setup in ["f=:+", "f=:/", "f=:@:"] {
        let mut engine = Engine::new();
        engine.eval(setup).unwrap();
        let plan = engine.prepare_name_effects("saved=:(f_:)").unwrap();
        let run = engine.execute_name_effects(&plan);
        run.result.unwrap();
        assert!(run.names[0].deleted);
        assert_eq!(engine.eval("f 0").unwrap_err().kind(), "value error");
        match setup {
            "f=:+" => scalar(&mut engine, "saved 3", 3),
            "f=:/" => scalar(&mut engine, "+saved 1 2 3", 6),
            _ => {
                engine.eval("h=:-saved+").unwrap();
                scalar(&mut engine, "h 3", -3);
            }
        }
    }
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    let plan = engine.prepare_name_effects("saved=:f_:").unwrap();
    engine.eval("f=:-").unwrap();
    engine.execute_name_effects(&plan).result.unwrap();
    scalar(&mut engine, "saved 3", -3);
    engine.eval("base=:+").unwrap();
    engine.eval("alias=:base").unwrap();
    let plan = engine.prepare_name_effects("saved=:alias_:").unwrap();
    engine.execute_name_effects(&plan).result.unwrap();
    engine.eval("base=:-").unwrap();
    scalar(&mut engine, "saved 3", -3);
}

#[test]
fn unsupported_admission_does_not_delete_names_or_run_definitions() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    engine.eval("c=:@:").unwrap();
    assert_eq!(
        engine
            .prepare_name_effects("missing_:+1 2+1 2 3")
            .unwrap_err()
            .kind(),
        "unsupported"
    );
    assert_eq!(
        engine.eval("missing_:+1 2+1 2 3").unwrap_err().kind(),
        "length error"
    );
    for source in ["h=:-c_:+", "f=:{{a_:}}", "f=:+/a_:", "'a b'=:a_:"] {
        assert!(engine.prepare_name_effects(source).is_err(), "{source}");
        scalar(&mut engine, "a", 7);
    }
    engine.eval("h=:-c+").unwrap();
    scalar(&mut engine, "h 3", -3);
}

#[test]
fn missing_ordinary_name_at_entry_rejects_a_stale_noun_plan() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let plan = engine.prepare_name_effects("b=:a").unwrap();
    engine.eval("a_:").unwrap();
    let run = engine.execute_name_effects(&plan);
    assert_eq!(run.result.unwrap_err().kind(), "unsupported");
    assert_eq!(run.completed.0, 0);
    assert!(run.names.is_empty());
    assert_eq!(engine.eval("b 0").unwrap_err().kind(), "value error");
    // A fresh J parse can treat the missing ordinary name as a verb reference.
    engine.eval("b=:a").unwrap();
    engine.eval("a=:+").unwrap();
    scalar(&mut engine, "b 3", 3);
}

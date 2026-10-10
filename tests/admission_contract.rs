use rustj::{
    Engine, Error,
    admission::Stage,
    error::{DiagnosticPhase, FailureCategory},
};

#[test]
fn categories_survive_context_and_machine_unlocation() {
    for (error, category, kind, catchable) in [
        (
            Error::Length,
            FailureCategory::JLanguage,
            "length error",
            true,
        ),
        (
            Error::Unsupported("valid J, missing route".into()),
            FailureCategory::UnsupportedCapability,
            "unsupported",
            false,
        ),
        (
            Error::Verification("broken SSA".into()),
            FailureCategory::VerifierDefect,
            "verifier failure",
            false,
        ),
        (
            Error::Backend("admitted implementation failed".into()),
            FailureCategory::BackendFailure,
            "backend failure",
            false,
        ),
    ] {
        let located = error.at(2..3).in_phase(DiagnosticPhase::SemanticAnalysis);
        assert_eq!(located.category(), category);
        assert_eq!(located.kind(), kind);
        assert_eq!(located.is_j_catchable(), catchable);
        let plain = located.into_unlocated();
        assert_eq!(plain.category(), category);
        assert_eq!(plain.is_j_catchable(), catchable);
    }
}

#[test]
fn frontend_success_is_distinct_from_binding_and_execution_admission() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let source = "b=:a+a_:";
    let frontend = engine.admit_frontend(source);
    assert_eq!(frontend.stage(), Stage::Frontend);
    frontend
        .result()
        .unwrap()
        .frontend
        .as_ref()
        .unwrap()
        .verify()
        .unwrap();
    let bound = engine.admit_semantic(source);
    let rejection = bound.result().unwrap_err();
    assert_eq!(rejection.stage(), Stage::SemanticBinding);
    assert_eq!(rejection.category(), FailureCategory::UnsupportedCapability);
    assert!(rejection.may_inspect_another_route());
    assert!(!rejection.is_j_handler_eligible());
    assert!(engine.admit_j_graph(source).result().is_err());
    assert!(engine.admit_logical(source).result().is_err());
    let effects = engine.admit_name_effects(source).into_result().unwrap();
    let arrays = engine.admit_name_arrays(source).into_result().unwrap();
    assert_eq!(engine.eval("a").unwrap().unwrap().int_at(0).unwrap(), 7);
    assert_eq!(engine.eval("b+0").unwrap_err().kind(), "value error");
    // Only this explicit execution performs Take; it must never be replayed.
    let execution = engine.execute_name_arrays(&arrays);
    assert_eq!(execution.result.unwrap_err().kind(), "value error");
    assert!(execution.names.iter().any(|name| name.deleted));
    assert!(execution.completed.0 > 0);
    assert_eq!(engine.eval("a+0").unwrap_err().kind(), "value error");
    effects.verify().unwrap();
}

#[test]
fn frontend_rejection_retains_original_prefix_but_has_no_j_handler_authority() {
    let engine = Engine::new();
    let admission = engine.admit_frontend("(+/\"(1+0)) 1 2");
    let failure = admission.result().unwrap_err();
    assert_eq!(failure.category(), FailureCategory::UnsupportedCapability);
    let prefix = failure.frontend_prefix().unwrap();
    assert!(!prefix.complete);
    assert!(prefix.pending.is_some());
    assert!(!prefix.reductions.is_empty());
    prefix.verify().unwrap();
    assert!(prefix.source_origin.is_some());
    let bad = engine.admit_frontend("1+)");
    let syntax = bad.result().unwrap_err();
    assert_eq!(syntax.category(), FailureCategory::JLanguage);
    assert!(!syntax.is_j_handler_eligible());
    assert!(!syntax.may_inspect_another_route());
}

#[test]
fn inspection_of_definitions_and_writing_constructors_never_executes_effects() {
    let mut engine = Engine::new();
    engine.eval("count=:0").unwrap();
    engine.eval("write=:1 : 'count=:count+1\nu' ").unwrap();
    for source in ["f=:{{count=:count+1\ny}}", "3 write"] {
        let _ = engine.admit_frontend(source);
        let _ = engine.admit_semantic(source);
        let _ = engine.admit_j_graph(source);
        let _ = engine.admit_logical(source);
        let _ = engine.admit_name_effects(source);
        let _ = engine.admit_name_arrays(source);
    }
    assert_eq!(engine.eval("count").unwrap().unwrap().int_at(0).unwrap(), 0);
    assert_eq!(engine.eval("f+0").unwrap_err().kind(), "value error");
}

#[test]
fn corrupted_logical_ir_is_a_verifier_defect_before_execution() {
    let engine = Engine::new();
    let mut plan = engine.admit_logical("1+2").into_result().unwrap().logical;
    plan.result = Some(rustj::logical_ir::ValueId(usize::MAX));
    let error = rustj::logical_executor::execute_closed(&plan).unwrap_err();
    assert_eq!(error.category(), FailureCategory::VerifierDefect);
    assert!(!error.is_j_catchable());
    assert_eq!(error.kind(), "verifier failure");
}

#[test]
fn capability_misses_bypass_catch_while_j_errors_preserve_effect_order() {
    let mut engine = Engine::new();
    engine.eval("count=:0").unwrap();
    engine
        .eval("f=:{{try. count=:count+1\n+&2 y catch. 42 end.}}")
        .unwrap();
    let error = engine.eval_diagnostic("f 3").unwrap_err();
    assert_eq!(error.category(), FailureCategory::UnsupportedCapability);
    assert!(!error.is_j_catchable());
    assert!(!error.context().unwrap().source_frames.is_empty());
    assert_eq!(engine.eval("count").unwrap().unwrap().int_at(0).unwrap(), 1);
    engine
        .eval("g=:{{try. count=:count+1\n1 2+1 2 3 catch. count end.}}")
        .unwrap();
    assert_eq!(engine.eval("g 0").unwrap().unwrap().int_at(0).unwrap(), 2);
}

#[test]
fn valid_logical_plan_with_wrong_inputs_is_a_capability_miss_not_a_defect() {
    let mut engine = Engine::new();
    engine.eval("a=:1 2").unwrap();
    let plan = engine.prepare_name_arrays("a+1").unwrap();
    let region = &plan.regions()[0];
    let error = rustj::logical_executor::execute_with_inputs(region.logical(), vec![]).unwrap_err();
    assert_eq!(error.category(), FailureCategory::UnsupportedCapability);
    region.logical().verify().unwrap();
}

use rustj::{
    Engine,
    frontend_context::{NameGuardCheck, NameUseId, SimpleNameGuard},
    name_guards::{
        AliasCall, AliasCallAttempt, AliasCallGuard, AliasGuardAdmission, AliasGuardCheck,
    },
};

fn prepare(engine: &mut Engine, name: &str) -> AliasCallGuard {
    // Public capture API retains the actual same-parser NAME context. The
    // ordinary call runs normally; preparation is inspection and adds no call.
    let capture = engine.eval_captured(&format!("{name} 2"));
    capture.result.unwrap();
    engine
        .prepare_alias_call_guard(capture.capture.frontend.as_ref().unwrap(), NameUseId(0))
        .unwrap()
}

#[test]
fn public_capture_retains_complete_context_and_failed_prefix_without_reparsing() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let capture = engine.eval_captured("a+1");
    assert_eq!(capture.result.unwrap().unwrap().int_at(0).unwrap(), 8);
    let context = capture.capture.frontend.unwrap();
    context.verify().unwrap();
    assert!(context.complete);
    assert!(context.name_uses[0].lookup.is_some());
    let failed = engine.eval_captured("a+(1 2+1 2 3)");
    assert_eq!(failed.result.unwrap_err().kind(), "length error");
    let prefix = failed.capture.frontend.unwrap();
    prefix.verify().unwrap();
    assert!(!prefix.complete);
    assert!(prefix.pending.is_some());
    assert!(SimpleNameGuard::from_name_use(&prefix, NameUseId(0)).is_err());
}

#[test]
fn transitive_alias_rebinding_invalidates_exact_dependency_not_just_root() {
    let mut engine = Engine::new();
    for source in ["f=:+", "g=:f", "h=:g"] {
        engine.eval(source).unwrap();
    }
    let guard = prepare(&mut engine, "h");
    guard.verify().unwrap();
    assert_eq!(guard.context().unit, guard.root().origin().0);
    assert_eq!(
        guard
            .reads()
            .iter()
            .map(|read| read.name())
            .collect::<Vec<_>>(),
        ["h", "g", "f"]
    );
    assert_eq!(
        engine.check_alias_call_guard(&guard),
        AliasGuardCheck::ValidAtCheck
    );
    engine.eval("unrelated=:99").unwrap();
    assert_eq!(
        engine.check_alias_call_guard(&guard),
        AliasGuardCheck::ValidAtCheck
    );
    engine.eval("f=:*").unwrap();
    assert_eq!(
        engine.check_name_guard(guard.root()),
        NameGuardCheck::ValidAtCheck
    );
    assert_eq!(
        engine.check_alias_call_guard(&guard),
        AliasGuardCheck::Invalidated {
            read: 2,
            reason: NameGuardCheck::LookupChanged,
        }
    );
    assert!(engine.validate_alias_call_guard(&guard).is_err());
    assert_eq!(engine.eval("h _2").unwrap().unwrap().int_at(0).unwrap(), -1);
}

#[test]
fn call_lease_uses_existing_monad_dyad_kernels_and_preserves_kernel_errors() {
    let mut engine = Engine::new();
    for primitive in ["+", "-", "*", "%"] {
        engine.eval(&format!("f=:{primitive}")).unwrap();
        engine.eval("g=:f").unwrap();
        let guard = prepare(&mut engine, "g");
        let y = engine.eval("i.6").unwrap().unwrap();
        let expected = engine.eval("g i.6").unwrap().unwrap();
        let actual = engine
            .validate_alias_call_guard(&guard)
            .unwrap()
            .apply_monad(y)
            .unwrap();
        assert_eq!(actual.json(), expected.json());
        let x = engine.eval("i.6").unwrap().unwrap();
        let y = engine.eval("1+i.6").unwrap().unwrap();
        let expected = engine.eval("(i.6) g 1+i.6").unwrap().unwrap();
        let actual = engine
            .validate_alias_call_guard(&guard)
            .unwrap()
            .apply_dyad(x, y)
            .unwrap();
        assert_eq!(actual.json(), expected.json());
        let x = engine.eval("i.2").unwrap().unwrap();
        let y = engine.eval("i.3").unwrap().unwrap();
        let error = engine
            .validate_alias_call_guard(&guard)
            .unwrap()
            .apply_dyad(x, y)
            .unwrap_err();
        assert_eq!(error.kind(), "length error");
    }
}

#[test]
fn effects_in_argument_evaluation_invalidate_before_call_without_replay() {
    let mut engine = Engine::new();
    for source in [
        "f=:+",
        "g=:f",
        "count=:0",
        "change=:{{ count=:count+1\nf=:*\nu }}",
    ] {
        engine.eval(source).unwrap();
    }
    let guard = prepare(&mut engine, "g");
    // An actual supported explicit modifier performs the argument-side effects.
    engine.eval("argument=:_2 change").unwrap();
    let y = engine.eval("argument").unwrap().unwrap();
    assert!(engine.validate_alias_call_guard(&guard).is_err());
    assert_eq!(engine.eval("count").unwrap().unwrap().int_at(0).unwrap(), 1);
    // Choose a supported interpreter call explicitly with the already computed
    // argument. Do not evaluate change again or restart a sentence.
    assert_eq!(y.int_at(0).unwrap(), -2);
    assert_eq!(
        engine
            .eval("g argument")
            .unwrap()
            .unwrap()
            .int_at(0)
            .unwrap(),
        -1
    );
    assert_eq!(engine.eval("count").unwrap().unwrap().int_at(0).unwrap(), 1);
}

#[test]
fn alias_depth_limit_is_a_coverage_result_without_executing_or_looping() {
    let mut engine = Engine::new();
    engine.eval("n0=:+").unwrap();
    for i in 1..=rustj::semantic::MAX_EXPR_DEPTH + 1 {
        engine.eval(&format!("n{i}=:n{}", i - 1)).unwrap();
    }
    let captured = engine.eval_captured(&format!("n{}", rustj::semantic::MAX_EXPR_DEPTH + 1));
    assert_eq!(captured.result.unwrap_err().kind(), "unsupported");
    let context = captured.capture.frontend.unwrap();
    assert!(context.complete);
    assert!(matches!(
        engine.prepare_alias_call_guard(&context, NameUseId(0)),
        Err(AliasGuardAdmission::DepthLimit)
    ));
}

#[test]
fn admission_never_guesses_derived_operands_definitions_missing_names_or_cycles() {
    let mut engine = Engine::new();
    for source in ["f=:+", "g=:f"] {
        engine.eval(source).unwrap();
    }
    let observed = engine.eval_captured("g 2");
    let origin = observed.capture.frontend.unwrap();
    engine.eval("f=:7").unwrap();
    assert!(
        matches!(engine.prepare_alias_call_guard(&origin, NameUseId(0)), Err(AliasGuardAdmission::WrongPartOfSpeech(name)) if name == "f")
    );
    engine.eval("f=:later").unwrap();
    assert!(
        matches!(engine.prepare_alias_call_guard(&origin, NameUseId(0)), Err(AliasGuardAdmission::UnboundTarget(name)) if name == "later")
    );
    engine.eval("f=:g").unwrap();
    assert!(matches!(
        engine.prepare_alias_call_guard(&origin, NameUseId(0)),
        Err(AliasGuardAdmission::Cycle(_))
    ));
    for target in ["+/", "(+ #)", "{{ y }}", "i."] {
        engine.eval(&format!("f=:{target}")).unwrap();
        assert!(matches!(
            engine.prepare_alias_call_guard(&origin, NameUseId(0)),
            Err(AliasGuardAdmission::UnsupportedTarget)
        ));
    }
}

#[test]
fn scope_engine_identity_and_root_staleness_remain_admission_conditions() {
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    let captured = engine.eval_captured("f 2");
    let context = captured.capture.frontend.unwrap();
    let guard = engine
        .prepare_alias_call_guard(&context, NameUseId(0))
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(guard.context(), &context));
    let mut other = Engine::new();
    other.eval("f=:+").unwrap();
    assert_eq!(
        other.check_alias_call_guard(&guard),
        AliasGuardCheck::Invalidated {
            read: 0,
            reason: NameGuardCheck::EngineChanged,
        }
    );
    engine.eval("f=:+").unwrap();
    assert!(matches!(
        engine.prepare_alias_call_guard(&context, NameUseId(0)),
        Err(AliasGuardAdmission::RootChanged(
            NameGuardCheck::LookupChanged
        ))
    ));
    let diagnostic = rustj::parser::parse_frontend("f 2");
    assert!(diagnostic.is_err()); // Diagnostic noun assumptions cannot invent a verb.
    let noun = engine.eval_captured("a=:3");
    assert!(
        engine
            .prepare_alias_call_guard(noun.capture.frontend.as_ref().unwrap(), NameUseId(0))
            .is_err()
    );
}

#[test]
fn call_ready_miss_retains_both_arguments_and_resumes_without_rebinding_nouns() {
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    engine.eval("g=:f").unwrap();
    let guard = std::sync::Arc::new(prepare(&mut engine, "g"));
    engine.eval("a=:2 3").unwrap();
    let y = engine.eval("a").unwrap().unwrap();
    let x = engine.eval("4 5").unwrap().unwrap();
    engine.eval("a=:9 9").unwrap();
    engine.eval("f=:*").unwrap();
    let AliasCallAttempt::Miss(miss) = engine.try_alias_call(AliasCall::new(guard, Some(x), y))
    else {
        panic!()
    };
    assert_eq!(miss.call.left().unwrap().int_at(0).unwrap(), 4);
    assert_eq!(miss.call.right().int_at(0).unwrap(), 2);
    let AliasCallAttempt::Executed(result) = engine.resume_alias_call(miss) else {
        panic!()
    };
    assert_eq!(
        result.unwrap().json(),
        engine.eval("8 15").unwrap().unwrap().json()
    );
}

#[test]
fn call_ready_resume_distinguishes_j_errors_from_scope_refusal_and_no_replay() {
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    engine.eval("g=:f").unwrap();
    let guard = std::sync::Arc::new(prepare(&mut engine, "g"));
    let y = engine.eval("2").unwrap().unwrap();
    engine.eval("f=:7").unwrap();
    let AliasCallAttempt::Miss(miss) =
        engine.try_alias_call(AliasCall::new(guard.clone(), None, y))
    else {
        panic!()
    };
    let mut other = Engine::new();
    let AliasCallAttempt::Miss(miss) = other.resume_alias_call(miss) else {
        panic!()
    };
    assert_eq!(miss.call.right().int_at(0).unwrap(), 2);
    let AliasCallAttempt::Executed(result) = engine.resume_alias_call(miss) else {
        panic!()
    };
    assert_eq!(result.unwrap_err().kind(), "domain error");
    engine.eval("f=:later").unwrap();
    let y = engine.eval("2").unwrap().unwrap();
    let AliasCallAttempt::Miss(miss) = engine.try_alias_call(AliasCall::new(guard, None, y)) else {
        panic!()
    };
    let AliasCallAttempt::Executed(result) = engine.resume_alias_call(miss) else {
        panic!()
    };
    assert_eq!(result.unwrap_err().kind(), "value error");
}

#[test]
fn call_ready_success_kernel_error_and_effect_miss_have_distinct_outcomes() {
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    engine.eval("g=:f").unwrap();
    let guard = std::sync::Arc::new(prepare(&mut engine, "g"));
    let y = engine.eval("2 3").unwrap().unwrap();
    let AliasCallAttempt::Executed(result) =
        engine.try_alias_call(AliasCall::new(guard.clone(), None, y))
    else {
        panic!()
    };
    assert_eq!(result.unwrap().int_at(0).unwrap(), 2);
    let x = engine.eval("i.2").unwrap().unwrap();
    let y = engine.eval("i.3").unwrap().unwrap();
    let AliasCallAttempt::Executed(result) =
        engine.try_alias_call(AliasCall::new(guard.clone(), Some(x), y))
    else {
        panic!()
    };
    assert_eq!(result.unwrap_err().kind(), "length error");
    engine.eval("count=:0").unwrap();
    engine
        .eval("change=:{{ count=:count+1\nf=:*\nu }}")
        .unwrap();
    let y = engine.eval("_2 change").unwrap().unwrap();
    let AliasCallAttempt::Miss(miss) = engine.try_alias_call(AliasCall::new(guard, None, y)) else {
        panic!()
    };
    let AliasCallAttempt::Executed(result) = engine.resume_alias_call(miss) else {
        panic!()
    };
    assert_eq!(result.unwrap().int_at(0).unwrap(), -1);
    assert_eq!(engine.eval("count").unwrap().unwrap().int_at(0).unwrap(), 1);
}

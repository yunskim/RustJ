use rustj::{
    Engine,
    error::FailureCategory,
    frontend_context::{NamePolicy, NodeKind},
    frontend_handoff::VerifiedFrontend,
    parser::parse_frontend,
    semantic::{ExprKind, FunctionHead},
};
use std::sync::Arc;

#[test]
fn handoff_preserves_original_program_occurrences_and_shared_functions() {
    for source in [
        "",
        "NB. comment",
        "1+2*3",
        "((1+2))",
        "a=:1",
        "'a b'=:3 4",
        "'a'=:7",
        "adv=:/",
        "(+/ % #)",
        "f=:{{y+1}}",
        "f=:3 : 't=.y\nt+1'",
    ] {
        let program = parse_frontend(source).unwrap();
        let context = program.frontend.clone().unwrap();
        let handoff =
            VerifiedFrontend::from_program(program).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert!(Arc::ptr_eq(handoff.context(), &context));
        assert_eq!(handoff.source_origin().text(), source);
        assert_eq!(handoff.program().source, source);
        assert_eq!(handoff.context().steps.len(), context.steps.len());
    }
}

#[test]
fn names_keep_capture_late_lookup_and_abandon_requirements() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    engine.eval("f=:+").unwrap();
    engine.eval("adv=:/").unwrap();
    for (source, policy, ordered, late) in [
        ("a+1", NamePolicy::CaptureAtRead, false, false),
        ("f a", NamePolicy::LateAtCall, false, true),
        ("g=:f_:", NamePolicy::CaptureAndAbandon, true, false),
        ("sum=:+adv", NamePolicy::CaptureAtRead, true, false),
    ] {
        let handoff = engine.admit_frontend_handoff(source).into_result().unwrap();
        assert!(
            handoff
                .context()
                .name_uses
                .iter()
                .any(|n| n.policy == policy),
            "{source}"
        );
        let requirements = handoff.requirements();
        assert!(requirements.name_environment);
        assert_eq!(requirements.ordered_name_effects, ordered, "{source}");
        assert_eq!(requirements.late_function_lookup, late, "{source}");
    }
    assert_eq!(engine.eval("a").unwrap().unwrap().int_at(0).unwrap(), 7);
    assert_eq!(engine.eval("f 3").unwrap().unwrap().int_at(0).unwrap(), 3);
    assert_eq!(engine.eval("g+0").unwrap_err().kind(), "value error");
}

#[test]
fn explicit_and_direct_definition_scopes_survive_without_body_evaluation() {
    let mut engine = Engine::new();
    engine.eval("global=:10").unwrap();
    for source in [
        "f=:{{local=.y\nglobal=:local\ninner=.{{y+1}}\ninner local}}",
        "f=:3 : 'local=.y\nglobal=:local\nlocal+1'",
    ] {
        let handoff = engine.admit_frontend_handoff(source).into_result().unwrap();
        assert!(handoff.requirements().deferred_definition_bodies);
        let Some(ExprKind::VerbValue(verb)) =
            handoff.program().expression.as_ref().map(|e| &e.kind)
        else {
            panic!("definition");
        };
        let FunctionHead::ExplicitDefinition(code) = &verb.entity.head else {
            panic!("code");
        };
        code.verify().unwrap();
        assert!(
            code.name_plan
                .monad
                .local_declarations
                .iter()
                .any(|n| n == "local")
        );
        assert!(
            code.name_plan
                .monad
                .occurrences
                .iter()
                .any(|n| n.role
                    == rustj::definition_code::DefinitionNameRole::GlobalAssignmentTarget)
        );
    }
    assert_eq!(
        engine.eval("global").unwrap().unwrap().int_at(0).unwrap(),
        10
    );
    assert_eq!(engine.eval("f+0").unwrap_err().kind(), "value error");
}

#[test]
fn same_handoff_lowers_ordered_effects_without_reparsing_or_lookup() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let handoff = engine
        .admit_frontend_handoff("b=:1+a_:")
        .into_result()
        .unwrap();
    let context = handoff.context().clone();
    engine.eval("a=:20").unwrap();
    let plan = handoff.lower_name_effects().unwrap();
    assert!(Arc::ptr_eq(
        plan.program().frontend.as_ref().unwrap(),
        &context
    ));
    assert_eq!(engine.eval("a").unwrap().unwrap().int_at(0).unwrap(), 20);
    engine.execute_name_effects(&plan).result.unwrap();
    assert_eq!(engine.eval("b").unwrap().unwrap().int_at(0).unwrap(), 21);
    assert_eq!(engine.eval("a+0").unwrap_err().kind(), "value error");
}

#[test]
fn observed_execution_is_not_a_deferred_handoff_or_replay_permission() {
    let mut engine = Engine::new();
    engine.eval("a=:7").unwrap();
    let observed = engine.eval_captured("b=:a_:+1");
    observed.result.unwrap();
    let mut program = parse_frontend("b=:a_:+1").unwrap();
    program.frontend = observed.capture.frontend;
    let error = VerifiedFrontend::from_program(program).unwrap_err();
    assert_eq!(error.category(), FailureCategory::UnsupportedCapability);
    assert_eq!(engine.eval("b").unwrap().unwrap().int_at(0).unwrap(), 8);
    assert_eq!(engine.eval("a+0").unwrap_err().kind(), "value error");
}

#[test]
fn forged_semantic_name_function_and_source_links_are_verifier_defects() {
    let mut named = parse_frontend("a+1").unwrap();
    if let ExprKind::Dyad { left, .. } = &mut named.expression.as_mut().unwrap().kind {
        left.kind = ExprKind::ReadName("different".into());
    } else {
        panic!("dyad");
    }
    assert_eq!(
        VerifiedFrontend::from_program(named)
            .unwrap_err()
            .category(),
        FailureCategory::VerifierDefect
    );
    let mut function = parse_frontend("+/1 2").unwrap();
    if let ExprKind::Monad { verb, .. } = &mut function.expression.as_mut().unwrap().kind {
        let replacement = parse_frontend("+/").unwrap().expression.unwrap();
        let ExprKind::VerbValue(replacement) = replacement.kind else {
            panic!("verb");
        };
        verb.entity = replacement.entity;
    } else {
        panic!("monad");
    }
    assert_eq!(
        VerifiedFrontend::from_program(function)
            .unwrap_err()
            .category(),
        FailureCategory::VerifierDefect
    );
    let mut source = parse_frontend("1+2").unwrap();
    source.source = "2+1".into();
    assert_eq!(
        VerifiedFrontend::from_program(source)
            .unwrap_err()
            .category(),
        FailureCategory::VerifierDefect
    );
}

#[test]
fn verified_handoff_keeps_modifier_result_pos_and_name_observations() {
    let mut engine = Engine::new();
    engine.eval("adv=:/").unwrap();
    let handoff = engine
        .admit_frontend_handoff("saved=:adv")
        .into_result()
        .unwrap();
    assert!(matches!(
        handoff.program().expression.as_ref().unwrap().kind,
        ExprKind::ModifierValue(_)
    ));
    assert!(handoff.context().nodes.iter().any(|n|matches!(&n.kind,NodeKind::Function(f) if f.result_pos==rustj::semantic::FunctionPartOfSpeech::Adverb)));
    assert!(
        handoff.lower_name_effects().is_err(),
        "modifier snapshot admission is still separate"
    );
}

#[test]
fn forged_assignment_names_copula_flags_and_abandon_rules_are_rejected() {
    let mut simple = parse_frontend("a=:1").unwrap();
    simple.assignment = Some("other".into());
    assert_eq!(
        VerifiedFrontend::from_program(simple)
            .unwrap_err()
            .category(),
        FailureCategory::VerifierDefect
    );
    let mut noun = parse_frontend("'a b'=:1 2").unwrap();
    noun.noun_assignment.as_mut().unwrap().names.swap(0, 1);
    assert_eq!(
        VerifiedFrontend::from_program(noun).unwrap_err().category(),
        FailureCategory::VerifierDefect
    );
    let mut flags = parse_frontend("a=:1").unwrap();
    flags
        .assignment_source
        .as_mut()
        .unwrap()
        .flags
        .local_assignment = true;
    assert_eq!(
        VerifiedFrontend::from_program(flags)
            .unwrap_err()
            .category(),
        FailureCategory::VerifierDefect
    );
    let mut take = parse_frontend("a_:+1").unwrap();
    if let ExprKind::Dyad { left, .. } = &mut take.expression.as_mut().unwrap().kind {
        if let ExprKind::TakeName { single_word, .. } = &mut left.kind {
            *single_word = true;
        } else {
            panic!("take");
        }
    } else {
        panic!("dyad");
    }
    assert_eq!(
        VerifiedFrontend::from_program(take).unwrap_err().category(),
        FailureCategory::VerifierDefect
    );
}

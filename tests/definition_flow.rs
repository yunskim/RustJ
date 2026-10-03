use rustj::{
    Engine,
    definition_code::DefinitionCode,
    definition_control::ControlWord as W,
    definition_flow::{ControlJump as J, ControlKind as K},
    semantic::{self, ExprKind, FunctionHead},
};
use std::sync::Arc;
fn code(body: &str) -> Arc<DefinitionCode> {
    let source = format!("f=:3 : '{}'", body.replace('\'', "''"));
    let expression = semantic::parse(&source).unwrap().expression.unwrap();
    let ExprKind::VerbValue(verb) = expression.kind else {
        panic!()
    };
    let FunctionHead::ExplicitDefinition(code) = &verb.entity.head else {
        panic!()
    };
    code.clone()
}
#[test]
fn branch_chain_targets_and_test_blocks_match_conend() {
    let code = code("if. a do. 1 elseif. b do. 2 else. 3 end.");
    let nodes = &code.monad_controls;
    assert_eq!(nodes.len(), 11);
    assert_eq!(nodes[1].kind, K::Test);
    assert_eq!(nodes[5].kind, K::Test);
    for (i, target) in [(2, 5), (4, 11), (6, 9), (8, 11), (10, 11)] {
        assert_eq!(nodes[i].go, J::Index(target));
    }
    assert_eq!(nodes[3].kind, K::Body);
    assert!(code.dyad_controls.is_empty());
}
#[test]
fn nested_loop_branchouts_are_owned_by_the_innermost_loop() {
    let code = code("while. y do. for_i. y do. continue. break. end. break. end.");
    let n = &code.monad_controls;
    assert_eq!(n.len(), 11);
    assert_eq!(n[5].kind, K::DoFor);
    assert_eq!(n[6].go, J::Index(5));
    assert_eq!(n[7].kind, K::BreakFor);
    assert_eq!(n[7].go, J::Index(9));
    assert_eq!(n[8].go, J::Index(5));
    assert_eq!(n[9].kind, K::Word(W::Break));
    assert_eq!(n[9].go, J::Index(11));
    assert_eq!(n[10].go, J::Index(1));
}
#[test]
fn whilst_initial_entry_and_assertion_origin_are_preserved() {
    let f = code("whilst. y do. y end.");
    assert_eq!(f.monad_controls[0].go, J::Index(3));
    assert_eq!(f.monad_controls[4].go, J::Index(1));
    let a = code("assert.\nNB. ignored\ny\n1 return. throw.");
    assert_eq!(a.monad_controls[0].kind, K::Assert);
    assert_eq!(a.monad_controls[0].line, 2);
    assert_eq!(a.monad_controls[0].assertion, Some(0..7));
    assert_eq!(a.monad_controls[2].go, J::Return);
    assert_eq!(a.monad_controls[3].go, J::DynamicError);
}
#[test]
fn independent_valence_audits_and_discarded_mode_four_source() {
    let f = code("if. y do. 1 end.\n:\nwhile. x do. break. end.");
    assert_eq!(f.monad_controls.len(), 5);
    assert_eq!(f.dyad_controls.len(), 5);
    let mut e = Engine::new();
    e.eval("f=:4 : 'if.\n:\nx+y'").unwrap();
    assert!(e.eval("f 1").is_err()); // Code construction is separate from invocation.
    assert_eq!(
        e.eval("f=:3 : 'if.\n:\ny'").unwrap_err().kind(),
        "control error"
    );
}
#[test]
fn malformed_controls_preserve_bindings_and_body_is_never_executed() {
    let mut e = Engine::new();
    e.eval("counter=:0").unwrap();
    e.eval("f=:3 : 'if. y do. counter=:99 else. counter=:7 end.'")
        .unwrap();
    assert_eq!(e.eval("counter").unwrap().unwrap().int_at(0).unwrap(), 0);
    let version = e.binding_version("f");
    for body in [
        "if.",
        "if. y end.",
        "do.",
        "else.",
        "end.",
        "break.",
        "continue.",
        "assert.",
        "assert. do.",
        "assert. assert. y",
        "if. y do. else. else. end.",
    ] {
        let source = format!("f=:3 : '{body}'");
        let report = e.eval_captured(&source);
        assert_eq!(report.result.unwrap_err().kind(), "control error", "{body}");
        report.capture.verify().unwrap();
        assert_eq!(e.binding_version("f"), version);
    }
}
#[test]
fn enqueue_errors_precede_control_audits_and_targets_are_verified() {
    let mut e = Engine::new();
    assert_eq!(
        e.eval("f=:3 : 'if. y do. 1 2e'").unwrap_err().kind(),
        "ill-formed number"
    );
    let f = code("if. y do. 1 end.");
    let mut nodes = f.monad_controls.clone();
    nodes[0].go = J::Index(999);
    assert!(rustj::definition_flow::verify(&nodes).is_err());
}

#[test]
fn try_handler_chain_and_inner_error_targets_match_conendtry() {
    let f = code("try. y catchd. 2 catch. 1 catcht. 3 end.");
    let n = &f.monad_controls;
    assert_eq!(n[0].go, J::Index(2));
    assert_eq!(n[2].go, J::Index(4));
    assert_eq!(n[4].go, J::Index(6));
    assert_eq!(n[6].go, J::Index(8));
    assert_eq!(n[1].go, J::Index(5));
    let f = code("try. try. y catchd. 1 end. throw. catch. 2 end.");
    let n = &f.monad_controls;
    assert_eq!(n[2].go, J::Index(4)); // Inner error remains assigned to the inner handler.
    assert_eq!(n[6].go, J::Index(8));
    let mut e = Engine::new();
    for body in [
        "try. end.",
        "catch. end.",
        "try. catch. catch. end.",
        "try. if. y catch. end.",
    ] {
        assert_eq!(
            e.eval(&format!("f=:3 : '{body}'")).unwrap_err().kind(),
            "control error",
            "{body}"
        );
    }
}

#[test]
fn static_construction_keeps_local_words_and_does_not_commit_a_binding() {
    let mut e = Engine::new();
    e.eval("counter=:0").unwrap();
    let source =
        "candidate=:3 : 'try. if. future do. copy=.y else. counter=:99 end. catcht. throw. end.'";
    let before = e.binding_version("counter");
    let bound = e.prepare_semantic(source).unwrap();
    assert!(e.binding_version("candidate").is_none());
    assert!(e.binding_version("copy").is_none());
    assert!(e.binding_version("future").is_none());
    assert_eq!(e.binding_version("counter"), before);
    let ExprKind::VerbValue(verb) = bound.program.expression.unwrap().kind else {
        panic!()
    };
    let FunctionHead::ExplicitDefinition(code) = &verb.entity.head else {
        panic!()
    };
    assert!(
        code.sentences[0]
            .words
            .iter()
            .any(|word| word.flags.local_assignment)
    );
    assert!(
        code.monad_controls
            .iter()
            .any(|node| node.kind == K::Word(W::Throw) && node.go == J::DynamicError)
    );
}

#[test]
fn control_entry_limit_preserves_the_old_function() {
    let mut e = Engine::new();
    e.eval("f=:+").unwrap();
    let before = e.binding_version("f");
    let source = format!("f=:3 : '{}'", "return. ".repeat(32766));
    assert_eq!(e.eval(&source).unwrap_err().kind(), "limit error");
    assert_eq!(e.binding_version("f"), before);
}

#[test]
fn code_verifier_rejects_dangling_source_word_and_valence_references() {
    let f = code("if. y do. 1 end.\n:\nx+y");
    f.verify().unwrap();
    let mut corrupt = (*f).clone();
    corrupt.monad_controls[1].words = 0..999;
    assert!(corrupt.verify().is_err());
    let mut corrupt = (*f).clone();
    corrupt.monad_controls[0].line = 999;
    assert!(corrupt.verify().is_err());
    let mut corrupt = (*f).clone();
    corrupt.dyad = 0..999;
    assert!(corrupt.verify().is_err());
}

#[test]
fn select_case_targets_fcase_trampoline_and_cleanup_tags() {
    let f = code("select. y case. 1 do. 2 fcase. 3 do. 4 case. 5 do. 6 end.");
    let n = &f.monad_controls;
    assert_eq!(n[0].go, J::Index(14));
    assert_eq!(n[2].go, J::Index(3));
    assert_eq!(n[4].go, J::Index(7));
    assert_eq!(n[6].go, J::Index(14));
    assert_eq!(n[10].go, J::Index(13));
    assert_eq!(n[14].kind, K::EndSelect);
    let f = code("while. y do. select. y case. 1 do. break. 0 end. end.");
    assert!(
        f.monad_controls
            .iter()
            .any(|node| node.kind == K::BreakSelect && node.go == J::Index(f.monad_controls.len()))
    );
    let f = code(
        "while. y do. select. y case. 1 do. select. z case. 2 do. continue. 0 end. 0 end. end.",
    );
    assert!(
        f.monad_controls
            .iter()
            .any(|node| node.kind == K::SelectNested)
    );
    assert!(
        f.monad_controls
            .iter()
            .any(|node| node.kind == K::ContinueSelect && node.go == J::Index(1))
    );
}

#[test]
fn packed_control_end_acceptance_matches_c_without_structured_graph_assumptions() {
    for body in [
        "while. if. end.",
        "while. while. end.",
        "while. whilst. end.",
        "while. for. end.",
    ] {
        let f = code(body);
        assert!(f.monad_controls.last().unwrap().analysis_barrier, "{body}");
        assert_eq!(f.monad_controls.last().unwrap().go, J::Index(1));
        f.verify().unwrap();
    }
    assert!(
        code("while. y do. y end.")
            .monad_controls
            .iter()
            .all(|node| !node.analysis_barrier)
    );
}

#[test]
fn noncanonical_packed_pair_does_not_underflow_loop_bookkeeping() {
    let mut e = Engine::new();
    assert_eq!(
        e.eval("f=:3 : 'if. do. else. if. end.'")
            .unwrap_err()
            .kind(),
        "control error"
    );
}

#[test]
fn monad_audit_precedes_dyad_enqueue_errors() {
    let mut e = Engine::new();
    assert_eq!(
        e.eval("f=:3 : 'if.\n:\n1 2e'").unwrap_err().kind(),
        "control error"
    );
    assert_eq!(
        e.eval("f=:3 : '1 2e\n:\nif.'").unwrap_err().kind(),
        "ill-formed number"
    );
}

#[test]
fn previous_result_status_distinguishes_test_values_from_body_values() {
    use rustj::definition_flow::PreviousResult::{CanReturn, CannotReturn, Unresolved};
    let f = code("1 while. y do. 2 end.");
    let states: Vec<_> = f.monad_controls.iter().map(|n| n.previous_result).collect();
    assert_eq!(
        states,
        vec![
            CannotReturn,
            CanReturn,
            CanReturn,
            CanReturn,
            CannotReturn,
            CanReturn
        ]
    );
    // A later body overwrites the earlier value on both loop successors.
    let f = code("while. y do. 2 end. 3");
    assert!(
        f.monad_controls
            .iter()
            .all(|n| n.previous_result == CannotReturn)
    );
    // Branches which preserve and overwrite the previous value do not agree.
    let f = code("1 if. y do. 2 end. return.");
    assert_eq!(f.monad_controls[2].previous_result, Unresolved);
    assert_eq!(f.monad_controls[3].previous_result, Unresolved);
    assert_eq!(f.monad_controls.last().unwrap().previous_result, CanReturn);
    let f = code("assert. y");
    assert_eq!(f.monad_controls[0].previous_result, CanReturn);
    let f = code("throw.");
    assert_eq!(f.monad_controls[0].previous_result, CannotReturn);
}

#[test]
fn fallthrough_end_metadata_observes_select_loop_and_final_end_boundaries() {
    let f = code("if. y do. y end. 3");
    assert!(f.monad_controls[3].before_fallthrough_end);
    let f = code("if. y do. y end.");
    assert!(!f.monad_controls[3].before_fallthrough_end);
    let f = code("while. y do. y end. 3");
    assert!(!f.monad_controls[3].before_fallthrough_end);
    let f = code("select. y case. 1 do. y end. 3");
    assert!(f.monad_controls.iter().all(|n| !n.before_fallthrough_end));
    let f = code("try. y catch. y end. 3");
    assert!(f.monad_controls[3].before_fallthrough_end);
    let f = code("if. y do. assert. y end. 3");
    assert!(f.monad_controls.iter().all(|n| !n.before_fallthrough_end));
}

#[test]
fn previous_result_metadata_is_separate_for_each_valence_and_analysis_barrier() {
    use rustj::definition_flow::PreviousResult::{CanReturn, CannotReturn};
    let f = code("return.\n:\nx+y");
    assert_eq!(f.monad_controls[0].previous_result, CanReturn);
    assert_eq!(f.dyad_controls[0].previous_result, CannotReturn);
    let f = code("while. if. end.");
    assert!(f.monad_controls.last().unwrap().analysis_barrier);
    assert_eq!(f.monad_controls.last().unwrap().previous_result, CanReturn);
}

#[test]
fn verifier_rejects_fallthrough_marker_on_test_or_final_body() {
    let f = code("if. y do. y end. 3");
    let mut invalid = (*f).clone();
    invalid.monad_controls[1].before_fallthrough_end = true;
    assert!(invalid.verify().is_err());
    let mut invalid = (*f).clone();
    invalid
        .monad_controls
        .last_mut()
        .unwrap()
        .before_fallthrough_end = true;
    assert!(invalid.verify().is_err());
}

#[test]
fn goto_targets_use_label_successor_and_exact_raw_suffix() {
    let f = code("goto_exit. 1 label_exitt. 2 label_exit. y");
    assert_eq!(f.monad_controls[0].named_target.as_deref(), Some("exit"));
    assert_eq!(f.monad_controls[0].go, J::Index(5));
    let f = code("label_again. y goto_again.");
    assert_eq!(f.monad_controls[2].go, J::Index(1));
    for body in ["goto_. label_. y", "goto_1x. label_1x. y"] {
        assert_eq!(code(body).monad_controls[0].go, J::Index(2));
    }
    let f = code("label_same. label_same. y");
    assert_eq!(f.monad_controls.len(), 3); // duplicates are checked only if referenced
}

#[test]
fn goto_audit_rejects_missing_duplicate_and_cross_valence_targets_transactionally() {
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    let version = engine.binding_version("f");
    for body in [
        "goto_missing.",
        "goto_x. label_xx.",
        "goto_a. label_a. label_a.",
        "goto_a.\n:\nlabel_a.",
    ] {
        let source = format!("f=:3 : '{body}'");
        assert_eq!(
            engine.eval(&source).unwrap_err().kind(),
            "control error",
            "{body}"
        );
        assert_eq!(engine.binding_version("f"), version);
    }
    let f = code("goto_a. label_a.\n:\ngoto_a. label_a.");
    assert_eq!(f.monad_controls[0].go, J::Index(2));
    assert_eq!(f.dyad_controls[0].go, J::Index(2));
}

#[test]
fn goto_cannot_enter_a_structure_or_sibling_branch_but_can_leave() {
    let mut e = Engine::new();
    for body in [
        "goto_a. if. y do. label_a. y end.",
        "if. y do. goto_a. else. label_a. y end.",
        "goto_a. while. y do. label_a. y end.",
        "goto_a. select. y case. 1 do. label_a. y end.",
    ] {
        assert_eq!(
            e.eval(&format!("f=:3 : '{body}'")).unwrap_err().kind(),
            "control error",
            "{body}"
        );
    }
    for body in [
        "if. y do. goto_a. end. label_a. y",
        "while. y do. goto_a. end. label_a. y",
        "if. y do. goto_a. label_a. y end.",
        "try. y catch. goto_a. label_a. y end.",
    ] {
        code(body).verify().unwrap();
    }
}

#[test]
fn named_control_verifier_and_duplicate_error_preserve_source_provenance() {
    let f = code("goto_a. label_a. y");
    let mut corrupted = (*f).clone();
    corrupted.monad_controls[0].named_target = Some("wrong".into());
    assert!(corrupted.verify().is_err());
    let mut corrupted = (*f).clone();
    corrupted.monad_controls[0].go = J::Index(1);
    assert!(corrupted.verify().is_err());
    let source = "f=:3 : '''quoted''\ngoto_a.\nlabel_a.\nlabel_a.'";
    let mut e = Engine::new();
    let error = e.eval_diagnostic(source).unwrap_err();
    let start = source.rfind("label_a.").unwrap();
    assert_eq!(error.kind(), "control error");
    assert_eq!(error.span(), Some(&(start..start + 8)));
}

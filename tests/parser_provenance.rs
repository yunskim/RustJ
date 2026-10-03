use rustj::parser::{ParseClass, ParseRow, parse_diagnostic};

#[test]
fn all_nine_rows_preserve_original_word_coverage_and_inherited_tokens() {
    let cases = [
        ("+1", ParseRow::MonadEdge, 0..2, 1),
        ("+ - 1", ParseRow::MonadVVN, 1..3, 2),
        ("1+2*3", ParseRow::DyadNVN, 2..5, 4),
        ("+/", ParseRow::Adverb, 0..2, 0),
        ("+\"1", ParseRow::Conjunction, 0..3, 0),
        ("(+/ % #)", ParseRow::Fork, 1..5, 1),
        ("(+ #)", ParseRow::Hook, 1..3, 1),
        ("out=:1+2", ParseRow::Assignment, 0..5, 4),
        ("(+/ % #)", ParseRow::Parenthesis, 0..6, 0),
    ];
    for (source, row, coverage, blame) in cases {
        let program = parse_diagnostic(source).unwrap();
        let reduction = program.reductions.iter().find(|r| r.row == row).unwrap();
        assert_eq!(reduction.result.word_range, coverage, "{source}");
        assert_eq!(reduction.result.blame_word_index, blame, "{source}");
        let words = rustj::enqueuer::enqueue(source).unwrap();
        assert_eq!(
            reduction.span,
            words[coverage.start].span.start..words[coverage.end - 1].span.end
        );
        assert_eq!(
            reduction.inputs.first().unwrap().word_range.start,
            coverage.start
        );
        assert_eq!(
            reduction.inputs.last().unwrap().word_range.end,
            coverage.end
        );
        for part in &reduction.inputs {
            assert!(part.word_range.contains(&part.blame_word_index));
        }
    }
}

#[test]
fn modifier_fork_and_parenthesis_keep_distinct_completed_origins() {
    let p = parse_diagnostic("(+/ % #)").unwrap();
    assert_eq!(
        p.reductions.iter().map(|r| r.row).collect::<Vec<_>>(),
        vec![ParseRow::Adverb, ParseRow::Fork, ParseRow::Parenthesis]
    );
    assert_eq!(p.reductions[1].inputs[0], p.reductions[0].result);
    assert_eq!(p.reductions[2].inputs[1], p.reductions[1].result);
    assert!(
        p.reductions
            .iter()
            .all(|r| r.result_class == ParseClass::Verb)
    );
    assert_eq!(p.reductions[2].result.blame_word_index, 0);
    assert_eq!(p.reductions[1].result.blame_word_index, 1);
}

#[test]
fn assignment_preserves_copula_and_rhs_origin_without_reclassifying_them() {
    for source in ["out=:1+2", "out=.1+2"] {
        let p = parse_diagnostic(source).unwrap();
        let assignment = p.assignment_source.unwrap();
        assert_eq!(assignment.target.word_range, 0..1);
        assert_eq!(assignment.copula.word_range, 1..2);
        assert!(assignment.flags.global_assignment);
        assert!(!assignment.flags.local_assignment); // jtenqueue's top-level contract
        assert!(assignment.flags.assignment_to_name);
        let last = p.reductions.last().unwrap();
        assert_eq!(last.row, ParseRow::Assignment);
        assert_eq!(last.inputs[2], p.reductions[0].result);
        assert_eq!(last.result.blame_word_index, 4);
    }
}

#[test]
fn constructor_error_blames_operator_after_inner_reduction() {
    let source = "(+/)\"'bad'";
    let error = parse_diagnostic(source).unwrap_err();
    assert_eq!(error.kind(), "domain error");
    assert_eq!(error.context().unwrap().blame_word_index, Some(4));
    assert_eq!(
        error.context().unwrap().phase,
        Some(rustj::error::DiagnosticPhase::Parse)
    );
}

#[test]
fn unmatched_parentheses_blame_the_original_control_word() {
    for (source, index, span) in [("1 ) + 2", 1, 2..3), ("1 + (2", 2, 4..5)] {
        let error = parse_diagnostic(source).unwrap_err();
        assert_eq!(error.kind(), "syntax error");
        assert_eq!(error.context().unwrap().blame_word_index, Some(index));
        assert_eq!(error.span(), Some(&span));
    }
}

#[test]
fn named_insert_in_fork_retains_structure_despite_runtime_coverage_boundary() {
    let mut engine = rustj::Engine::new();
    engine.eval("entryverb=:+").unwrap();
    engine.eval("entrynoun=:1 2 3").unwrap();
    let source = "(entryverb/ % #) entrynoun";
    let bound = engine.prepare_semantic(source).unwrap();
    assert!(
        bound
            .program
            .reductions
            .iter()
            .any(|r| r.row == ParseRow::Adverb)
    );
    assert!(
        bound
            .program
            .reductions
            .iter()
            .any(|r| r.row == ParseRow::Fork)
    );
    assert!(
        bound
            .verb_references
            .iter()
            .any(|(name, _)| name == "entryverb")
    );
    let graph = engine.analyze_j_graph(source).unwrap();
    graph.verify().unwrap();
    assert!(
        graph
            .regions
            .iter()
            .any(|r| matches!(r.kind, rustj::j_graph_ir::RegionKind::Fork { .. }))
    );
    assert_eq!(engine.eval(source).unwrap_err().kind(), "unsupported");
}

use rustj::{
    Error,
    facts::TypeFact,
    j_graph_ir::{GraphFacts, NodeKind, RegionKind},
    semantic::FunctionPartOfSpeech,
    static_analysis::{BoundaryReason, StaticAnalyzer},
    types::DType,
};
fn input(shape: Option<Vec<usize>>) -> GraphFacts {
    GraphFacts {
        dtype: TypeFact::Exact(DType::Int),
        rank: shape.as_ref().map(Vec::len),
        shape,
    }
}

#[test]
fn computed_constructor_operand_stops_analysis_without_committing_a_target() {
    let mut analyzer = StaticAnalyzer::new();
    analyzer
        .declare_noun("data", input(Some(vec![2, 3])))
        .unwrap();
    let original = analyzer.binding("data").unwrap().clone();
    analyzer.analyze("out=:(+/\"1) data").unwrap();
    assert_eq!(
        analyzer
            .analyze("out=:(+/\"(1+0)) data")
            .unwrap_err()
            .kind(),
        "unsupported"
    );
    assert!(analyzer.binding("out").is_none());
    assert_eq!(analyzer.binding("data").unwrap(), &original);
    let value = rustj::Engine::new()
        .eval("(+/\"(1+0)) i.2 3")
        .unwrap()
        .unwrap();
    assert_eq!(value.shape(), &[2]);
    assert_eq!(value.int_at(0).unwrap(), 3);
    assert_eq!(value.int_at(1).unwrap(), 12);
}

#[test]
fn nonfinal_write_stops_analysis_before_reusing_a_stale_name_read() {
    let mut analyzer = StaticAnalyzer::new();
    analyzer.declare_noun("a", input(Some(vec![]))).unwrap();
    let original = analyzer.binding("a").unwrap().clone();
    assert_eq!(
        analyzer.analyze("a+a=:2").unwrap_err().kind(),
        "unsupported"
    );
    assert_eq!(analyzer.binding("a").unwrap(), &original);
    let mut engine = rustj::Engine::new();
    engine.eval("a=:1").unwrap();
    assert_eq!(
        engine.eval("a+a=:2").unwrap().unwrap().int_at(0).unwrap(),
        4
    );
    assert_eq!(engine.eval("a").unwrap().unwrap().int_at(0).unwrap(), 2);
}

#[test]
fn unknown_result_shape_keeps_a_deferred_call_instead_of_demanding_atoms() {
    let mut analyzer = StaticAnalyzer::new();
    analyzer.declare_noun("n", input(Some(vec![]))).unwrap();
    let report = analyzer.analyze("out=:i.n").unwrap();
    report.graph.verify().unwrap();
    assert!(analyzer.binding("out").is_none());
    assert!(
        report
            .graph
            .nodes
            .iter()
            .any(|node| matches!(node.kind, NodeKind::ReadNoun { .. }))
    );
    assert!(
        report
            .graph
            .nodes
            .iter()
            .any(|node| matches!(node.kind, NodeKind::Apply { .. }))
    );
    assert!(
        report
            .graph
            .nodes
            .iter()
            .all(|node| !matches!(node.kind, NodeKind::Literal(_)))
    );
}
#[test]
fn trillion_atom_inputs_require_no_data_values_and_preserve_graph_edges() {
    let mut a = StaticAnalyzer::new();
    for name in ["x", "y", "z"] {
        a.declare_noun(name, input(Some(vec![1_000_000_000_000])))
            .unwrap();
    }
    let r = a.analyze("x+y*z").unwrap();
    r.graph.verify().unwrap();
    assert_eq!(r.inputs.len(), 3);
    assert_eq!(r.graph.nodes.len(), 5);
    assert!(
        r.graph
            .nodes
            .iter()
            .all(|node| !matches!(node.kind, NodeKind::Literal(_)))
    );
    let root = r.graph.result.unwrap();
    assert_eq!(r.memory.extent(root).unwrap().atoms, 1_000_000_000_000);
    let NodeKind::Apply { right, .. } = r.graph.nodes[root.0].kind else {
        panic!()
    };
    assert!(matches!(
        r.graph.nodes[right.0].kind,
        NodeKind::Apply { .. }
    ));
    let grouped = a.analyze("(x+y)*z").unwrap();
    let NodeKind::Apply {
        left: Some(left), ..
    } = grouped.graph.nodes[grouped.graph.result.unwrap().0].kind
    else {
        panic!()
    };
    assert!(matches!(
        grouped.graph.nodes[left.0].kind,
        NodeKind::Apply { .. }
    ));
}
#[test]
fn unknown_extents_are_not_zero_and_declared_functions_remain_dynamic() {
    let mut a = StaticAnalyzer::new();
    a.declare_noun("x", input(None)).unwrap();
    let r = a.analyze(",x").unwrap();
    assert!(r.memory.extent(r.graph.result.unwrap()).is_none());
    assert!(!r.boundaries.is_empty());
    a.declare_function("f", FunctionPartOfSpeech::Verb).unwrap();
    let r = a.analyze("f x").unwrap();
    assert!(
        r.boundaries
            .iter()
            .any(|b| b.reason == BoundaryReason::FunctionSpecialization)
    );
}
#[test]
fn analysis_does_not_commit_assignment_or_invoke_erroring_call() {
    let mut a = StaticAnalyzer::new();
    a.declare_noun("x", input(Some(vec![3]))).unwrap();
    let version = a.binding("x").unwrap().version;
    let r = a.analyze("result=:x+'a'").unwrap();
    assert_eq!(r.graph.write.unwrap().name, "result");
    assert!(a.binding("result").is_none());
    assert_eq!(a.binding("x").unwrap().version, version);
    assert_eq!(
        rustj::Engine::new().eval("1+'a'").unwrap_err().kind(),
        "domain error"
    );
    assert!(a.analyze("1+'a'").is_ok());
}
#[test]
fn fork_structure_survives_without_an_input_array() {
    let mut a = StaticAnalyzer::new();
    a.declare_noun("x", input(Some(vec![4]))).unwrap();
    let r = a.analyze("(+/ % #) x").unwrap();
    assert!(
        r.graph
            .regions
            .iter()
            .any(|region| matches!(region.kind, RegionKind::Fork { .. }))
    );
    assert_eq!(
        r.memory.extent(r.graph.result.unwrap()).unwrap().shape,
        Vec::<usize>::new()
    );
}
#[test]
fn missing_pos_and_value_dependent_constructors_fail_as_analysis_coverage() {
    let mut a = StaticAnalyzer::new();
    let error = a.analyze("missing+1").unwrap_err();
    assert_eq!(error.kind(), "unsupported");
    assert_eq!(error.context().unwrap().blame_word_index, Some(0));
    a.declare_noun("r", input(Some(vec![]))).unwrap();
    assert_eq!(a.analyze("+\"r").unwrap_err().kind(), "unsupported");
    assert!(a.analyze("+\"1").is_ok());
}
#[test]
fn catalog_rejects_inconsistent_facts_without_replacing_a_binding() {
    let mut a = StaticAnalyzer::new();
    a.declare_noun("x", input(Some(vec![2]))).unwrap();
    let original = a.binding("x").unwrap().clone();
    let mut bad = input(Some(vec![2]));
    bad.rank = Some(2);
    assert_eq!(a.declare_noun("x", bad).unwrap_err(), Error::Rank);
    assert_eq!(a.binding("x").unwrap(), &original);
    assert!(a.declare_noun("+", input(None)).is_err());
}

#[test]
fn frontend_provenance_survives_reduction_without_assignment_execution() {
    let mut a = StaticAnalyzer::new();
    a.declare_noun("x", input(Some(vec![3]))).unwrap();
    let source = "out=: (x+x) NB. ignored";
    let r = a.analyze(source).unwrap();
    let queue = rustj::enqueuer::enqueue(source).unwrap();
    assert_eq!(r.source_words.len(), queue.len());
    for (saved, word) in r.source_words.iter().zip(queue) {
        assert_eq!(saved.span, word.span);
        assert_eq!(saved.word_index, word.word_index);
        assert_eq!(saved.class, word.class);
        assert_eq!(saved.flags, word.flags);
    }
    assert!(!r.source_words[0].flags.lookup_name);
    assert!(r.source_words[1].flags.global_assignment);
    assert!(r.source_words[1].flags.assignment_to_name);
    assert!(
        r.source_words
            .iter()
            .all(|w| !source[w.span.clone()].starts_with("NB."))
    );
    assert!(
        r.assignment_source
            .as_ref()
            .unwrap()
            .flags
            .global_assignment
    );
    assert_eq!(
        r.reductions.last().unwrap().row,
        rustj::parser::ParseRow::Assignment
    );
    assert_eq!(
        r.reductions.last().unwrap().result.word_range,
        0..r.source_words.len()
    );
    assert!(a.binding("out").is_none());
    assert!(
        r.graph
            .nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Apply { .. }))
    );
}

#[test]
fn modifier_pos_alone_does_not_prove_the_application_result_pos() {
    let mut analyzer = StaticAnalyzer::new();
    analyzer.declare_noun("x", input(Some(vec![3]))).unwrap();
    analyzer
        .declare_function("adv", FunctionPartOfSpeech::Adverb)
        .unwrap();
    analyzer
        .declare_function("conj", FunctionPartOfSpeech::Conjunction)
        .unwrap();
    for (source, name) in [("out=: + adv x", "adv"), ("out=: + conj 1 x", "conj")] {
        let version = analyzer.binding(name).unwrap().version;
        let error = analyzer.analyze(source).unwrap_err();
        assert_eq!(error.kind(), "unsupported");
        assert_eq!(error.context().unwrap().current_name.as_deref(), Some(name));
        assert_eq!(error.context().unwrap().blame_word_index, Some(3));
        assert_eq!(&source[error.span().unwrap().clone()], name);
        assert_eq!(analyzer.binding(name).unwrap().version, version);
        assert!(analyzer.binding("out").is_none());
    }
}

#[test]
fn known_modifiers_build_static_structure_and_memory_from_metadata_only() {
    use rustj::primitive::{AdverbId, ConjunctionId, PrimitiveSemanticId};
    let mut analyzer = StaticAnalyzer::new();
    analyzer
        .declare_noun("x", input(Some(vec![1_000_000_000_000])))
        .unwrap();
    analyzer
        .declare_primitive_modifier("fold", PrimitiveSemanticId::Adverb(AdverbId::Insert))
        .unwrap();
    let report = analyzer.analyze("out=: + fold x").unwrap();
    report.graph.verify().unwrap();
    let root = report.graph.result.unwrap();
    assert_eq!(
        report.memory.extent(root).unwrap().shape,
        Vec::<usize>::new()
    );
    let NodeKind::Apply { function, .. } = &report.graph.nodes[root.0].kind else {
        panic!()
    };
    assert_eq!(
        function.head,
        rustj::semantic::FunctionHead::PrimitiveAdverb(AdverbId::Insert)
    );
    let snapshot = &report.graph.modifier_snapshots[0];
    assert_eq!(snapshot.name, "fold");
    assert_eq!(snapshot.version, analyzer.binding("fold").unwrap().version);
    assert_eq!(&report.graph.source[snapshot.span.clone()], "fold");
    assert!(report.graph.verb_references.is_empty());
    assert!(analyzer.binding("out").is_none());
    analyzer
        .declare_primitive_modifier(
            "ranked",
            PrimitiveSemanticId::Conjunction(ConjunctionId::Rank),
        )
        .unwrap();
    let report = analyzer.analyze("+ ranked 1 x").unwrap();
    assert_eq!(
        report
            .memory
            .extent(report.graph.result.unwrap())
            .unwrap()
            .shape,
        vec![1_000_000_000_000]
    );
    assert_eq!(
        report.graph.modifier_snapshots[0].expected,
        FunctionPartOfSpeech::Conjunction
    );
}

#[test]
fn known_constructor_errors_and_value_boundaries_are_preserved_without_execution() {
    use rustj::primitive::{AdverbId, ConjunctionId, PrimitiveSemanticId};
    let mut analyzer = StaticAnalyzer::new();
    analyzer.declare_noun("x", input(Some(vec![3]))).unwrap();
    analyzer
        .declare_primitive_modifier("fold", PrimitiveSemanticId::Adverb(AdverbId::Insert))
        .unwrap();
    analyzer
        .declare_primitive_modifier(
            "ranked",
            PrimitiveSemanticId::Conjunction(ConjunctionId::Rank),
        )
        .unwrap();
    for (source, error) in [
        ("3 fold", "domain error"),
        ("+ ranked 'a'", "domain error"),
        ("+ ranked 1 2 3 4", "length error"),
        ("+ ranked (#x)", "unsupported"),
    ] {
        assert_eq!(
            analyzer.analyze(source).unwrap_err().kind(),
            error,
            "{source}"
        );
    }
    assert!(analyzer.analyze("+ ranked (#1 2)").is_err()); // no hidden evaluation
}

#[test]
fn invalid_modifier_declaration_does_not_replace_the_existing_binding() {
    use rustj::primitive::{AdverbId, PrimitiveId, PrimitiveSemanticId};
    let mut analyzer = StaticAnalyzer::new();
    analyzer
        .declare_primitive_modifier("fold", PrimitiveSemanticId::Adverb(AdverbId::Insert))
        .unwrap();
    let previous = analyzer.binding("fold").unwrap().clone();
    assert_eq!(
        analyzer
            .declare_primitive_modifier("fold", PrimitiveSemanticId::Verb(PrimitiveId::Add))
            .unwrap_err(),
        Error::Domain
    );
    assert_eq!(analyzer.binding("fold").unwrap(), &previous);
    assert!(
        analyzer
            .declare_primitive_modifier("+", PrimitiveSemanticId::Adverb(AdverbId::Insert))
            .is_err()
    );
    assert_eq!(analyzer.binding("fold").unwrap(), &previous);
}

#[test]
fn snapshot_identity_survives_engine_rebinding_and_analysis_does_not_assign() {
    use rustj::{Engine, parser_capture::CaptureEvent};
    let mut engine = Engine::new();
    for source in ["adv=:/", "alias=:adv", "x=:i.4"] {
        engine.eval(source).unwrap();
    }
    let source = "candidate=: + alias x";
    let graph = engine.analyze_j_graph(source).unwrap();
    graph.verify().unwrap();
    assert_eq!(engine.binding_version("candidate"), None);
    let snapshot = &graph.modifier_snapshots[0];
    assert_eq!(snapshot.name, "alias");
    let report = engine.eval_captured("+alias x");
    assert_eq!(report.result.unwrap().unwrap().int_at(0).unwrap(), 6);
    let function = report
        .capture
        .events
        .iter()
        .find_map(|event| match event {
            CaptureEvent::ModifierStacked { snapshot } => Some(&snapshot.function),
            _ => None,
        })
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(function, &snapshot.function));
    engine.eval("adv=:1").unwrap();
    let alias_graph = engine.analyze_j_graph(source).unwrap();
    assert_eq!(alias_graph.modifier_snapshots[0].version, snapshot.version);
    engine.eval("alias=:\\").unwrap();
    let changed = engine.analyze_j_graph(source).unwrap();
    assert_ne!(changed.modifier_snapshots[0].version, snapshot.version);
    let NodeKind::Apply { function: old, .. } = &graph.nodes[graph.result.unwrap().0].kind else {
        panic!()
    };
    let NodeKind::Apply { function: new, .. } = &changed.nodes[changed.result.unwrap().0].kind
    else {
        panic!()
    };
    assert_eq!(
        old.head,
        rustj::semantic::FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert)
    );
    assert_eq!(
        new.head,
        rustj::semantic::FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::PrefixInfix)
    );
    // The engine supplies noun metadata without evaluating the reduction.
    assert_eq!(
        graph
            .static_memory_analysis()
            .extent(graph.result.unwrap())
            .unwrap()
            .shape,
        Vec::<usize>::new()
    );
}

#[test]
fn graph_verifier_checks_snapshot_pos_version_and_source_use() {
    use rustj::primitive::{AdverbId, PrimitiveSemanticId};
    let mut analyzer = StaticAnalyzer::new();
    analyzer.declare_noun("x", input(Some(vec![3]))).unwrap();
    analyzer
        .declare_primitive_modifier("fold", PrimitiveSemanticId::Adverb(AdverbId::Insert))
        .unwrap();
    let report = analyzer.analyze("+fold x").unwrap();
    for mutation in 0..3 {
        let mut graph = report.graph.clone();
        let snapshot = &mut graph.modifier_snapshots[0];
        match mutation {
            0 => snapshot.span = 0..1,
            1 => snapshot.expected = FunctionPartOfSpeech::Verb,
            _ => snapshot.version = rustj::semantic::NameVersion(0),
        }
        assert!(graph.verify().is_err());
    }
}

#[test]
fn bound_rank_bident_can_analyze_trillion_atom_metadata_without_a_kernel() {
    let mut analyzer = StaticAnalyzer::new();
    analyzer
        .declare_noun("x", input(Some(vec![1_000_000_000_000])))
        .unwrap();
    let report = analyzer.analyze("out=: - (\"1) x").unwrap();
    report.graph.verify().unwrap();
    assert_eq!(
        report
            .memory
            .extent(report.graph.result.unwrap())
            .unwrap()
            .shape,
        vec![1_000_000_000_000]
    );
    assert!(analyzer.binding("out").is_none());
    assert!(
        report
            .reductions
            .iter()
            .any(|r| r.row == rustj::parser::ParseRow::Hook)
    );
    assert!(
        report
            .reductions
            .iter()
            .any(|r| r.row == rustj::parser::ParseRow::Adverb)
    );
}

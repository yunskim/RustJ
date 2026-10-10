use rustj::{
    Engine,
    facts::TypeFact,
    j_graph_ir::{GraphBasisKind, NodeKind},
    j_graph_scan::ScanBoundaryReason as B,
    types::DType,
};

#[test]
fn boolean_scan_candidates_preserve_source_and_c_short_cases() {
    for op in ["+", "*"] {
        for (noun, shape, dtype) in [
            ("0", vec![1], DType::Bool),
            (",1", vec![1], DType::Bool),
            ("0$0", vec![0], DType::Bool),
            ("2 0$0", vec![2, 0], DType::Bool),
            ("1 3$1", vec![1, 3], DType::Bool),
            (
                "2 3$0 1",
                vec![2, 3],
                if op == "+" { DType::Int } else { DType::Bool },
            ),
        ] {
            let mut engine = Engine::new();
            engine.eval(&format!("scaninput=: {noun}")).unwrap();
            let source = format!("({op}/)\\ scaninput");
            let graph = engine.analyze_j_graph(&source).unwrap();
            let analysis = graph.scan_analysis().unwrap();
            analysis.verify(&graph).unwrap();
            assert_eq!(analysis.candidates.len(), 1, "{source}: {analysis:?}");
            let candidate = &analysis.candidates[0];
            assert_eq!(candidate.basis.layers, vec![GraphBasisKind::Scan]);
            assert_eq!(candidate.contract.output_facts.shape, Some(shape));
            assert_eq!(
                candidate.contract.output_facts.dtype,
                TypeFact::Exact(dtype)
            );
            assert!(!candidate.contract.inject_identity);
            assert!(!candidate.contract.parallel_prefix_authorized);
            let NodeKind::Apply { basis, .. } = &graph.nodes[candidate.source.0].kind else {
                panic!()
            };
            assert_eq!(
                basis.layers,
                vec![GraphBasisKind::Window, GraphBasisKind::Reduce]
            );
            // Analysis recognition does not claim an executable prefix route.
            assert_eq!(engine.eval(&source).unwrap_err().kind(), "unsupported");
        }
    }
}

#[test]
fn arbitrary_prefix_infix_numeric_and_ranked_calls_remain_window() {
    let mut engine = Engine::new();
    engine.eval("scaninput=: 0 1 1 0").unwrap();
    engine.eval("scanreducer=: +").unwrap();
    for (source, reason) in [
        ("+\\ scaninput", B::NonPrimitiveInsert),
        ("2 (+/)\\ scaninput", B::DyadicInfix),
        ("(scanreducer/)\\ scaninput", B::NonPrimitiveInsert),
        ("(-/)\\ scaninput", B::UnprovenNumericOrErrorSemantics),
        ("(+/\\)\"1 scaninput", B::NestedCallRequiresRankFacts),
        ("(+/)\\ 2 3", B::UnknownOrNonBooleanInput),
        ("(+/)\\ 0.5 1.5", B::UnknownOrNonBooleanInput),
        ("(+/)\\ 'xy'", B::UnknownOrNonBooleanInput),
    ] {
        let graph = engine.analyze_j_graph(source).unwrap();
        let analysis = graph.scan_analysis().unwrap();
        assert!(analysis.candidates.is_empty(), "{source}");
        assert!(
            analysis.boundaries.iter().any(|b| b.reason == reason),
            "{source}: {analysis:?}"
        );
    }
}

#[test]
fn witness_verifier_rejects_stale_facts_outputs_and_parallel_permission() {
    let graph = Engine::new().analyze_j_graph("(+/)\\ 0 1 1 0").unwrap();
    let original = graph.scan_analysis().unwrap();
    assert_eq!(original.candidates.len(), 1);
    let mut forged = original.clone();
    forged.candidates[0].contract.parallel_prefix_authorized = true;
    assert!(forged.verify(&graph).is_err());
    let mut forged = original.clone();
    forged.candidates[0].contract.output_facts.dtype = TypeFact::Exact(DType::Bool);
    assert!(forged.verify(&graph).is_err());
    let mut forged = original.clone();
    forged.candidates[0].source_span = 0..0;
    assert!(forged.verify(&graph).is_err());
    let mut changed = graph.clone();
    changed.nodes[original.candidates[0].input.0].facts.dtype = TypeFact::Unknown;
    assert!(original.verify(&changed).is_err());
    assert!(changed.scan_analysis().unwrap().candidates.is_empty());
}

#[test]
fn missing_shape_and_unbounded_boolean_extent_do_not_gain_scan_witnesses() {
    let mut engine = Engine::new();
    engine.eval("scaninput=: 0 1").unwrap();
    let graph = engine.analyze_j_graph("(+/)\\ scaninput").unwrap();
    let input = graph.scan_analysis().unwrap().candidates[0].input;
    for (shape, rank, expected) in [
        (None, None, B::UnknownOrInconsistentShape),
        (Some(vec![2]), None, B::UnknownOrInconsistentShape),
        (
            Some(vec![i64::MAX as usize + 1]),
            Some(1),
            B::ExtentOverflow,
        ),
        (Some(vec![usize::MAX, 2]), Some(2), B::ExtentOverflow),
    ] {
        let mut unknown = graph.clone();
        unknown.nodes[input.0].facts.shape = shape;
        unknown.nodes[input.0].facts.rank = rank;
        let analysis = unknown.scan_analysis().unwrap();
        assert!(analysis.candidates.is_empty());
        assert_eq!(analysis.boundaries[0].reason, expected);
    }
}

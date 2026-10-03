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
    assert!(a.binding("out").is_none());
    assert!(
        r.graph
            .nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Apply { .. }))
    );
}

use rustj::{
    Data, Error, Value,
    facts::TypeFact,
    j_graph_ir::{GraphFacts, NodeKind},
    semantic::FunctionPartOfSpeech,
    static_analysis::StaticAnalyzer,
    storage::CpuStorage,
    types::DType,
};

fn declared(dtype: TypeFact, shape: Option<Vec<usize>>, rank: Option<usize>) -> GraphFacts {
    GraphFacts { dtype, shape, rank }
}

#[test]
fn one_analysis_accepts_different_batches_without_retaining_their_payloads() {
    let mut analyzer = StaticAnalyzer::new();
    analyzer
        .declare_noun(
            "X",
            declared(TypeFact::Exact(DType::Int), Some(vec![2, 3]), None),
        )
        .unwrap();
    let report = analyzer.analyze("X+X").unwrap();
    assert!(
        report
            .graph
            .nodes
            .iter()
            .all(|node| !matches!(node.kind, NodeKind::Literal(_)))
    );
    let first = Value::ints([2, 3], vec![1, 2, 3, 4, 5, 6])
        .unwrap()
        .into_shared();
    let before = match first.data() {
        Data::Int(CpuStorage::Shared(data)) => (data.as_ptr(), std::sync::Arc::strong_count(data)),
        _ => panic!("expected shared array"),
    };
    report.validate_noun_inputs([("X", &first)]).unwrap();
    match first.data() {
        Data::Int(CpuStorage::Shared(data)) => {
            assert_eq!(before, (data.as_ptr(), std::sync::Arc::strong_count(data)))
        }
        _ => unreachable!(),
    }
    drop(first);
    let second = Value::ints([2, 3], vec![9, 8, 7, 6, 5, 4]).unwrap();
    report.validate_noun_inputs([("X", &second)]).unwrap();
    assert_eq!(report.inputs.len(), 1);
    report.graph.verify().unwrap();
}

#[test]
fn metadata_mismatch_and_missing_duplicate_or_extra_inputs_are_rejected() {
    let mut analyzer = StaticAnalyzer::new();
    analyzer
        .declare_noun(
            "X",
            declared(TypeFact::Exact(DType::Int), Some(vec![2, 3]), None),
        )
        .unwrap();
    let report = analyzer.analyze(",X").unwrap();
    let right = Value::ints([2, 3], vec![0; 6]).unwrap();
    let wrong_type = Value::new([2, 3], Data::Float(CpuStorage::new(vec![0.0; 6]))).unwrap();
    let wrong_rank = Value::ints([6], vec![0; 6]).unwrap();
    let wrong_shape = Value::ints([3, 2], vec![0; 6]).unwrap();
    assert_eq!(
        report
            .validate_noun_inputs([("X", &wrong_type)])
            .unwrap_err(),
        Error::Domain
    );
    assert_eq!(
        report
            .validate_noun_inputs([("X", &wrong_rank)])
            .unwrap_err(),
        Error::Rank
    );
    assert_eq!(
        report
            .validate_noun_inputs([("X", &wrong_shape)])
            .unwrap_err(),
        Error::Length
    );
    assert_eq!(
        report.validate_noun_inputs([]).unwrap_err(),
        Error::Value("X".into())
    );
    assert_eq!(
        report
            .validate_noun_inputs([("X", &right), ("X", &right)])
            .unwrap_err(),
        Error::Domain
    );
    assert_eq!(
        report.validate_noun_inputs([("typo", &right)]).unwrap_err(),
        Error::Domain
    );
    report.validate_noun_inputs([("X", &right)]).unwrap();
}

#[test]
fn partial_metadata_does_not_invent_fixed_extents_or_function_guards() {
    let mut analyzer = StaticAnalyzer::new();
    analyzer
        .declare_noun("X", declared(TypeFact::IntOrFloat, None, Some(2)))
        .unwrap();
    analyzer
        .declare_function("f", FunctionPartOfSpeech::Verb)
        .unwrap();
    let report = analyzer.analyze("f X").unwrap();
    assert!(!report.boundaries.is_empty());
    let first = Value::ints([1, 3], vec![1; 3]).unwrap();
    let second = Value::new([4, 3], Data::Float(CpuStorage::new(vec![2.0; 12]))).unwrap();
    report.validate_noun_inputs([("X", &first)]).unwrap();
    report.validate_noun_inputs([("X", &second)]).unwrap();
    assert_eq!(
        report
            .validate_noun_inputs([("X", &first), ("f", &first)])
            .unwrap_err(),
        Error::Domain
    );
    // Successful noun metadata validation does not remove the unresolved call.
    assert!(!report.boundaries.is_empty());
    analyzer
        .declare_noun(
            "X",
            declared(TypeFact::Exact(DType::Int), Some(vec![1, 3]), None),
        )
        .unwrap();
    // The report owns its old declaration; a later catalog change does not
    // silently specialize its unknown extents or upgrade its version witness.
    report.validate_noun_inputs([("X", &second)]).unwrap();
    assert_ne!(
        report
            .inputs
            .iter()
            .find(|input| input.name == "X")
            .unwrap()
            .version,
        analyzer.binding("X").unwrap().version
    );
}

#[test]
fn unknown_and_empty_inputs_are_checked_without_using_array_contents() {
    let mut analyzer = StaticAnalyzer::new();
    analyzer.declare_noun("X", GraphFacts::default()).unwrap();
    let unknown = analyzer.analyze(",X").unwrap();
    let text = Value::new([3], Data::Char(CpuStorage::new(vec![b'a'; 3]))).unwrap();
    unknown.validate_noun_inputs([("X", &text)]).unwrap();
    analyzer
        .declare_noun(
            "X",
            declared(TypeFact::Exact(DType::Int), Some(vec![0, 3]), None),
        )
        .unwrap();
    let empty = analyzer.analyze(",X").unwrap();
    let array = Value::ints([0, 3], vec![]).unwrap();
    empty.validate_noun_inputs([("X", &array)]).unwrap();
    let scalar = Value::ints([], vec![3]).unwrap();
    assert_eq!(
        empty.validate_noun_inputs([("X", &scalar)]).unwrap_err(),
        Error::Rank
    );
}

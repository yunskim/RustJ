use rustj::{Data, Engine, Error, Value, logical_executor::execute_closed};

fn assert_value_eq(actual: &Value, expected: &Value, source: &str) {
    assert_eq!(actual.shape(), expected.shape(), "{source}: shape");
    assert_eq!(actual.type_code(), expected.type_code(), "{source}: type");
    match (actual.data(), expected.data()) {
        (Data::Bool(actual), Data::Bool(expected)) | (Data::Char(actual), Data::Char(expected)) => {
            assert!(actual.iter().eq(expected.iter()), "{source}: byte atoms");
        }
        (Data::Int(actual), Data::Int(expected)) => {
            assert!(actual.iter().eq(expected.iter()), "{source}: integer atoms");
        }
        (Data::Float(actual), Data::Float(expected)) => {
            assert!(actual.iter().eq(expected.iter()), "{source}: float atoms");
        }
        _ => panic!("{source}: unsupported comparison representation"),
    }
}

fn compare(source: &str) {
    let mut runtime = Engine::new();
    let expected = runtime.eval(source);
    let plan = Engine::new().analyze_a3(source).unwrap();
    let actual = execute_closed(&plan);
    match (actual, expected) {
        (Ok(Some(actual)), Ok(Some(expected))) => {
            assert_value_eq(&actual, &expected, source);
        }
        (Ok(None), Ok(None)) => {}
        (Err(actual), Err(expected)) => {
            assert_eq!(actual.kind(), expected.kind(), "{source}: error class");
        }
        _ => panic!("{source}: reference/runtime result mismatch"),
    }
}

#[test]
fn closed_a3_reference_executor_matches_runtime_for_v0_basis_core() {
    for source in [
        "1+2",
        "1 2+3 4",
        "+/1 2 3",
        "|.1 2 3",
        ",2 3$ i.6",
        "i.2 3",
        "$1 2 3",
        "#1 2 3",
        "'a'='a'",
        "'co' E. 'cocoa'",
    ] {
        compare(source);
    }
}

#[test]
fn reference_executor_preserves_prefix_agreement_error() {
    let source = "1 2+1 2 3";
    assert!(matches!(Engine::new().eval(source), Err(Error::Length)));
    let plan = Engine::new().analyze_a3(source).unwrap();
    assert!(matches!(execute_closed(&plan), Err(Error::Length)));
}

#[test]
fn reference_executor_preserves_gather_index_semantics_including_negative_indices() {
    for source in ["1 { 10 20 30", "_1 { 10 20 30"] {
        compare(source);
    }

    let source = "3 { 10 20 30";
    assert!(matches!(Engine::new().eval(source), Err(Error::Index)));
    let plan = Engine::new().analyze_a3(source).unwrap();
    assert!(matches!(execute_closed(&plan), Err(Error::Index)));
}

#[test]
fn reference_executor_runs_valid_cell_apply_without_flattening_it() {
    compare("+/\"1 (2 3$ i.6)");
}

#[test]
fn reference_executor_rejects_environment_dependent_name_reads() {
    let mut engine = Engine::new();
    engine.eval("a=:1 2 3").unwrap();
    let plan = engine.analyze_a3("a").unwrap();
    assert!(matches!(execute_closed(&plan), Err(Error::Unsupported(_))));
}

#[test]
fn rank_noun_contract_is_shared_by_parser_analysis_and_reference_executor() {
    for rank in [
        "_",
        "__",
        "_.",
        "1e100",
        "_1e100",
        "1.00000000000001",
        "0 1 _",
    ] {
        compare(&format!("(+\"{rank}) i.2 3"));
        compare(&format!("(2+i.2 3) (+\"{rank}) i.2 3"));
    }
}

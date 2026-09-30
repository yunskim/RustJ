use rustj::{
    Engine,
    analysis::BasisKind,
    logical_ir::{Constraint, OpKind, SemanticErrorKind},
};

#[test]
fn a3_separates_operations_from_values() {
    let plan = Engine::new().analyze_a3("1+2").unwrap();
    plan.verify().unwrap();

    assert_eq!(plan.values.len(), 3);
    assert_eq!(plan.operations.len(), 3);
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    assert!(matches!(
        plan.operations[producer.0].kind,
        OpKind::Basis {
            kind: BasisKind::Elementwise,
            ..
        }
    ));
}

#[test]
fn proven_prefix_agreement_needs_no_runtime_check() {
    let plan = Engine::new().analyze_a3("1 2+3 4").unwrap();
    plan.verify().unwrap();

    assert!(
        plan.operations
            .iter()
            .all(|op| !matches!(op.kind, OpKind::SemanticCheck(_)))
    );

    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    let OpKind::Basis { call, .. } = &plan.operations[producer.0].kind else {
        panic!("result should be a basis call")
    };
    assert_eq!(call.constraints.facts.len(), 1);
    assert!(call.constraints.facts[0].witness.is_some());
}

#[test]
fn unresolved_prefix_agreement_is_a_zero_result_semantic_check() {
    let plan = Engine::new().analyze_a3("1 2+1 2 3").unwrap();
    plan.verify().unwrap();

    let (check_id, check) = plan
        .operations
        .iter()
        .enumerate()
        .find_map(|(index, op)| match &op.kind {
            OpKind::SemanticCheck(check) => Some((index, check)),
            _ => None,
        })
        .expect("length check");

    assert!(plan.operations[check_id].results.is_empty());
    assert_eq!(check.error, SemanticErrorKind::Length);
    assert!(matches!(check.constraint, Constraint::PrefixAgreement { .. }));

    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    assert_eq!(
        plan.operations[producer.0].order_after,
        Some(rustj::logical_ir::OpId(check_id))
    );
}

#[test]
fn gather_has_an_explicit_index_check() {
    let plan = Engine::new().analyze_a3("1 { 10 20 30").unwrap();
    plan.verify().unwrap();

    let check = plan
        .operations
        .iter()
        .find_map(|op| match &op.kind {
            OpKind::SemanticCheck(check) => Some(check),
            _ => None,
        })
        .expect("index check");
    assert_eq!(check.error, SemanticErrorKind::Index);
    assert!(matches!(check.constraint, Constraint::IndicesInBounds { .. }));

    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    assert!(matches!(
        plan.operations[producer.0].kind,
        OpKind::Basis {
            kind: BasisKind::Gather,
            ..
        }
    ));
}

#[test]
fn semantic_check_cannot_produce_an_ssa_value() {
    let mut plan = Engine::new().analyze_a3("1 2+1 2 3").unwrap();
    let check_id = plan
        .operations
        .iter()
        .position(|op| matches!(op.kind, OpKind::SemanticCheck(_)))
        .expect("semantic check");
    let result = plan.result.unwrap();
    plan.operations[check_id].results.push(result);

    let error = plan.verify().unwrap_err();
    assert_eq!(error.operation, Some(rustj::logical_ir::OpId(check_id)));
    assert!(error.message.contains("must not produce"));
}

#[test]
fn write_metadata_uses_a3_value_and_operation_ids() {
    let plan = Engine::new().analyze_a3("a=:1+2").unwrap();
    plan.verify().unwrap();

    let write = plan.write.as_ref().expect("write");
    assert_eq!(Some(write.value), plan.result);
    assert!(write.after.is_some());
    assert!(write.after.unwrap().0 < plan.operations.len());
}

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


#[test]
fn reduce_domain_marks_the_reduced_axis_explicitly() {
    use rustj::logical_ir::{
        AxisRole, BasisPayload, IterationAxisKind, ReductionAxis,
    };

    let plan = Engine::new().analyze_a3("+/1 2 3").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    let OpKind::Basis {
        kind,
        payload,
        call,
    } = &plan.operations[producer.0].kind
    else {
        panic!("reduce basis op")
    };

    assert_eq!(*kind, BasisKind::Reduce);
    assert_eq!(
        *payload,
        BasisPayload::Reduce {
            axis: ReductionAxis::LeadingCellAxis
        }
    );
    assert_eq!(call.iteration_domain.axes.len(), 1);
    assert_eq!(call.iteration_domain.axes[0].extent, Some(3));
    assert_eq!(
        call.iteration_domain.axes[0].kind,
        IterationAxisKind::Reduction
    );
    assert_eq!(call.iteration_domain.axes[0].role, AxisRole::Reduction);
}

#[test]
fn cell_apply_domain_is_the_result_frame_not_the_cell() {
    use rustj::logical_ir::{AxisRole, IterationAxisKind};

    let mut engine = Engine::new();
    engine.eval("a=:i.2 3").unwrap();
    let plan = engine.analyze_a3("+/\"1 a").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    let OpKind::Basis { kind, call, .. } = &plan.operations[producer.0].kind else {
        panic!("cell apply basis op")
    };

    assert_eq!(*kind, BasisKind::CellApply);
    assert_eq!(call.iteration_domain.axes.len(), 1);
    assert_eq!(call.iteration_domain.axes[0].extent, Some(2));
    assert_eq!(call.iteration_domain.axes[0].kind, IterationAxisKind::Parallel);
    assert_eq!(call.iteration_domain.axes[0].role, AxisRole::Frame);
}

#[test]
fn static_reindex_payload_preserves_the_reindex_family() {
    use rustj::logical_ir::{BasisPayload, ReindexKind};

    let plan = Engine::new().analyze_a3("|.1 2 3").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    let OpKind::Basis { payload, .. } = &plan.operations[producer.0].kind else {
        panic!("reindex basis op")
    };
    assert_eq!(
        *payload,
        BasisPayload::StaticReindex {
            kind: ReindexKind::Reverse
        }
    );
}

#[test]
fn verifier_rejects_a_basis_payload_that_no_longer_matches_the_call() {
    use rustj::logical_ir::BasisPayload;

    let mut plan = Engine::new().analyze_a3("|.1 2 3").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    let OpKind::Basis { payload, .. } = &mut plan.operations[producer.0].kind else {
        panic!("reindex basis op")
    };
    *payload = BasisPayload::Elementwise;

    let error = plan.verify().unwrap_err();
    assert_eq!(error.operation, Some(producer));
    assert!(error.message.contains("basis payload"));
}

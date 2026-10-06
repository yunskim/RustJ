use rustj::{
    Data, Engine, Value,
    logical_executor::execute_closed,
    logical_ir::{OpId, ValueId},
    physical::Encoding,
    physical_plan::{
        BufferOwnership, MemorySpace, PhysicalOp, PhysicalPlan, PhysicalPlanError,
        PhysicalViewId, PlanBufferId, ViewAccess,
    },
};

fn assert_same_dense_value(actual: &Value, expected: &Value) {
    assert_eq!(actual.shape(), expected.shape());
    assert_eq!(actual.type_code(), expected.type_code());
    match (actual.data(), expected.data()) {
        (Data::Bool(a), Data::Bool(b)) | (Data::Char(a), Data::Char(b)) => {
            assert!(a.iter().eq(b.iter()));
        }
        (Data::Int(a), Data::Int(b)) => assert!(a.iter().eq(b.iter())),
        (Data::Float(a), Data::Float(b)) => assert!(a.iter().eq(b.iter())),
        _ => panic!("identity test unexpectedly received boxed/sparse output"),
    }
}

#[test]
fn m4_empty_a3_plan_requires_no_buffers_views_or_execution() {
    let logical = Engine::new().analyze_a3("NB. empty sentence").unwrap();
    let physical = PhysicalPlan::empty_from_a3(&logical).unwrap();
    physical.verify(&logical).unwrap();
    assert!(physical.execute_identity(&logical).unwrap().is_none());
    assert!(physical.buffers.is_empty());
    assert!(physical.views.is_empty());

    let mut forged = physical.clone();
    forged.operations.push(PhysicalOp::Check { source_op: OpId(0) });
    assert!(matches!(
        forged.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));
}

#[test]
fn m4_identity_literal_result_matches_independent_a3_reference() {
    for source in ["7", "1 2 3", "2.5", "'abc'", "''"] {
        let logical = Engine::new().analyze_a3(source).unwrap();
        let physical = PhysicalPlan::identity_literal(&logical).unwrap();
        physical.verify(&logical).unwrap();
        let actual = physical.execute_identity(&logical).unwrap().unwrap();
        let expected = execute_closed(&logical).unwrap().unwrap();
        assert_same_dense_value(&actual, &expected);
        assert_eq!(physical.buffers[0].memory, MemorySpace::Host);
        assert_eq!(physical.buffers[0].ownership, BufferOwnership::Input);
        assert_eq!(physical.views[0].access, ViewAccess::ReadOnly);
    }
}

#[test]
fn m4_rejects_check_kernel_and_stateful_a3_instead_of_skipping_them() {
    for source in ["1+2", "1 2+1 2 3", "a=:3"] {
        let logical = Engine::new().analyze_a3(source).unwrap();
        assert!(
            matches!(
                PhysicalPlan::identity_literal(&logical),
                Err(PhysicalPlanError::Unsupported(_))
            ),
            "must fail closed: {source}"
        );
    }
}

#[test]
fn m4_negative_verifier_rejects_each_forged_identity_invariant() {
    let logical = Engine::new().analyze_a3("1 2 3").unwrap();
    let plan = PhysicalPlan::identity_literal(&logical).unwrap();

    let mut wrong_source = plan.clone();
    wrong_source.source_text.push(' ');
    assert!(matches!(
        wrong_source.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut wrong_size = plan.clone();
    wrong_size.buffers[0].atoms += 1;
    assert!(matches!(
        wrong_size.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut wrong_encoding = plan.clone();
    wrong_encoding.views[0].encoding = Encoding::Float64;
    assert!(matches!(
        wrong_encoding.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut wrong_buffer = plan.clone();
    wrong_buffer.views[0].buffer = PlanBufferId(123);
    assert!(matches!(
        wrong_buffer.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut wrong_stride = plan.clone();
    wrong_stride.views[0].strides[0] = -1;
    assert!(matches!(
        wrong_stride.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut wrong_access = plan.clone();
    wrong_access.views[0].access = ViewAccess::ExclusiveWrite;
    assert!(matches!(
        wrong_access.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut wrong_ownership = plan.clone();
    wrong_ownership.buffers[0].ownership = BufferOwnership::Temporary;
    assert!(matches!(
        wrong_ownership.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut early_return = plan.clone();
    early_return.operations.swap(0, 1);
    assert!(matches!(
        early_return.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut dangling_return = plan.clone();
    dangling_return.operations[1] = PhysicalOp::Return {
        logical_value: ValueId(0),
        view: PhysicalViewId(1),
    };
    assert!(matches!(
        dangling_return.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut illicit_kernel = plan.clone();
    illicit_kernel.operations[1] = PhysicalOp::Kernel {
        source_op: OpId(0),
        inputs: vec![PhysicalViewId(0)],
        output: PhysicalViewId(0),
        realization: rustj::lowering::RealizationFamily::CpuSimd,
    };
    assert!(matches!(
        illicit_kernel.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));
}

#[test]
fn m4_executes_no_invalid_plan_or_wrong_logical_source() {
    let logical = Engine::new().analyze_a3("1 2 3").unwrap();
    let mut invalid = PhysicalPlan::identity_literal(&logical).unwrap();
    invalid.operations.pop();
    assert!(matches!(
        invalid.execute_identity(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let other = Engine::new().analyze_a3("4 5 6").unwrap();
    let physical = PhysicalPlan::identity_literal(&logical).unwrap();
    assert!(matches!(
        physical.execute_identity(&other),
        Err(PhysicalPlanError::Invalid(_))
    ));
}


#[test]
fn m4_monadic_reindex_uses_a_checked_view_then_materializes_logical_order() {
    for source in ["|.1 2 3", "|.'abc'", "|:1 2 3", "|:1 2 3 4"] {
        let logical = Engine::new().analyze_a3(source).unwrap();
        let physical = PhysicalPlan::monadic_static_view(&logical).unwrap();
        assert_eq!(physical.buffers.len(), 2, "{source}");
        assert_eq!(physical.views.len(), 3, "{source}");
        assert!(matches!(physical.operations[1], PhysicalOp::View { .. }));
        assert!(matches!(physical.operations[2], PhysicalOp::Materialize { .. }));
        physical.verify(&logical).unwrap();
        let actual = physical.execute_host_result(&logical).unwrap().unwrap();
        let expected = execute_closed(&logical).unwrap().unwrap();
        assert_same_dense_value(&actual, &expected);
    }
}

#[test]
fn m4_monadic_reindex_rejects_affine_forgery_and_lifetime_miswiring() {
    let logical = Engine::new().analyze_a3("|.1 2 3").unwrap();
    let original = PhysicalPlan::monadic_static_view(&logical).unwrap();

    let mut bad_stride = original.clone();
    bad_stride.views[1].strides[0] = isize::MAX;
    assert!(matches!(
        bad_stride.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut bad_span = original.clone();
    bad_span.views[1].offset = isize::MAX;
    assert!(matches!(
        bad_span.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut wrong_return_owner = original.clone();
    wrong_return_owner.buffers[1].ownership = BufferOwnership::Input;
    assert!(matches!(
        wrong_return_owner.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut skipped_materialization = original.clone();
    skipped_materialization.operations.remove(2);
    assert!(matches!(
        skipped_materialization.execute_host_result(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut wrong_schedule = original.clone();
    wrong_schedule.operations.swap(1, 2);
    assert!(matches!(
        wrong_schedule.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut wrong_buffer = original.clone();
    wrong_buffer.views[1].buffer = PlanBufferId(1);
    assert!(matches!(
        wrong_buffer.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));

    let mut wrong_output_view = original;
    wrong_output_view.views[2].strides[0] = -1;
    assert!(matches!(
        wrong_output_view.verify(&logical),
        Err(PhysicalPlanError::Invalid(_))
    ));
}

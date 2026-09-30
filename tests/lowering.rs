use rustj::{
    Engine,
    analysis::BasisKind,
    logical_ir::{CallOp, EffectSummary, OpKind, SpeculationSemantics},
    lowering::{LoweringRegistry, RealizationFamily, TargetCapabilities},
};

fn result_basis_call(source: &str) -> (BasisKind, CallOp) {
    let plan = Engine::new().analyze_a3(source).unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    let OpKind::Basis { kind, call, .. } = &plan.operations[producer.0].kind else {
        panic!("result is not a basis op: {source}")
    };
    (*kind, call.clone())
}

#[test]
fn registry_returns_reference_route_without_claiming_parallel_legality() {
    let (basis, call) = result_basis_call("1+2");
    assert_eq!(basis, BasisKind::Elementwise);

    let registry = LoweringRegistry::a3_v0();
    let candidates = registry.legal_candidates(basis, &call, &TargetCapabilities::cpu_simd());
    assert_eq!(candidates, vec![RealizationFamily::ReferenceSequential]);

    let gpu = registry.legal_candidates(basis, &call, &TargetCapabilities::gpu_generic());
    assert!(gpu.is_empty());
}

#[test]
fn parallel_elementwise_route_opens_only_after_semantic_safety_is_proven() {
    let (basis, mut call) = result_basis_call("1+2");
    call.effect = EffectSummary::Pure;
    call.speculation = SpeculationSemantics {
        may_raise_observable_error: false,
        preserve_evaluation_order: false,
    };

    let registry = LoweringRegistry::a3_v0();
    let cpu = registry.legal_candidates(basis, &call, &TargetCapabilities::cpu_simd());
    assert!(cpu.contains(&RealizationFamily::ReferenceSequential));
    assert!(cpu.contains(&RealizationFamily::CpuSimd));

    let gpu = registry.legal_candidates(basis, &call, &TargetCapabilities::gpu_generic());
    assert_eq!(gpu, vec![RealizationFamily::GpuDataParallel]);
}

#[test]
fn tree_reduction_requires_reassociation_and_error_order_freedom() {
    let (basis, mut call) = result_basis_call("+/1 2 3");
    assert_eq!(basis, BasisKind::Reduce);

    let registry = LoweringRegistry::a3_v0();
    let gpu = TargetCapabilities::gpu_generic();
    assert!(
        registry
            .legal_candidates(basis, &call, &gpu)
            .is_empty()
    );

    call.effect = EffectSummary::Pure;
    call.speculation = SpeculationSemantics {
        may_raise_observable_error: false,
        preserve_evaluation_order: false,
    };
    assert!(
        registry
            .legal_candidates(basis, &call, &gpu)
            .is_empty()
    );

    call.contract.allow_reassociation = true;
    assert_eq!(
        registry.legal_candidates(basis, &call, &gpu),
        vec![RealizationFamily::GpuTreeReduction]
    );
}

#[test]
fn cell_apply_has_only_the_generic_cpu_route_until_uniformity_is_proven() {
    let (basis, call) = result_basis_call("+/\"1 (1 2 3)");
    assert_eq!(basis, BasisKind::CellApply);

    let registry = LoweringRegistry::a3_v0();
    assert_eq!(
        registry.legal_candidates(basis, &call, &TargetCapabilities::cpu_simd()),
        vec![RealizationFamily::GenericCellLoop]
    );
    assert!(
        registry
            .legal_candidates(basis, &call, &TargetCapabilities::gpu_generic())
            .is_empty()
    );
}

#[test]
fn gather_keeps_indexed_parallel_routes_closed_while_errors_are_observable() {
    let (basis, mut call) = result_basis_call("1 { 10 20 30");
    assert_eq!(basis, BasisKind::Gather);

    let registry = LoweringRegistry::a3_v0();
    assert_eq!(
        registry.legal_candidates(basis, &call, &TargetCapabilities::cpu_simd()),
        vec![RealizationFamily::ReferenceSequential]
    );

    call.effect = EffectSummary::Pure;
    call.speculation = SpeculationSemantics {
        may_raise_observable_error: false,
        preserve_evaluation_order: false,
    };
    assert_eq!(
        registry.legal_candidates(basis, &call, &TargetCapabilities::gpu_generic()),
        vec![RealizationFamily::GpuIndexed]
    );
}

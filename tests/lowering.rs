use rustj::{
    Engine,
    analysis::ExecutionBasisKind,
    logical_ir::{CallOp, EffectSummary, OpKind, SpeculationSemantics},
    lowering::{
        BasisTargetFeasibility, LoweringRegistry, RealizationFamily,
        RewritePlanningState, RewriteTargetFeasibilityKind, TargetCapabilities,
    },
};


fn literal_source_values(plan: &rustj::logical_ir::Plan) -> Vec<Option<rustj::Value>> {
    let mut values = vec![None; plan.values.len()];
    for operation in &plan.operations {
        if let OpKind::Literal(value) = &operation.kind {
            let [result] = operation.results.as_slice() else {
                panic!("literal should have one result")
            };
            values[result.0] = Some(value.clone());
        }
    }
    values
}

fn result_basis_call(source: &str) -> (ExecutionBasisKind, CallOp) {
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
    assert_eq!(basis, ExecutionBasisKind::Elementwise);

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
    call.possible_errors.unknown = false;
    call.possible_errors.known.clear();
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
    assert_eq!(basis, ExecutionBasisKind::Reduce);

    let registry = LoweringRegistry::a3_v0();
    let gpu = TargetCapabilities::gpu_generic();
    assert!(
        registry
            .legal_candidates(basis, &call, &gpu)
            .is_empty()
    );

    call.effect = EffectSummary::Pure;
    call.possible_errors.unknown = false;
    call.possible_errors.known.clear();
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
    assert_eq!(basis, ExecutionBasisKind::CellApply);

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
    assert_eq!(basis, ExecutionBasisKind::Gather);

    let registry = LoweringRegistry::a3_v0();
    assert_eq!(
        registry.legal_candidates(basis, &call, &TargetCapabilities::cpu_simd()),
        vec![RealizationFamily::ReferenceSequential]
    );

    call.effect = EffectSummary::Pure;
    call.possible_errors.unknown = false;
    call.possible_errors.known.clear();
    call.speculation = SpeculationSemantics {
        may_raise_observable_error: false,
        preserve_evaluation_order: false,
    };
    assert_eq!(
        registry.legal_candidates(basis, &call, &TargetCapabilities::gpu_generic()),
        vec![RealizationFamily::GpuIndexed]
    );
}



#[test]
fn graph_rewrite_target_feasibility_does_not_confuse_equivalence_with_lowerability() {
    let analysis = Engine::new()
        .analyze_compilation("'ana' E. 'banana'")
        .unwrap();
    let candidate = &analysis.graph_rewrites[0];
    let registry = LoweringRegistry::a3_v0();

    let cpu = registry.rewrite_candidate_target_feasibility(
        candidate,
        &TargetCapabilities::cpu_baseline(),
    );
    assert_eq!(cpu.overall, RewriteTargetFeasibilityKind::Supported);
    assert_eq!(
        cpu.composite_candidates,
        vec![RealizationFamily::ReferenceRewriteComposite]
    );
    assert!(!cpu.composite_requires_call_facts);
    assert_eq!(cpu.nodes.len(), 2);
    assert!(matches!(
        cpu.nodes[0].feasibility,
        BasisTargetFeasibility::Unsupported
    ));
    assert!(matches!(
        cpu.nodes[1].feasibility,
        BasisTargetFeasibility::RequiresCallFacts
    ));

    let gpu = registry.rewrite_candidate_target_feasibility(
        candidate,
        &TargetCapabilities::gpu_generic(),
    );
    assert_eq!(gpu.overall, RewriteTargetFeasibilityKind::Unsupported);
    assert!(gpu.nodes.iter().all(|node| {
        matches!(node.feasibility, BasisTargetFeasibility::Unsupported)
    }));
}


#[test]
fn rewrite_planning_report_defers_selection_until_target_and_resource_facts_exist() {
    let analysis = Engine::new()
        .analyze_compilation("'ana' E. 'banana'")
        .unwrap();
    let registry = LoweringRegistry::a3_v0();
    let reports = registry.rewrite_planning_reports(
        &analysis,
        &TargetCapabilities::cpu_baseline(),
    );

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(report.candidate_index, 0);
    assert_eq!(report.state, RewritePlanningState::NeedsResourceFacts);
    assert!(!report.early_pruning_allowed);
    assert_eq!(
        report.target_feasibility.overall,
        RewriteTargetFeasibilityKind::Supported
    );
    assert_eq!(
        report.target_feasibility.composite_candidates,
        vec![RealizationFamily::ReferenceRewriteComposite]
    );
    assert_eq!(
        report.resource_evaluation.source_value,
        analysis.graph_rewrites[0].provenance.source_value
    );
}


#[test]
fn registered_cpu_rewrite_composite_dispatches_to_reference_expansion() {
    let analysis = Engine::new()
        .analyze_compilation("'co' E. 'cocoa'")
        .unwrap();
    let plan = rustj::logical_ir::Plan::from_transition(&analysis.execution);
    plan.verify().unwrap();
    let values = literal_source_values(&plan);

    let registry = LoweringRegistry::a3_v0();
    let rewritten = registry
        .execute_reference_rewrite(
            &analysis,
            &plan,
            0,
            &TargetCapabilities::cpu_baseline(),
            &values,
        )
        .unwrap()
        .json();
    let direct = Engine::new()
        .eval("'co' E. 'cocoa'")
        .unwrap()
        .unwrap()
        .json();
    assert_eq!(rewritten, direct);

    assert!(
        registry
            .execute_reference_rewrite(
                &analysis,
                &plan,
                0,
                &TargetCapabilities::gpu_generic(),
                &values,
            )
            .is_err()
    );
}

#[test]
fn route_partition_distinguishes_native_fallback_checks_and_value_ops() {
    use rustj::lowering::RouteDecision;

    let registry = LoweringRegistry::a3_v0();
    let cpu = TargetCapabilities::cpu_baseline();
    let gpu = TargetCapabilities::gpu_generic();

    let plan = Engine::new().analyze_a3("1+2").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    assert_eq!(
        registry.route_operation(&plan.operations[producer.0], &cpu),
        RouteDecision::NativeExecutionBasis {
            basis: ExecutionBasisKind::Elementwise,
            candidates: vec![RealizationFamily::ReferenceSequential],
        }
    );
    assert_eq!(
        registry.route_operation(&plan.operations[producer.0], &gpu),
        RouteDecision::RuntimeSemanticFallback
    );

    let plan = Engine::new().analyze_a3("future 3").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    assert_eq!(
        registry.route_operation(&plan.operations[producer.0], &cpu),
        RouteDecision::RuntimeSemanticFallback
    );

    let plan = Engine::new().analyze_a3("1 2+1 2 3").unwrap();
    let check = plan
        .operations
        .iter()
        .find(|op| matches!(op.kind, OpKind::SemanticCheck(_)))
        .expect("semantic check");
    assert_eq!(
        registry.route_operation(check, &cpu),
        RouteDecision::SemanticCheck
    );

    let literal = &plan.operations[0];
    assert_eq!(
        registry.route_operation(literal, &cpu),
        RouteDecision::NoKernel
    );
}


#[test]
fn route_partition_forms_contiguous_semantic_regions() {
    use rustj::lowering::RouteRegionClass;

    let registry = LoweringRegistry::a3_v0();
    let cpu = TargetCapabilities::cpu_baseline();

    let plan = Engine::new().analyze_a3("1 2+1 2 3").unwrap();
    let regions = registry.partition_plan(&plan, &cpu);
    assert_eq!(
        regions.iter().map(|region| region.class).collect::<Vec<_>>(),
        vec![
            RouteRegionClass::ValueOnly,
            RouteRegionClass::SemanticCheck,
            RouteRegionClass::PureArray,
        ]
    );

    let plan = Engine::new().analyze_a3("future 3").unwrap();
    let regions = registry.partition_plan(&plan, &cpu);
    assert_eq!(
        regions.iter().map(|region| region.class).collect::<Vec<_>>(),
        vec![
            RouteRegionClass::ValueOnly,
            RouteRegionClass::RuntimeSemantic,
        ]
    );
}

#[test]
fn rank_and_reduce_can_use_native_reference_routes_after_purity_resolution() {
    use rustj::lowering::RouteDecision;

    let registry = LoweringRegistry::a3_v0();
    let cpu = TargetCapabilities::cpu_baseline();

    for (source, expected_basis, expected_realization) in [
        (
            "+/1 2 3",
            ExecutionBasisKind::Reduce,
            RealizationFamily::OrderedReduction,
        ),
        (
            "+/\"1 (2 3$ i.6)",
            ExecutionBasisKind::CellApply,
            RealizationFamily::GenericCellLoop,
        ),
    ] {
        let plan = Engine::new().analyze_a3(source).unwrap();
        let result = plan.result.unwrap();
        let producer = plan.values[result.0].producer;
        assert_eq!(
            registry.route_operation(&plan.operations[producer.0], &cpu),
            RouteDecision::NativeExecutionBasis {
                basis: expected_basis,
                candidates: vec![expected_realization],
            },
            "{source}"
        );
    }
}


#[test]
fn lowering_recipe_keeps_semantic_parameters_but_not_schedule_choices() {
    use rustj::logical_ir::{ExecutionBasisPayload, ReductionAxis};

    let registry = LoweringRegistry::a3_v0();
    let cpu = TargetCapabilities::cpu_baseline();
    let plan = Engine::new().analyze_a3("+/1 2 3").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;

    let recipes = registry.recipes_for_operation(&plan.operations[producer.0], &cpu);
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].basis, ExecutionBasisKind::Reduce);
    assert_eq!(
        recipes[0].realization,
        RealizationFamily::OrderedReduction
    );
    assert_eq!(
        recipes[0].payload,
        ExecutionBasisPayload::Reduce {
            axis: ReductionAxis::LeadingCellAxis
        }
    );
    assert_eq!(recipes[0].iteration_domain.axes.len(), 1);
    assert_eq!(recipes[0].iteration_domain.axes[0].extent, Some(3));
}

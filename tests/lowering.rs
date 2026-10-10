use rustj::{
    Engine,
    analysis::ExecutionBasisKind,
    j_graph_jsource::{JsourceFamily, family_rule},
    logical_ir::{CallOp, EffectSummary, OpKind, SpeculationSemantics},
    lowering::{
        BasisTargetFeasibility, JsourceExistingRoute, JsourcePlanningState, LoweringRegistry,
        RealizationFamily, RewritePlanningState, RewriteTargetFeasibilityKind, TargetCapabilities,
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
    assert!(registry.legal_candidates(basis, &call, &gpu).is_empty());

    call.effect = EffectSummary::Pure;
    call.possible_errors.unknown = false;
    call.possible_errors.known.clear();
    call.speculation = SpeculationSemantics {
        may_raise_observable_error: false,
        preserve_evaluation_order: false,
    };
    assert!(registry.legal_candidates(basis, &call, &gpu).is_empty());

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

    let cpu = registry
        .rewrite_candidate_target_feasibility(candidate, &TargetCapabilities::cpu_baseline());
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

    let gpu = registry
        .rewrite_candidate_target_feasibility(candidate, &TargetCapabilities::gpu_generic());
    assert_eq!(gpu.overall, RewriteTargetFeasibilityKind::Unsupported);
    assert!(
        gpu.nodes
            .iter()
            .all(|node| { matches!(node.feasibility, BasisTargetFeasibility::Unsupported) })
    );
}

#[test]
fn rewrite_planning_report_defers_selection_until_target_and_resource_facts_exist() {
    let analysis = Engine::new()
        .analyze_compilation("'ana' E. 'banana'")
        .unwrap();
    let registry = LoweringRegistry::a3_v0();
    let reports = registry.rewrite_planning_reports(&analysis, &TargetCapabilities::cpu_baseline());

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
    let plan = analysis.logical.clone();
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
        regions
            .iter()
            .map(|region| region.class)
            .collect::<Vec<_>>(),
        vec![
            RouteRegionClass::ValueOnly,
            RouteRegionClass::SemanticCheck,
            RouteRegionClass::PureArray,
        ]
    );

    let plan = Engine::new().analyze_a3("future 3").unwrap();
    let regions = registry.partition_plan(&plan, &cpu);
    assert_eq!(
        regions
            .iter()
            .map(|region| region.class)
            .collect::<Vec<_>>(),
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
    assert_eq!(recipes[0].realization, RealizationFamily::OrderedReduction);
    assert_eq!(
        recipes[0].payload,
        ExecutionBasisPayload::Reduce {
            axis: ReductionAxis::LeadingCellAxis
        }
    );
    assert_eq!(recipes[0].iteration_domain.axes.len(), 1);
    assert_eq!(recipes[0].iteration_domain.axes[0].extent, Some(3));
}

#[test]
fn jsource_source_evidence_reaches_logical_routes_without_authorizing_specialization() {
    let registry = LoweringRegistry::a3_v0();
    for (source, family) in [
        ("+/1 2 3", JsourceFamily::ReductionFastPath),
        ("(+/ % #) 1 2 3", JsourceFamily::MeanIdiom),
        ("1 { 10 20 30", JsourceFamily::GatherCopyOrView),
        ("1 3 5 I. 2 4", JsourceFamily::IntervalLookup),
    ] {
        let analysis = Engine::new().analyze_compilation(source).unwrap();
        let reports = registry
            .jsource_planning_reports(&analysis, &TargetCapabilities::cpu_baseline())
            .unwrap();
        let report = reports
            .iter()
            .find(|report| report.family == family)
            .unwrap_or_else(|| panic!("no {family:?} planning report for {source}"));
        assert_eq!(
            report.source_value,
            analysis.jsource_opportunities[report.candidate_index].source_value
        );
        assert_eq!(report.decision_owner, family_rule(family).owner);
        assert_eq!(
            report.unresolved_proofs.as_slice(),
            family_rule(family).proof_requirements
        );
        assert!(!report.unresolved_proofs.is_empty());
        assert_eq!(report.state, JsourcePlanningState::NeedsSemanticProof);
        assert!(!report.linked_calls.is_empty());
        assert!(report.linked_calls.iter().all(|call| {
            analysis.logical.operations[call.operation.0].j_origin == Some(report.source_value)
        }));
        assert!(analysis.jsource_opportunities.iter().all(|c| !c.selected));
    }
}

#[test]
fn jsource_planning_keeps_existing_routes_distinct_from_specialized_algorithms() {
    let analysis = Engine::new().analyze_compilation("1 { 10 20 30").unwrap();
    let registry = LoweringRegistry::a3_v0();
    let report = registry
        .jsource_planning_reports(&analysis, &TargetCapabilities::cpu_baseline())
        .unwrap()
        .into_iter()
        .find(|report| report.family == JsourceFamily::GatherCopyOrView)
        .unwrap();
    assert!(report.linked_calls.iter().any(|call| matches!(
        &call.existing,
        JsourceExistingRoute::Basis {
            kind: ExecutionBasisKind::Gather,
            realizations,
        } if realizations.contains(&RealizationFamily::ReferenceSequential)
    )));
    // The existing ordinary route must not discharge jsource proof obligations.
    assert_eq!(report.state, JsourcePlanningState::NeedsSemanticProof);

    let gpu = registry
        .jsource_planning_reports(&analysis, &TargetCapabilities::gpu_generic())
        .unwrap();
    assert!(gpu.iter().any(|report| {
        report.family == JsourceFamily::GatherCopyOrView
            && report.state == JsourcePlanningState::NeedsSemanticProof
            && report.linked_calls.iter().all(|call| {
                matches!(&call.existing, JsourceExistingRoute::Basis { realizations, .. } if realizations.is_empty())
            })
    }));
}

#[test]
fn jsource_planning_rejects_forged_candidates_and_missing_logical_origins() {
    let registry = LoweringRegistry::a3_v0();
    let mut analysis = Engine::new().analyze_compilation("1 { 10 20 30").unwrap();
    let source_value = analysis.jsource_opportunities[0].source_value;
    for op in &mut analysis.logical.operations {
        if op.j_origin == Some(source_value) {
            op.j_origin = None;
        }
    }
    let reports = registry
        .jsource_planning_reports(&analysis, &TargetCapabilities::cpu_baseline())
        .unwrap();
    assert_eq!(reports[0].state, JsourcePlanningState::NeedsLogicalCallLink);
    assert!(reports[0].linked_calls.is_empty());

    analysis.jsource_opportunities[0].selected = true;
    assert!(
        registry
            .jsource_planning_reports(&analysis, &TargetCapabilities::cpu_baseline())
            .is_err()
    );
}

#[test]
fn registry_separates_search_semantics_guarded_routes_and_unproven_tolerance() {
    use rustj::{
        logical_ir::{ExecutionBasisPayload, SearchOutputKind},
        lowering::{SearchAlgorithm as A, SearchAlgorithmReadiness as R},
    };
    let plan = Engine::new().analyze_a3("3 1 3 i: 3 4").unwrap();
    let output = plan.result.unwrap();
    let op = &plan.operations[plan.values[output.0].producer.0];
    let OpKind::Basis {
        kind,
        call,
        payload: ExecutionBasisPayload::LookupClassify { search },
    } = &op.kind
    else {
        panic!("no typed search");
    };

    assert_eq!(search.output, SearchOutputKind::LastIndex);
    let registry = LoweringRegistry::a3_v0();
    let cpu = TargetCapabilities::cpu_baseline();
    let reports = registry.search_algorithm_reports(search, &cpu);
    let status = |algorithm| {
        reports
            .iter()
            .find(|r| r.algorithm == algorithm)
            .map(|r| r.readiness)
    };
    assert_eq!(status(A::Sequential), Some(R::Baseline));
    for algorithm in [
        A::DirectAddress,
        A::IndexedHash,
        A::ReverseQueryHash,
        A::PreparedHash,
    ] {
        assert_eq!(status(algorithm), Some(R::RequiresExactScalarGuard));
    }
    assert_eq!(status(A::TolerantNeighborHash), Some(R::NeedsSemanticProof));
    assert!(
        registry
            .legal_candidates(*kind, call, &cpu)
            .contains(&RealizationFamily::ReferenceSequential)
    );
    assert!(
        registry
            .search_algorithm_reports(search, &TargetCapabilities::gpu_generic())
            .iter()
            .all(|r| r.readiness == R::UnsupportedTarget)
    );

    let plan = Engine::new().analyze_a3("1 3 5 I. 2 4").unwrap();
    let out = plan.result.unwrap();
    let op = &plan.operations[plan.values[out.0].producer.0];
    let OpKind::Basis {
        payload: ExecutionBasisPayload::LookupClassify { search },
        ..
    } = &op.kind
    else {
        panic!("expected interval lookup");
    };
    assert!(
        registry
            .search_algorithm_reports(search, &cpu)
            .iter()
            .all(|r| r.readiness == R::UnsupportedSearchForm)
    );
}

#[test]
fn jsource_search_report_retains_unproven_algorithm_options() {
    use rustj::lowering::{SearchAlgorithm as A, SearchAlgorithmReadiness as R};

    let registry = LoweringRegistry::a3_v0();
    for source in ["3 1 3 i. 3 4", "3 1 3 i: 3 4", "3 1 3 e. 3 4"] {
        let analysis = Engine::new().analyze_compilation(source).unwrap();
        let reports = registry
            .jsource_planning_reports(&analysis, &TargetCapabilities::cpu_baseline())
            .unwrap();
        let report = reports
            .iter()
            .find(|report| report.family == JsourceFamily::SearchAlgorithm)
            .expect("source graph did not retain the i. family");
        assert_eq!(report.state, JsourcePlanningState::NeedsSemanticProof);
        let linked = report
            .linked_calls
            .iter()
            .find(|call| !call.search_algorithms.is_empty())
            .expect("source/A3 search linkage missing");
        let state = |algorithm| {
            linked
                .search_algorithms
                .iter()
                .find(|r| r.algorithm == algorithm)
                .map(|r| r.readiness)
        };
        assert_eq!(state(A::Sequential), Some(R::Baseline));
        assert_eq!(state(A::DirectAddress), Some(R::RequiresExactScalarGuard));
        assert_eq!(
            state(A::ReverseQueryHash),
            Some(R::RequiresExactScalarGuard)
        );
        assert_eq!(state(A::TolerantNeighborHash), Some(R::NeedsSemanticProof));
        assert!(!analysis.jsource_opportunities.iter().any(|c| c.selected));
    }
}

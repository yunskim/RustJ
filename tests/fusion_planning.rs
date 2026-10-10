use rustj::{
    Engine,
    fusion_planning::FusionReadinessState,
    j_graph_fusion::{FusionRegistry, OBLIGATIONS, TargetQuery},
    j_graph_ir::ValueId,
    j_graph_work_depth::{Metric, OperatorComponent, OperatorCostModel},
    lowering::{BasisTargetFeasibility, LoweringRegistry, TargetCapabilities},
};

struct UnitWeights;
impl OperatorCostModel for UnitWeights {
    fn cost(&self, _: ValueId, _: Metric, _: OperatorComponent) -> Option<u64> {
        Some(1)
    }
}

#[test]
fn source_basis_capability_does_not_authorize_fusion_or_cost_improvement() {
    let graph = Engine::new().analyze_j_graph("(- @: *) 1 2 3").unwrap();
    let rules = FusionRegistry::default();
    let lowering = LoweringRegistry::a3_v0();
    let target = TargetCapabilities::cpu_baseline();
    let analysis = lowering.fusion_readiness(&graph, &rules, &target).unwrap();
    analysis.verify(&graph, &rules, &lowering, &target).unwrap();
    assert_eq!(analysis.reports.len(), 1);
    let report = &analysis.reports[0];
    assert_eq!(report.state, FusionReadinessState::AwaitingSemanticProofs);
    assert_eq!(
        report.fused_target_query,
        TargetQuery::DeferredUntilLegality
    );
    assert_eq!(report.unresolved_obligations, OBLIGATIONS);
    assert!(!report.selected);
    assert!(
        report
            .source_basis_feasibility
            .iter()
            .all(|q| q.layers.iter().all(|(_, f)| matches!(
                f,
                BasisTargetFeasibility::RequiresCallFacts
                    | BasisTargetFeasibility::Supported { .. }
            )))
    );
    let model = &analysis.comparisons.models[0];
    assert_eq!(
        analysis
            .comparisons
            .expressions
            .evaluate(model.source.work, &graph, &UnitWeights),
        Some(8)
    );
    assert_eq!(
        analysis
            .comparisons
            .expressions
            .evaluate(model.replacement.work, &graph, &UnitWeights),
        None
    );
}

#[test]
fn changed_target_or_lowering_registry_invalidates_readiness_report() {
    let graph = Engine::new().analyze_j_graph("(- @: *) 1 2 3").unwrap();
    let rules = FusionRegistry::default();
    let lowering = LoweringRegistry::a3_v0();
    let cpu = TargetCapabilities::cpu_baseline();
    let analysis = lowering.fusion_readiness(&graph, &rules, &cpu).unwrap();
    // Static metadata query only; no GPU runtime or kernel is executed.
    assert!(
        analysis
            .verify(
                &graph,
                &rules,
                &lowering,
                &TargetCapabilities::gpu_generic()
            )
            .is_err()
    );
    assert!(
        analysis
            .verify(&graph, &rules, &LoweringRegistry::default(), &cpu)
            .is_err()
    );
    let unsupported = LoweringRegistry::default()
        .fusion_readiness(&graph, &rules, &cpu)
        .unwrap();
    assert!(!unsupported.reports[0].selected);
    assert!(
        unsupported.reports[0]
            .source_basis_feasibility
            .iter()
            .all(|q| q
                .layers
                .iter()
                .all(|(_, f)| *f == BasisTargetFeasibility::Unsupported))
    );
}

#[test]
fn batch_cost_models_share_one_arena_and_do_not_promote_scan_to_execution() {
    let rules = FusionRegistry::default();
    let lowering = LoweringRegistry::a3_v0();
    let target = TargetCapabilities::cpu_baseline();
    for source in ["(- + *) 1 2 3 4", "0 1 1 0 ((*/)\\ @: =) 0 0 1 1"] {
        let graph = Engine::new().analyze_j_graph(source).unwrap();
        let analysis = lowering.fusion_readiness(&graph, &rules, &target).unwrap();
        analysis.verify(&graph, &rules, &lowering, &target).unwrap();
        assert_eq!(analysis.comparisons.models.len(), analysis.reports.len());
        assert_eq!(
            analysis.comparisons.expressions.nodes.len(),
            analysis.work_depth.expressions.nodes.len() + 3 * analysis.reports.len()
        );
        assert!(
            analysis
                .reports
                .iter()
                .all(|r| !r.selected && !r.unresolved_obligations.is_empty())
        );
        assert!(
            analysis
                .comparisons
                .models
                .iter()
                .all(|m| !m.improvement_proven)
        );
    }
}

#[test]
fn forged_selection_obligations_or_profitability_are_rejected() {
    let graph = Engine::new().analyze_j_graph("(- @: *) 1 2 3").unwrap();
    let rules = FusionRegistry::default();
    let lowering = LoweringRegistry::a3_v0();
    let target = TargetCapabilities::cpu_baseline();
    let analysis = lowering.fusion_readiness(&graph, &rules, &target).unwrap();
    let mut forged = analysis.clone();
    forged.reports[0].selected = true;
    assert!(forged.verify(&graph, &rules, &lowering, &target).is_err());
    let mut forged = analysis.clone();
    forged.reports[0].unresolved_obligations.clear();
    assert!(forged.verify(&graph, &rules, &lowering, &target).is_err());
    let mut forged = analysis.clone();
    forged.comparisons.models[0].improvement_proven = true;
    assert!(forged.verify(&graph, &rules, &lowering, &target).is_err());
    let graph = Engine::new().analyze_j_graph("c. 1").unwrap();
    assert!(
        lowering
            .fusion_readiness(&graph, &rules, &target)
            .unwrap()
            .reports
            .is_empty()
    );
}

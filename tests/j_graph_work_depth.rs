use rustj::{
    Engine,
    j_graph_ir::ValueId,
    j_graph_work_depth::{
        Metric, OperatorComponent, OperatorCostModel, WorkDepthExprId, WorkDepthExprNode,
    },
};

struct UnitWeights;
impl OperatorCostModel for UnitWeights {
    fn cost(&self, _: ValueId, _: Metric, _: OperatorComponent) -> Option<u64> {
        Some(1)
    }
}
struct UnknownWeights;
impl OperatorCostModel for UnknownWeights {
    fn cost(&self, _: ValueId, _: Metric, _: OperatorComponent) -> Option<u64> {
        None
    }
}

#[test]
fn map_pipeline_work_depth_are_symbolic_and_not_memory_or_latency() {
    let graph = Engine::new().analyze_j_graph("(- @: *) 1 2 3").unwrap();
    let analysis = graph.work_depth_analysis().unwrap();
    analysis.verify(&graph).unwrap();
    assert!(analysis.successful_path_model);
    assert_eq!(
        analysis
            .expressions
            .evaluate(analysis.total.work, &graph, &UnitWeights),
        Some(8)
    );
    assert_eq!(
        analysis
            .expressions
            .evaluate(analysis.total.depth, &graph, &UnitWeights),
        Some(8)
    );
    assert_eq!(
        analysis
            .expressions
            .evaluate(analysis.total.work, &graph, &UnknownWeights),
        None
    );
    assert_eq!(analysis.regions[0].operations.len(), 2);
    assert_eq!(
        analysis
            .expressions
            .evaluate(analysis.regions[0].expressions.work, &graph, &UnitWeights),
        Some(8)
    );
    struct DifferentWeights;
    impl OperatorCostModel for DifferentWeights {
        fn cost(&self, _: ValueId, metric: Metric, _: OperatorComponent) -> Option<u64> {
            Some(if metric == Metric::Work { 2 } else { 3 })
        }
    }
    assert_eq!(
        analysis
            .expressions
            .evaluate(analysis.total.work, &graph, &DifferentWeights),
        Some(16)
    );
    assert_eq!(
        analysis
            .expressions
            .evaluate(analysis.total.depth, &graph, &DifferentWeights),
        Some(24)
    );
}

#[test]
fn fork_depth_stays_ordered_and_nested_regions_do_not_double_count() {
    for (source, operations, expected) in
        [("(- + *) 1 2 3 4", 3, 15), ("((- @: *) + *) 1 2 3", 4, 16)]
    {
        let graph = Engine::new().analyze_j_graph(source).unwrap();
        let analysis = graph.work_depth_analysis().unwrap();
        analysis.verify(&graph).unwrap();
        assert_eq!(analysis.applied_operations.len(), operations);
        assert_eq!(
            analysis
                .expressions
                .evaluate(analysis.total.work, &graph, &UnitWeights),
            Some(expected)
        );
        assert_eq!(
            analysis
                .expressions
                .evaluate(analysis.total.depth, &graph, &UnitWeights),
            Some(expected)
        );
        assert!(
            !analysis
                .expressions
                .nodes
                .iter()
                .any(|n| matches!(n, WorkDepthExprNode::Max(_)))
        );
        let outer = analysis.regions.last().unwrap();
        assert_eq!(outer.operations.len(), operations);
        assert_eq!(
            analysis
                .expressions
                .evaluate(outer.expressions.work, &graph, &UnitWeights),
            Some(expected)
        );
    }
}

#[test]
fn reduction_empty_identity_and_singleton_keep_dispatch_cost() {
    for (noun, expected) in [
        ("0$0", 2),
        (",1", 1),
        ("1", 1),
        ("1 2 3", 3),
        ("2 3$i.6", 4),
    ] {
        let mut engine = Engine::new();
        engine.eval(&format!("wdinput=: {noun}")).unwrap();
        let graph = engine.analyze_j_graph("+/wdinput").unwrap();
        let analysis = graph.work_depth_analysis().unwrap();
        assert_eq!(
            analysis
                .expressions
                .evaluate(analysis.total.work, &graph, &UnitWeights),
            Some(expected),
            "{noun}"
        );
    }
}

#[test]
fn ordered_scan_identity_model_is_separate_from_source_window_execution() {
    for (noun, expected) in [
        ("0$0", 1),
        (",1", 2),
        ("0", 2),
        ("0 1 1", 6),
        ("2 0$0", 1),
        ("2 3$0 1", 10),
    ] {
        let mut engine = Engine::new();
        engine.eval(&format!("wdinput=: {noun}")).unwrap();
        let graph = engine.analyze_j_graph("(+/)\\ wdinput").unwrap();
        let analysis = graph.work_depth_analysis().unwrap();
        analysis.verify(&graph).unwrap();
        assert_eq!(
            analysis
                .expressions
                .evaluate(analysis.total.work, &graph, &UnitWeights),
            None
        );
        assert_eq!(analysis.ordered_scan_models.len(), 1);
        let model = &analysis.ordered_scan_models[0];
        assert_eq!(
            analysis
                .expressions
                .evaluate(model.expressions.work, &graph, &UnitWeights),
            Some(expected),
            "{noun}"
        );
        assert_eq!(
            analysis
                .expressions
                .evaluate(model.expressions.depth, &graph, &UnitWeights),
            Some(expected)
        );
        assert!(!model.identity.contract.parallel_prefix_authorized);
    }
}

#[test]
fn unknown_operator_rank_extent_and_assignment_are_not_zero_work() {
    for source in ["c. 0 1", "(+/ )\"1 2 3$i.6", "out=:1+2"] {
        let graph = Engine::new().analyze_j_graph(source).unwrap();
        let analysis = graph.work_depth_analysis().unwrap();
        assert_eq!(
            analysis
                .expressions
                .evaluate(analysis.total.work, &graph, &UnitWeights),
            None
        );
        assert_eq!(
            analysis
                .expressions
                .evaluate(analysis.total.depth, &graph, &UnitWeights),
            None
        );
    }
    let mut graph = Engine::new().analyze_j_graph("- 1 2 3").unwrap();
    let output = graph.result.unwrap();
    graph.nodes[output.0].facts.shape = None;
    graph.nodes[output.0].facts.rank = None;
    graph.nodes[output.0].analyzability =
        rustj::j_graph_ir::GraphAnalyzability::StaticWithUnknownFacts;
    let analysis = graph.work_depth_analysis().unwrap();
    assert_eq!(
        analysis
            .expressions
            .evaluate(analysis.total.work, &graph, &UnitWeights),
        None
    );
}

#[test]
fn duplication_hypothesis_accounts_for_extra_work_without_authorizing_it() {
    let graph = Engine::new().analyze_j_graph("- 1 2 3").unwrap();
    let analysis = graph.work_depth_analysis().unwrap();
    let model = analysis
        .single_operation_duplication(graph.result.unwrap(), 2)
        .unwrap();
    model.verify(&analysis, &graph).unwrap();
    assert_eq!(
        model
            .expressions
            .evaluate(model.additional.work, &graph, &UnitWeights),
        Some(8)
    );
    assert_eq!(
        model
            .expressions
            .evaluate(model.additional.depth, &graph, &UnitWeights),
        Some(8)
    );
    assert!(!model.duplication_authorized);
    assert!(
        analysis
            .single_operation_duplication(ValueId(0), 2)
            .is_err()
    );
    let mut forged = model.clone();
    forged.duplication_authorized = true;
    assert!(forged.verify(&analysis, &graph).is_err());
    let zero = analysis
        .single_operation_duplication(graph.result.unwrap(), 0)
        .unwrap();
    assert_eq!(
        zero.expressions
            .evaluate(zero.additional.work, &graph, &UnknownWeights),
        Some(0)
    );
}

#[test]
fn provenance_expression_cycles_forged_totals_and_overflow_fail_conservatively() {
    let graph = Engine::new().analyze_j_graph("- 1 2 3").unwrap();
    let analysis = graph.work_depth_analysis().unwrap();
    let mut forged = analysis.clone();
    forged.expressions.nodes[0] = WorkDepthExprNode::Sum(vec![WorkDepthExprId(0)]);
    assert!(forged.verify(&graph).is_err());
    let mut forged = analysis.clone();
    forged.total.work = WorkDepthExprId(0);
    assert!(forged.verify(&graph).is_err());
    struct OverflowWeights;
    impl OperatorCostModel for OverflowWeights {
        fn cost(&self, _: ValueId, _: Metric, _: OperatorComponent) -> Option<u64> {
            Some(u64::MAX)
        }
    }
    assert_eq!(
        analysis
            .expressions
            .evaluate(analysis.total.work, &graph, &OverflowWeights),
        None
    );
    assert_eq!(
        analysis
            .expressions
            .evaluate(WorkDepthExprId(usize::MAX), &graph, &UnitWeights),
        None
    );
}

#[test]
fn fusion_model_preserves_source_work_without_inventing_an_improvement() {
    let graph = Engine::new().analyze_j_graph("(- + *) 1 2 3 4").unwrap();
    let analysis = graph.work_depth_analysis().unwrap();
    let registry = rustj::j_graph_fusion::FusionRegistry::default();
    let fusion = graph.fusion_analysis(&registry).unwrap();
    let i = fusion
        .candidates
        .iter()
        .position(|c| c.rule == rustj::j_graph_fusion::FusionRuleId::CommonInputMaps)
        .unwrap();
    let model = analysis
        .fusion_envelope_model(&fusion, &registry, i, &graph)
        .unwrap();
    model.verify(&analysis, &fusion, &registry, &graph).unwrap();
    assert_eq!(model.operations.len(), 2);
    assert_eq!(model.retained_values.len(), 2);
    assert_eq!(
        model
            .expressions
            .evaluate(model.source.work, &graph, &UnitWeights),
        Some(10)
    );
    assert_eq!(
        model
            .expressions
            .evaluate(model.source.depth, &graph, &UnitWeights),
        Some(10)
    );
    assert_eq!(
        model
            .expressions
            .evaluate(model.replacement.work, &graph, &UnitWeights),
        None
    );
    assert!(!model.improvement_proven);
    let mut forged = model.clone();
    forged.improvement_proven = true;
    assert!(
        forged
            .verify(&analysis, &fusion, &registry, &graph)
            .is_err()
    );
}

use rustj::{
    Engine,
    j_graph_fusion::{FusionLegality, FusionRegistry, FusionRuleId},
};

#[test]
fn registry_rejects_duplicates_conflicts_versions_and_missing_obligations() {
    let registry = FusionRegistry::default();
    assert_eq!(registry.rules().len(), 4);
    assert_eq!(registry.rules()[0].id.stable_id(), "fusion.map-map");
    let rules = registry.rules().to_vec();
    let mut invalid = rules.clone();
    invalid.push(rules[0].clone());
    assert!(FusionRegistry::new(invalid).is_err());
    let mut invalid = rules.clone();
    invalid[1].pattern = rules[0].pattern;
    assert!(FusionRegistry::new(invalid).is_err());
    let mut invalid = rules.clone();
    invalid[0].version = 2;
    assert!(FusionRegistry::new(invalid).is_err());
    let mut invalid = rules;
    invalid[0].obligations.pop();
    assert!(FusionRegistry::new(invalid).is_err());
}

#[test]
fn vertical_patterns_discover_source_envelopes_without_execution_selection() {
    let registry = FusionRegistry::default();
    for (source, expected) in [
        ("(- @: *) 1 2 3", FusionRuleId::MapMap),
        ("(+/ @: *) 1 2 3", FusionRuleId::MapReduce),
        ("0 1 1 0 ((*/)\\ @: =) 0 0 1 1", FusionRuleId::MapScan),
    ] {
        let graph = Engine::new().analyze_j_graph(source).unwrap();
        let analysis = graph.fusion_analysis(&registry).unwrap();
        analysis.verify(&graph, &registry).unwrap();
        let c = analysis
            .candidates
            .iter()
            .find(|c| c.rule == expected)
            .expect(source);
        assert_eq!(c.replacement.operations.len(), 2);
        assert_eq!(c.provenance.len(), 2);
        assert!(c.replacement.operations[0].0 < c.replacement.operations[1].0);
        assert_eq!(c.legality, FusionLegality::Unknown);
        assert!(!c.selected && !c.resource_transfer_proven);
        assert_eq!(c.scan_identity.is_some(), expected == FusionRuleId::MapScan);
        let mut forged = analysis.clone();
        forged.candidates[0].selected = true;
        assert!(forged.verify(&graph, &registry).is_err());
        let mut forged = analysis.clone();
        forged.candidates[0].provenance[0].source_span = 0..0;
        assert!(forged.verify(&graph, &registry).is_err());
    }
}

#[test]
fn horizontal_candidate_keeps_h_f_order_join_and_external_consumers() {
    let registry = FusionRegistry::default();
    let graph = Engine::new().analyze_j_graph("(- + *) 1 2 3").unwrap();
    let analysis = graph.fusion_analysis(&registry).unwrap();
    let c = analysis
        .candidates
        .iter()
        .find(|c| c.rule == FusionRuleId::CommonInputMaps)
        .unwrap();
    assert!(c.region.is_some());
    assert_eq!(c.replacement.outputs.len(), 2);
    assert!(c.replacement.fanout.iter().all(|f| f.external_uses == 1));
    assert!(c.replacement.operations[0].0 < c.replacement.operations[1].0);
    assert!(!c.replacement.operations.contains(&graph.result.unwrap()));
    let mut forged = analysis.clone();
    let c = forged
        .candidates
        .iter_mut()
        .find(|c| c.rule == FusionRuleId::CommonInputMaps)
        .unwrap();
    c.replacement.operations.swap(0, 1);
    assert!(forged.verify(&graph, &registry).is_err());
    let mut forged = analysis.clone();
    forged.candidates[0].replacement.retained_values.clear();
    assert!(forged.verify(&graph, &registry).is_err());
}

#[test]
fn repeated_slots_deduplicate_candidate_but_retain_every_use() {
    let registry = FusionRegistry::default();
    let mut graph = Engine::new().analyze_j_graph("(- + *) 1 2 3").unwrap();
    graph.regions.clear();
    let join = graph.result.unwrap();
    let producer = if let rustj::j_graph_ir::NodeKind::Apply { left, right, .. } =
        &mut graph.nodes[join.0].kind
    {
        *left = Some(*right);
        *right
    } else {
        panic!()
    };
    let analysis = graph.fusion_analysis(&registry).unwrap();
    assert_eq!(analysis.candidates.len(), 1);
    let fanout = analysis.candidates[0]
        .replacement
        .fanout
        .iter()
        .find(|f| f.value == producer)
        .unwrap();
    assert_eq!(fanout.internal_occurrences, 2);
    assert_eq!(fanout.source_uses, 2);
    assert_eq!(fanout.external_uses, 0);
    graph.result = Some(producer);
    let analysis = graph.fusion_analysis(&registry).unwrap();
    let c = &analysis.candidates[0];
    let fanout = c
        .replacement
        .fanout
        .iter()
        .find(|f| f.value == producer)
        .unwrap();
    assert_eq!(fanout.source_uses, 3);
    assert_eq!(fanout.external_uses, 1);
    assert!(c.replacement.outputs.contains(&producer));
    assert!(c.replacement.retained_values.contains(&producer));
}

#[test]
fn unknown_numeric_scan_and_noun_left_are_not_horizontal_fusion() {
    let registry = FusionRegistry::default();
    for source in ["(c. + *) 1 2 3", "(3 + *) 1 2 3", "([: + *) 1 2 3"] {
        let graph = Engine::new().analyze_j_graph(source).unwrap();
        let analysis = graph.fusion_analysis(&registry).unwrap();
        assert!(
            !analysis
                .candidates
                .iter()
                .any(|c| c.rule == FusionRuleId::CommonInputMaps)
        );
    }
    let graph = Engine::new().analyze_j_graph("(+/\\ @: *) 2 3 4").unwrap();
    assert!(
        !graph
            .fusion_analysis(&registry)
            .unwrap()
            .candidates
            .iter()
            .any(|c| c.rule == FusionRuleId::MapScan)
    );
    assert!(
        graph
            .fusion_analysis(&FusionRegistry::new(vec![]).unwrap())
            .unwrap()
            .candidates
            .is_empty()
    );
}

use std::collections::HashSet;

use rustj::{
    Engine,
    j_graph_jsource::{
        family_rule, DiscoveryCoverage, JsourceFamily, OpportunityLegality,
        JSOURCE_CATALOG_VERSION, JSOURCE_FAMILY_RULES, JSOURCE_SOURCE_PIN,
    },
};

fn graph(source: &str) -> rustj::j_graph_ir::Plan {
    Engine::new().analyze_j_graph(source).unwrap()
}

#[test]
fn reviewed_rules_have_unique_identifiers_and_explicit_ownership() {
    assert_eq!(JSOURCE_CATALOG_VERSION, 2);
    assert_eq!(
        JSOURCE_SOURCE_PIN,
        "13994ffa1ed5f06f79fad6e9822a7ed2d29b1528"
    );
    let mut ids = HashSet::new();
    let mut families = HashSet::new();
    for r in JSOURCE_FAMILY_RULES {
        assert!(ids.insert(r.stable_id));
        assert!(families.insert(r.family));
        assert!(r.source_file.starts_with("jsrc/"));
        assert!(!r.source_symbol.is_empty());
        assert!(!r.proof_requirements.is_empty());
        assert_eq!(family_rule(r.family), r);
    }
    for family in [
        JsourceFamily::GroupAggregate,
        JsourceFamily::MatrixContraction,
        JsourceFamily::GradeRanking,
        JsourceFamily::ResultAssemblyDemand,
    ] {
        assert_eq!(
            family_rule(family).discovery,
            DiscoveryCoverage::AwaitingFrontendOrFacts
        );
    }
    assert_eq!(
        family_rule(JsourceFamily::MapReduceStreaming).discovery,
        DiscoveryCoverage::ExistingAnalyzer
    );
    assert_eq!(
        family_rule(JsourceFamily::NameLookupCache).discovery,
        DiscoveryCoverage::DownstreamOnly
    );
}

#[test]
fn source_idioms_are_detected_without_fabricating_equivalence_or_execution() {
    for (source, expected) in [
        ("+/1 2 3", JsourceFamily::ReductionFastPath),
        ("(+/)\\ 1 2 3", JsourceFamily::WindowAlgorithm),
        ("1 { 10 20 30", JsourceFamily::GatherCopyOrView),
        ("|. 1 2 3", JsourceFamily::ReindexCopyOrView),
        ("'ana' E. 'banana'", JsourceFamily::SearchAlgorithm),
    ] {
        let source_graph = graph(source);
        let opportunities = source_graph.jsource_opportunities();
        let c = opportunities.iter().find(|c| c.family == expected)
            .unwrap_or_else(|| panic!("no {expected:?} from {source}"));
        assert_eq!(c.legality, OpportunityLegality::AwaitingSemanticProofs);
        assert!(!c.selected);
        assert_eq!(c.source_span, source_graph.nodes[c.source_value.0].span);
        assert_eq!(c.source_facts, source_graph.nodes[c.source_value.0].facts);
        assert_eq!(c.rule().discovery, DiscoveryCoverage::AnalysisOnly);
        c.verify(&source_graph).unwrap();
        assert!(opportunities.iter().all(|c| !c.selected));
    }
}

#[test]
fn interval_index_is_separate_from_general_index_of_and_monadic_index_space() {
    let g = graph("1 3 5 I. 2 4");
    let c = g.jsource_opportunities();
    assert!(c.iter().any(|c| c.family == JsourceFamily::IntervalLookup));
    assert!(!c.iter().any(|c| c.family == JsourceFamily::SearchAlgorithm));

    let g = graph("i. 4");
    assert!(!g.jsource_opportunities().iter()
        .any(|c| c.family == JsourceFamily::SearchAlgorithm));
}

#[test]
fn provenance_and_unknown_legality_cannot_be_forged() {
    let g = graph("1 { 10 20 30");
    let original = g.jsource_opportunities().pop().unwrap();
    let mut modified = original.clone();
    modified.selected = true;
    assert!(modified.verify(&g).is_err());
    modified = original.clone();
    modified.source_span = 999..1000;
    assert!(modified.verify(&g).is_err());
    modified = original.clone();
    modified.source_facts.rank = Some(999);
    assert!(modified.verify(&g).is_err());

    // These families may have upstream fast paths but are not supported
    // as graph rewrites merely by registering their source evidence.
    for family in [
        JsourceFamily::GroupAggregate,
        JsourceFamily::MatrixContraction,
        JsourceFamily::GradeRanking,
        JsourceFamily::MapReduceStreaming,
        JsourceFamily::NameLookupCache,
    ] {
        assert!(!g.jsource_opportunities().iter().any(|c| c.family == family));
    }
}

#[test]
fn compilation_bundle_preserves_independent_existing_rewrite_and_fusion_analysis() {
    let compilation = Engine::new()
        .analyze_compilation("'ana' E. 'banana'")
        .unwrap();
    assert_eq!(compilation.graph_rewrites.len(), 1);
    assert_eq!(compilation.jsource_opportunities.len(), 1);
    assert_eq!(
        compilation.jsource_opportunities[0].family,
        JsourceFamily::SearchAlgorithm
    );
    compilation.jsource_opportunities[0]
        .verify(&compilation.j_graph)
        .unwrap();
    assert!(compilation.jsource_opportunities.iter().all(|c| !c.selected));

    let g = graph("(+/ @: *) 1 2 3");
    let fusion = g.fusion_analysis(&Default::default()).unwrap();
    assert!(fusion.candidates.iter().any(|c| {
        c.rule == rustj::j_graph_fusion::FusionRuleId::MapReduce
    }));
    assert!(g.jsource_opportunities().iter().all(|c| {
        c.family != JsourceFamily::MapReduceStreaming
    }));
}

#[test]
fn mean_fork_uses_derived_verb_identity_and_preserves_source_order() {
    let g = graph("(+/ % #) 1 2 3 4");
    let opportunities = g.jsource_opportunities();
    let mean = opportunities.iter()
        .find(|c| c.family == JsourceFamily::MeanIdiom)
        .expect("exact mean fork should be recognized");
    assert_eq!(mean.legality, OpportunityLegality::AwaitingSemanticProofs);
    assert!(!mean.selected);
    mean.verify(&g).unwrap();
    assert!(g.regions.iter().any(|r| r.result == mean.source_value));

    // A general fork is not evidence of a mean; syntax must match all
    // three component verb identities and the Insert-derived left operand.
    for source in ["(+/ + #) 1 2 3", "(-/ % #) 1 2 3", "(- + *) 1 2 3"] {
        let g = graph(source);
        assert!(
            !g.jsource_opportunities().iter()
                .any(|c| c.family == JsourceFamily::MeanIdiom),
            "{source} was incorrectly treated as mean"
        );
    }
}

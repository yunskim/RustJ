use rustj::{
    Engine,
    j_graph_memory::{
        AtomRepresentation, MaterializationOpportunity,
    },
    types::DType,
};

struct Dense64;

impl AtomRepresentation for Dense64 {
    fn bytes_per_atom(&self, dtype: DType) -> Option<usize> {
        match dtype {
            DType::Bool => Some(1),
            DType::Int | DType::Float => Some(8),
            DType::Char => Some(1),
            // Variable-width / representation-dependent logical atoms remain unknown.
            DType::Complex
            | DType::ExtendedInt
            | DType::Rational
            | DType::Symbol
            | DType::Boxed => None,
        }
    }
}

#[test]
fn pipeline_memory_analysis_knows_extents_and_materialization_candidates() {
    let graph = Engine::new()
        .analyze_j_graph("(|. @: , @: |.) 1 2 3")
        .unwrap();
    let memory = graph.static_memory_analysis();

    let (_, region) = graph
        .region_for_result(graph.result.unwrap())
        .expect("pipeline region");
    let rustj::j_graph_ir::RegionKind::Pipeline { stage_results } = &region.kind else {
        panic!()
    };

    for stage in stage_results {
        let extent = memory.extent(*stage).expect("known stage extent");
        assert_eq!(extent.shape, vec![3]);
        assert_eq!(extent.atoms, 3);
        assert_eq!(memory.represented_bytes(*stage, &Dense64), Some(24));
    }

    for intermediate in stage_results.iter().take(stage_results.len() - 1) {
        assert!(memory.opportunities.iter().any(|item| {
            item.value == *intermediate
                && item.kind == MaterializationOpportunity::PipelineIntermediate
        }));
    }
}

#[test]
fn fork_memory_analysis_extends_shared_input_lifetime_to_the_join() {
    let graph = Engine::new()
        .analyze_j_graph("(+/ % #) 1 2 3 4")
        .unwrap();
    let memory = graph.static_memory_analysis();

    let (_, region) = graph
        .region_for_result(graph.result.unwrap())
        .expect("fork region");
    let rustj::j_graph_ir::RegionKind::Fork {
        join_result,
        live_across,
        ..
    } = &region.kind
    else {
        panic!()
    };

    let input = live_across[0];
    let range = &memory.live_ranges[input.0];
    assert!(range.last_use >= join_result.0);
    assert!(memory.opportunities.iter().any(|item| {
        item.value == input
            && item.kind == MaterializationOpportunity::RetainedAcrossBranch
    }));
}

#[test]
fn virtual_reindex_is_a_memory_opportunity_not_an_allocation_command() {
    let graph = Engine::new().analyze_j_graph("|. 1 2 3").unwrap();
    let result = graph.result.unwrap();
    let memory = graph.static_memory_analysis();

    assert!(memory.opportunities.iter().any(|item| {
        item.value == result && item.kind == MaterializationOpportunity::VirtualView
    }));
    assert_eq!(memory.extent(result).unwrap().atoms, 3);
}

#[test]
fn byte_accounting_requires_an_explicit_representation_model() {
    let graph = Engine::new().analyze_j_graph("1 2 3").unwrap();
    let result = graph.result.unwrap();
    let memory = graph.static_memory_analysis();

    assert_eq!(memory.extent(result).unwrap().atoms, 3);
    assert_eq!(memory.represented_bytes(result, &Dense64), Some(24));
    assert!(memory.graph_order_peak_materialized_bytes(&Dense64).is_some());
}

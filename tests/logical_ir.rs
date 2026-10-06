use rustj::{
    Engine,
    analysis::ExecutionBasisKind,
    facts::RepresentationClassFact,
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
            kind: ExecutionBasisKind::Elementwise,
            ..
        }
    ));
}

#[test]
fn logical_facts_use_semantic_representation_class_not_physical_layout() {
    let plan = Engine::new().analyze_a3("1 2 3").unwrap();
    plan.verify().unwrap();

    let result = plan.result.unwrap();
    assert_eq!(
        plan.values[result.0].facts.representation_class,
        RepresentationClassFact::Dense
    );
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
    assert!(matches!(
        check.constraint,
        Constraint::PrefixAgreement { .. }
    ));

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
    assert!(matches!(
        check.constraint,
        Constraint::IndicesInBounds { .. }
    ));

    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    assert!(matches!(
        plan.operations[producer.0].kind,
        OpKind::Basis {
            kind: ExecutionBasisKind::Gather,
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
    use rustj::logical_ir::{AxisRole, ExecutionBasisPayload, IterationAxisKind, ReductionAxis};

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

    assert_eq!(*kind, ExecutionBasisKind::Reduce);
    assert_eq!(
        *payload,
        ExecutionBasisPayload::Reduce {
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

    assert_eq!(*kind, ExecutionBasisKind::CellApply);
    assert_eq!(
        call.execution_basis.layers,
        vec![ExecutionBasisKind::CellApply, ExecutionBasisKind::Reduce]
    );
    assert_eq!(call.iteration_domain.axes.len(), 1);
    assert_eq!(call.iteration_domain.axes[0].extent, Some(2));
    assert_eq!(
        call.iteration_domain.axes[0].kind,
        IterationAxisKind::Parallel
    );
    assert_eq!(call.iteration_domain.axes[0].role, AxisRole::Frame);
}

#[test]
fn static_reindex_payload_preserves_the_reindex_family() {
    use rustj::logical_ir::{ExecutionBasisPayload, ReindexKind};

    let plan = Engine::new().analyze_a3("|.1 2 3").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    let OpKind::Basis { payload, .. } = &plan.operations[producer.0].kind else {
        panic!("reindex basis op")
    };
    assert_eq!(
        *payload,
        ExecutionBasisPayload::StaticReindex {
            kind: ReindexKind::Reverse
        }
    );
}

#[test]
fn verifier_rejects_a_basis_payload_that_no_longer_matches_the_call() {
    use rustj::logical_ir::ExecutionBasisPayload;

    let mut plan = Engine::new().analyze_a3("|.1 2 3").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    let OpKind::Basis { payload, .. } = &mut plan.operations[producer.0].kind else {
        panic!("reindex basis op")
    };
    *payload = ExecutionBasisPayload::Elementwise;

    let error = plan.verify().unwrap_err();
    assert_eq!(error.operation, Some(producer));
    assert!(error.message.contains("basis payload"));
}

#[test]
fn a3_v0_has_explicit_single_function_region_block_and_return() {
    use rustj::logical_ir::{BlockId, FunctionId, RegionId, Terminator};

    let plan = Engine::new().analyze_a3("1+2").unwrap();
    assert_eq!(plan.entry, FunctionId(0));
    assert_eq!(plan.functions.len(), 1);
    assert_eq!(plan.functions[0].body, RegionId(0));
    assert_eq!(plan.regions.len(), 1);
    assert_eq!(plan.regions[0].blocks, vec![BlockId(0)]);
    assert_eq!(plan.blocks.len(), 1);
    assert_eq!(plan.blocks[0].operations, 0..plan.operations.len());
    assert_eq!(plan.blocks[0].terminator, Terminator::Return(plan.result));
    plan.verify().unwrap();
}

#[test]
fn verifier_rejects_an_entry_block_that_does_not_cover_the_operation_sequence() {
    let mut plan = Engine::new().analyze_a3("1+2").unwrap();
    plan.blocks[0].operations = 0..1;
    let error = plan.verify().unwrap_err();
    assert!(error.message.contains("complete operation sequence"));
}

#[test]
fn a3_header_records_schema_and_registry_provenance() {
    use rustj::logical_ir::A3_SCHEMA_VERSION;

    let plan = Engine::new().analyze_a3("1+2").unwrap();
    assert_eq!(plan.header.schema, A3_SCHEMA_VERSION);
    assert_eq!(
        plan.header.provenance.primitive_registry_version,
        rustj::primitive::REGISTRY_VERSION
    );
    assert!(!plan.header.provenance.compiler_version.is_empty());
}

#[test]
fn verifier_rejects_an_unknown_a3_schema() {
    let mut plan = Engine::new().analyze_a3("1+2").unwrap();
    plan.header.schema.major = plan.header.schema.major.saturating_add(1);
    let error = plan.verify().unwrap_err();
    assert!(error.message.contains("schema version"));
}

#[test]
fn semantic_capability_view_hides_storage_layout_of_call_metadata() {
    use rustj::{
        analysis::{AccessFact, AccessRelation},
        logical_ir::{AxisRole, SemanticCapabilityView},
    };

    let plan = Engine::new().analyze_a3("+/1 2 3").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    let view = plan.operation_view(producer).unwrap();

    assert_eq!(view.result_facts().unwrap().shape, Some(vec![]));
    assert_eq!(
        view.access_fact(),
        Some(AccessFact::Known(AccessRelation::ReduceLeadingAxis))
    );
    assert!(view.effect_summary().unwrap().is_pure());
    assert!(view.possible_errors().may_raise());
    assert_eq!(
        view.iteration_domain().unwrap().axes[0].role,
        AxisRole::Reduction
    );
    assert!(view.destination_relation().is_some());
}

#[test]
fn a3_preserves_structural_opportunities_after_flattening() {
    use rustj::opportunity::{OpportunitySource, StructuralTopology};

    let plan = Engine::new().analyze_a3("(|. @: , @: |.) 1 2 3").unwrap();
    plan.verify().unwrap();

    assert_eq!(plan.opportunities.len(), 1);
    let opportunity = &plan.opportunities[0];
    assert_eq!(opportunity.source, OpportunitySource::Atop);
    let StructuralTopology::Pipeline { stage_results, .. } = &opportunity.topology else {
        panic!("expected pipeline opportunity")
    };
    assert_eq!(stage_results.len(), 3);
    for value in stage_results {
        assert!(value.0 < plan.values.len());
    }
}

#[test]
fn incremental_a3_projection_keeps_roles_discovered_by_later_consumers() {
    use rustj::facts::ValueRole;

    let plan = Engine::new().analyze_a3("i.2 3").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    let OpKind::Basis { call, .. } = &plan.operations[producer.0].kind else {
        panic!("index-space basis op")
    };

    assert!(
        plan.values[call.right.0]
            .roles
            .contains(ValueRole::ShapeVector),
        "the literal shape value must retain the role added by its later consumer"
    );
    plan.verify().unwrap();
}

#[test]
fn direct_a3_lowering_preserves_graph_provenance_versions_and_check_order() {
    let source = "a=:1 2+1 2 3";
    let engine = Engine::new();
    let graph = engine.analyze_j_graph(source).unwrap();
    let plan = engine.analyze_a3(source).unwrap();
    plan.verify().unwrap();

    assert_eq!(plan.source, graph.source);
    assert_eq!(plan.j_graph_node_count, graph.nodes.len());
    assert_eq!(plan.j_graph_region_count, graph.regions.len());

    for operation in &plan.operations {
        if let Some(origin) = operation.j_origin {
            assert_eq!(
                operation.span, graph.nodes[origin.0].span,
                "A3 operation span must come from its J Graph origin"
            );
        }
    }

    let graph_write = graph.write.as_ref().expect("graph write");
    let logical_write = plan.write.as_ref().expect("logical write");
    assert_eq!(logical_write.previous, graph_write.previous);
    assert_eq!(logical_write.proposed, graph_write.proposed);
    assert_eq!(logical_write.span, graph_write.span);
    assert_eq!(plan.symbols[logical_write.symbol.0].name, graph_write.name);

    let check_id = plan
        .operations
        .iter()
        .position(|operation| matches!(operation.kind, OpKind::SemanticCheck(_)))
        .expect("prefix agreement check");
    let result = plan.result.expect("result");
    let producer = plan.values[result.0].producer;
    assert_eq!(
        plan.operations[producer.0].order_after,
        Some(rustj::logical_ir::OpId(check_id)),
        "observable semantic check must remain ordered before the value-producing call"
    );
    assert_eq!(logical_write.after, Some(producer));
}


#[test]
fn a3_lookup_classify_keeps_j_search_meaning_and_provenance() {
    use rustj::logical_ir::{
        ExecutionBasisPayload, SearchComparison, SearchOutputKind,
    };
    for (source, output, comparison) in [
        ("3 1 3 i. 3 4", SearchOutputKind::FirstIndex, SearchComparison::JEquality),
        ("3 1 3 i: 3 4", SearchOutputKind::LastIndex, SearchComparison::JEquality),
        ("3 1 3 e. 3 4", SearchOutputKind::MembershipMask, SearchComparison::JEquality),
        ("1 3 5 I. 2 4", SearchOutputKind::IntervalIndex, SearchComparison::JOrderedInterval),
    ] {
        let plan = Engine::new().analyze_a3(source).unwrap();
        plan.verify().unwrap();
        let result = plan.result.unwrap();
        let op = &plan.operations[plan.values[result.0].producer.0];
        let OpKind::Basis { payload: ExecutionBasisPayload::LookupClassify { search }, call, .. } = &op.kind else {
            panic!("not a typed search basis: {source}");
        };
        assert_eq!(search.output, output);
        assert_eq!(search.comparison, comparison);
        if output == SearchOutputKind::MembershipMask {
            // e. is "x belongs to y": index right, query left.
            assert_eq!(search.indexed, Some(call.right));
            assert_eq!(search.queried, call.left.unwrap());
        } else {
            // i./i:/I. index left, query right.
            assert_eq!(search.indexed, call.left);
            assert_eq!(search.queried, call.right);
        }
        assert_eq!(search.rank_boundary, call.instantiation.rank_boundary);
    }
}

#[test]
fn a3_verifier_rejects_forged_search_meaning() {
    use rustj::logical_ir::{ExecutionBasisPayload, SearchOutputKind};

    let mut plan = Engine::new().analyze_a3("3 1 3 i. 3").unwrap();
    let result = plan.result.unwrap();
    let op_id = plan.values[result.0].producer;
    let OpKind::Basis { payload: ExecutionBasisPayload::LookupClassify { search }, .. } =
        &mut plan.operations[op_id.0].kind else {
        panic!("expected typed lookup");
    };
    search.output = SearchOutputKind::LastIndex;
    let err = plan.verify().unwrap_err();
    assert_eq!(err.operation, Some(op_id));
    assert!(err.message.contains("basis payload"));
}

use rustj::{
    Engine,
    contracts::{Effect, Overflow},
    execution_semantics::{AccessFact, AccessRelation, CallTarget, ExecutionBasisKind, Scope},
    logical_ir::{CallOp, OpId, OpKind, Plan, ValueId},
    semantic::{FunctionHead, NameVersion},
};

fn operation_for_value(plan: &Plan, value: ValueId) -> &rustj::logical_ir::Operation {
    let producer = plan.values[value.0].producer;
    &plan.operations[producer.0]
}

fn call_for_value(plan: &Plan, value: ValueId) -> &CallOp {
    match &operation_for_value(plan, value).kind {
        OpKind::Basis { call, .. } | OpKind::SemanticCall(call) => call,
        _ => panic!("expected call-producing value"),
    }
}

fn call_for_value_mut(plan: &mut Plan, value: ValueId) -> &mut CallOp {
    let producer = plan.values[value.0].producer;
    match &mut plan.operations[producer.0].kind {
        OpKind::Basis { call, .. } | OpKind::SemanticCall(call) => call,
        _ => panic!("expected call-producing value"),
    }
}

fn result_call(plan: &Plan) -> &CallOp {
    call_for_value(plan, plan.result.expect("plan result"))
}

fn ordered_after(plan: &Plan, later: OpId, earlier: OpId) -> bool {
    let mut cursor = plan.operations[later.0].order_after;
    let mut remaining = plan.operations.len();
    while let Some(operation) = cursor {
        if operation == earlier {
            return true;
        }
        if remaining == 0 {
            return false;
        }
        remaining -= 1;
        cursor = plan.operations[operation.0].order_after;
    }
    false
}

fn call_operations(plan: &Plan) -> Vec<(OpId, &rustj::logical_ir::Operation, &CallOp)> {
    plan.operations
        .iter()
        .enumerate()
        .filter_map(|(index, operation)| {
            let call = match &operation.kind {
                OpKind::Basis { call, .. } | OpKind::SemanticCall(call) => call,
                _ => return None,
            };
            Some((OpId(index), operation, call))
        })
        .collect()
}

#[test]
fn analysis_does_not_execute_or_commit() {
    let mut e = Engine::new();
    e.eval("a=:7").unwrap();
    let stats = e.output_cache_stats();
    let p = e.analyze_a3("a=:i.9223372036854775807").unwrap();
    assert_eq!(e.binding_version("a"), Some(NameVersion(1)));
    assert_eq!(e.output_cache_stats(), stats);
    let write = p.write.as_ref().unwrap();
    assert_eq!(write.previous, Some(NameVersion(1)));
    assert_eq!(write.proposed, NameVersion(2));
    assert_eq!(Some(write.value), p.result);
    assert_eq!(write.after, Some(p.values[write.value.0].producer));
    assert!(e.analyze_a3("'a'+1").is_ok());
    assert!(e.analyze_a3("NB. empty").unwrap().operations.is_empty());
}

#[test]
fn noun_versions_and_dynamic_calls_are_distinct() {
    let mut e = Engine::new();
    e.eval("a=:3").unwrap();
    e.eval("f=:+").unwrap();
    let p = e.analyze_a3("out=:f a+a").unwrap();
    assert_eq!(p.symbols.iter().filter(|s| s.name == "a").count(), 1);
    assert!(p.symbols.iter().all(|s| s.scope == Scope::CurrentGlobal));

    let reads: Vec<_> = p
        .operations
        .iter()
        .filter_map(|operation| match operation.kind {
            OpKind::ReadNoun { symbol, version } => Some((symbol, version)),
            _ => None,
        })
        .collect();
    assert_eq!(reads.len(), 2);
    assert_eq!(reads[0], reads[1]);
    assert_eq!(reads[0].1, NameVersion(1));

    let call = result_call(&p);
    let CallTarget::Dynamic(id) = call.callable.target else {
        panic!()
    };
    assert_eq!(p.symbols[id.0].name, "f");
    assert_eq!(call.contract.effect, Effect::Unknown);

    e.eval("a=:9").unwrap();
    e.eval("f=:*").unwrap();
    assert_eq!(reads[0].1, NameVersion(1));

    let next = e.analyze_a3("a").unwrap();
    let result = next.result.unwrap();
    assert!(matches!(
        operation_for_value(&next, result).kind,
        OpKind::ReadNoun {
            version: NameVersion(2),
            ..
        }
    ));
}

#[test]
fn order_edges_preserve_competing_errors_and_inputs() {
    let source = "('a'+1)+(1 2+1 2 3)";
    let p = Engine::new().analyze_a3(source).unwrap();
    let calls = call_operations(&p);

    assert_eq!(calls.len(), 3);
    assert_eq!(&source[calls[0].1.span.clone()], "1 2+1 2 3");
    assert_eq!(&source[calls[1].1.span.clone()], "'a'+1");
    assert!(ordered_after(&p, calls[1].0, calls[0].0));
    assert!(ordered_after(&p, calls[2].0, calls[1].0));

    for (index, operation) in p.operations.iter().enumerate() {
        if let Some(before) = operation.order_after {
            assert!(before.0 < index);
        }
        if let OpKind::Basis { call, .. } | OpKind::SemanticCall(call) = &operation.kind {
            assert!(p.values[call.right.0].producer.0 < index);
            if let Some(left) = call.left {
                assert!(p.values[left.0].producer.0 < index);
            }
        }
    }
}

#[test]
fn primitive_and_derived_contracts_are_conservative() {
    let e = Engine::new();

    let p = e.analyze_a3("1+2").unwrap();
    assert_eq!(result_call(&p).contract.overflow, Overflow::WholeResultPromotion);

    let p = e.analyze_a3("+/1 2").unwrap();
    assert_eq!(
        result_call(&p).callable.semantic.head,
        FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert)
    );

    for source in ["+/1 2", "+\"0 (1 2)", "future 3"] {
        let p = e.analyze_a3(source).unwrap();
        let call = result_call(&p);
        assert_eq!(call.contract.effect, Effect::Unknown);
        assert!(!call.contract.allow_reassociation);
    }

    let p = e.analyze_a3("g=:future").unwrap();
    let result = p.result.unwrap();
    assert!(matches!(
        operation_for_value(&p, result).kind,
        OpKind::VerbReference(_)
    ));
    assert_eq!(p.write.unwrap().after, None);
}

#[test]
fn inferred_facts_match_successful_execution() {
    use rustj::facts::{DType, Facts, TypeFact};

    let mut e = Engine::new();
    for s in [
        "a=:i.2 3",
        "b=:i.2",
        "empty=:i.2 0",
        "max=:9223372036854775807",
    ] {
        e.eval(s).unwrap();
    }

    for s in [
        "a",
        "a+b",
        "2+a",
        "|.a",
        "|:a",
        ",a",
        "$a",
        "#a",
        "$|:a",
        "#\"0 a",
        "a=a",
        "empty+empty",
        ",empty",
        "$empty",
        "max+2",
        "2+3",
        "'abc'='axc'",
        "future 3",
    ] {
        let plan = e.analyze_a3(s).unwrap();
        let facts = &plan.values[plan.result.unwrap().0].facts;
        if s == "future 3" {
            assert_eq!(facts, &Facts::default());
            continue;
        }
        let value = e.eval(s).unwrap().unwrap();
        if let Some(shape) = &facts.shape {
            assert_eq!(shape, value.shape(), "{s}");
        }
        if let Some(rank) = facts.rank {
            assert_eq!(rank, value.shape().len(), "{s}");
        }
        let actual = Facts::of(&value);
        match facts.dtype {
            TypeFact::Exact(_) => assert_eq!(facts.dtype, actual.dtype, "{s}"),
            TypeFact::IntOrFloat => assert!(
                matches!(actual.dtype, TypeFact::Exact(DType::Int | DType::Float)),
                "{s}"
            ),
            TypeFact::Unknown => (),
        }
    }

    let p = e.analyze_a3("max+2").unwrap();
    assert_eq!(
        p.values[p.result.unwrap().0].facts.dtype,
        TypeFact::IntOrFloat
    );
}

#[test]
fn agreement_is_prefix_and_rank_is_known_without_extents() {
    let mut e = Engine::new();
    for s in ["matrix=:i.2 3", "prefix=:i.2", "suffix=:i.3"] {
        e.eval(s).unwrap();
    }

    let p = e.analyze_a3("prefix+matrix").unwrap();
    assert_eq!(
        p.values[p.result.unwrap().0].facts.shape,
        Some(vec![2, 3])
    );

    let p = e.analyze_a3("suffix+matrix").unwrap();
    assert_eq!(p.values[p.result.unwrap().0].facts.shape, None);
    assert!(matches!(e.eval("suffix+matrix"), Err(rustj::Error::Length)));

    let p = e.analyze_a3(",future 3").unwrap();
    let f = &p.values[p.result.unwrap().0].facts;
    assert_eq!(f.rank, Some(1));
    assert_eq!(f.shape, None);

    let p = e.analyze_a3("+/matrix").unwrap();
    assert_eq!(p.values[p.result.unwrap().0].facts.shape, Some(vec![3]));

    let p = e.analyze_a3("i.9223372036854775807").unwrap();
    assert_eq!(p.values[p.result.unwrap().0].facts.shape, None);
}

#[test]
fn rank_cell_frame_and_reduction_facts_match_execution() {
    use rustj::facts::{DType, Facts, TypeFact};

    let mut e = Engine::new();
    for s in [
        "a=:i.2 3",
        "b=:i.2 3 4",
        "emptycell=:i.2 0",
        "emptyrows=:i.0 3",
        "single=:1 3$1 2 3",
        "large=:2 2$9223372036854775807 1 2 3",
    ] {
        e.eval(s).unwrap();
    }

    for s in [
        "+/a",
        "*/emptyrows",
        "+/emptyrows",
        "+/single",
        "+/large",
        "+/7",
        "+/\"1 a",
        "+/\"_1 a",
        "+/\"0 a",
        "$\"1 a",
        "#\"0 a",
        ",\"1 a",
        "|:\"1 a",
        "a+\"1 0 b",
        "a+\"0 a",
        "a+\"99 a",
        "a+\"_99 a",
        "+/\"1 emptycell",
    ] {
        let p = e.analyze_a3(s).unwrap();
        let result = p.result.unwrap();
        let facts = &p.values[result.0].facts;
        let value = e.eval(s).unwrap().unwrap();
        assert_eq!(facts.shape.as_deref(), Some(value.shape()), "{s}");
        assert_eq!(facts.rank, Some(value.shape().len()), "{s}");
        match facts.dtype {
            TypeFact::Exact(_) => assert_eq!(facts.dtype, Facts::of(&value).dtype, "{s}"),
            TypeFact::IntOrFloat => assert!(matches!(
                Facts::of(&value).dtype,
                TypeFact::Exact(DType::Int | DType::Float)
            )),
            TypeFact::Unknown => (),
        }
    }

    let p = e.analyze_a3("a+\"1 0 b").unwrap();
    let layout = result_call(&p).rank_plan.as_ref().unwrap();
    assert_eq!(layout.left_frame, Some(vec![2]));
    assert_eq!(layout.left_cell, Some(vec![3]));
    assert_eq!(layout.right_frame, vec![2, 3, 4]);
    assert_eq!(layout.right_cell, Vec::<usize>::new());
    assert_eq!(layout.result_frame, Some(vec![2, 3, 4]));
}

#[test]
fn empty_frames_and_incompatible_frames_remain_unresolved() {
    use rustj::facts::Facts;

    let mut e = Engine::new();
    for s in ["empty=:i.0 3", "a=:i.2 3", "b=:i.4 3"] {
        e.eval(s).unwrap();
    }

    for s in ["+/\"1 empty", "empty+\"1 empty"] {
        let p = e.analyze_a3(s).unwrap();
        let result = p.result.unwrap();
        assert_eq!(p.values[result.0].facts, Facts::default());
        assert!(
            result_call(&p)
                .rank_plan
                .as_ref()
                .unwrap()
                .requires_empty_frame_prototype
        );
        assert!(matches!(e.eval(s), Err(rustj::Error::Unsupported(_))));
    }

    let p = e.analyze_a3("a+\"1 b").unwrap();
    let result = p.result.unwrap();
    assert_eq!(p.values[result.0].facts, Facts::default());
    let rank_plan = result_call(&p).rank_plan.as_ref().unwrap();
    assert_eq!(rank_plan.result_frame, None);
    assert!(matches!(e.eval("a+\"1 b"), Err(rustj::Error::Length)));
    assert!(!rank_plan.requires_empty_frame_prototype);
}

#[test]
fn verifier_rejects_malformed_dependencies_but_accepts_current_plans() {
    let mut e = Engine::new();
    e.eval("a=:1 2 3").unwrap();
    let mut p = e.analyze_a3("a+1").unwrap();
    p.verify().unwrap();

    let result = p.result.unwrap();
    let producer = p.values[result.0].producer;
    p.operations[producer.0].order_after = Some(producer);
    let err = p.verify().unwrap_err();
    assert_eq!(err.operation, Some(producer));
    assert!(err.message.contains("earlier operation"));
}

#[test]
fn access_knowledge_is_explicit_and_opaque_is_not_a_semantic_error() {
    let e = Engine::new();

    let p = e.analyze_a3("1+2").unwrap();
    assert_eq!(
        result_call(&p).access,
        AccessFact::Known(AccessRelation::ElementwiseMap)
    );
    p.verify().unwrap();

    let p = e.analyze_a3("+/1 2 3").unwrap();
    assert_eq!(
        result_call(&p).access,
        AccessFact::Known(AccessRelation::ReduceLeadingAxis)
    );
    p.verify().unwrap();

    let p = e.analyze_a3("+/\"1 (1 2 3)").unwrap();
    assert_eq!(result_call(&p).access, AccessFact::Opaque);
    p.verify().unwrap();

    let p = e.analyze_a3("|.1 2 3").unwrap();
    assert_eq!(result_call(&p).access, AccessFact::Opaque);
    p.verify().unwrap();
}

#[test]
fn canonical_mean_fork_lowers_to_reduce_tally_divide_in_jsource_order() {
    use rustj::primitive::PrimitiveId;

    let mut e = Engine::new();
    e.eval("y=:1 2 3 4").unwrap();
    let p = e.analyze_a3("(+/ % #) y").unwrap();
    p.verify().unwrap();

    let calls = call_operations(&p);
    assert_eq!(calls.len(), 3);
    assert!(matches!(
        calls[0].2.callable.target,
        CallTarget::Primitive(PrimitiveId::Tally)
    ));
    assert!(matches!(
        calls[1].2.callable.target,
        CallTarget::Primitive(PrimitiveId::Add)
    ));
    assert!(matches!(
        calls[1].2.callable.semantic.head,
        FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert)
    ));
    assert!(matches!(
        calls[2].2.callable.target,
        CallTarget::Primitive(PrimitiveId::Divide)
    ));
    assert!(ordered_after(&p, calls[1].0, calls[0].0));
    assert!(ordered_after(&p, calls[2].0, calls[1].0));
}

#[test]
fn longer_train_analysis_uses_nested_hook_and_fork_graphs() {
    use rustj::primitive::PrimitiveId;

    let mut e = Engine::new();
    e.eval("y=:2 3 4").unwrap();
    let p = e.analyze_a3("(+ - * %) y").unwrap();
    p.verify().unwrap();

    let calls = call_operations(&p);
    assert_eq!(calls.len(), 4);
    assert!(matches!(
        calls[0].2.callable.target,
        CallTarget::Primitive(PrimitiveId::Divide)
    ));
    assert!(matches!(
        calls[1].2.callable.target,
        CallTarget::Primitive(PrimitiveId::Subtract)
    ));
    assert!(matches!(
        calls[2].2.callable.target,
        CallTarget::Primitive(PrimitiveId::Multiply)
    ));
    assert!(matches!(
        calls[3].2.callable.target,
        CallTarget::Primitive(PrimitiveId::Add)
    ));
    assert!(ordered_after(&p, calls[1].0, calls[0].0));
    assert!(ordered_after(&p, calls[2].0, calls[1].0));
    assert!(ordered_after(&p, calls[3].0, calls[2].0));
}

#[test]
fn analysis_diagnostics_share_structured_context() {
    let engine = Engine::new();
    let error = engine.analyze_diagnostic("1 + )").unwrap_err();
    assert_eq!(error.kind(), "syntax error");
    let context = error.context().expect("diagnostic context");
    assert_eq!(
        context.phase,
        Some(rustj::error::DiagnosticPhase::Parse)
    );
    assert_eq!(context.span.clone(), Some(4..5));
    assert_eq!(context.blame_word_index, Some(2));
}

#[test]
fn provisional_basis_metadata_is_explicit() {
    use rustj::contracts::Valence;

    let mut e = Engine::new();
    e.eval("a=:i.2 3").unwrap();

    for (source, expected_basis, expected_valence) in [
        ("1+2", vec![ExecutionBasisKind::Elementwise], Valence::Dyad),
        ("+/1 2 3", vec![ExecutionBasisKind::Reduce], Valence::Monad),
        (
            "|.1 2 3",
            vec![ExecutionBasisKind::StaticReindex],
            Valence::Monad,
        ),
        ("i.2 3", vec![ExecutionBasisKind::IndexSpace], Valence::Monad),
        ("1 { 10 20 30", vec![ExecutionBasisKind::Gather], Valence::Dyad),
        (
            "10 20 i. 20",
            vec![ExecutionBasisKind::LookupClassify],
            Valence::Dyad,
        ),
        (
            "+/\"1 a",
            vec![ExecutionBasisKind::CellApply, ExecutionBasisKind::Reduce],
            Valence::Monad,
        ),
    ] {
        let plan = e.analyze_a3(source).unwrap();
        let result = plan.result.unwrap();
        let call = call_for_value(&plan, result);
        assert_eq!(call.execution_basis.layers, expected_basis, "{source}");
        assert_eq!(call.instantiation.valence, expected_valence, "{source}");
        assert_eq!(
            call.instantiation.result_dtype,
            plan.values[result.0].facts.dtype,
            "{source}"
        );
        assert_eq!(
            call.instantiation.result_rank,
            plan.values[result.0].facts.rank,
            "{source}"
        );
        plan.verify().unwrap();
    }
}

#[test]
fn modifier_nesting_order_is_preserved_in_basis_and_fact_inference() {
    let mut e = Engine::new();
    e.eval("a=:i.2 3").unwrap();

    let outer_rank = e.analyze_a3("+/\"1 a").unwrap();
    let outer_result = outer_rank.result.unwrap();
    let outer = call_for_value(&outer_rank, outer_result);
    assert_eq!(
        outer.execution_basis.layers,
        vec![ExecutionBasisKind::CellApply, ExecutionBasisKind::Reduce]
    );
    assert_eq!(
        outer_rank.values[outer_result.0].facts.shape.as_deref(),
        Some(&[2][..])
    );
    assert_eq!(outer.instantiation.rank_boundary, Some([1, 1, 1]));

    let inner_rank = e.analyze_a3("(+\"1)/ a").unwrap();
    let inner_result = inner_rank.result.unwrap();
    let inner = call_for_value(&inner_rank, inner_result);
    assert_eq!(
        inner.execution_basis.layers,
        vec![
            ExecutionBasisKind::Reduce,
            ExecutionBasisKind::CellApply,
            ExecutionBasisKind::Elementwise,
        ]
    );
    assert_eq!(
        inner_rank.values[inner_result.0].facts.shape.as_deref(),
        Some(&[3][..])
    );
    assert_eq!(inner.instantiation.rank_boundary, None);

    outer_rank.verify().unwrap();
    inner_rank.verify().unwrap();
}

#[test]
fn value_roles_are_contextual_facts_not_noun_types() {
    use rustj::facts::ValueRole;

    let e = Engine::new();

    let plan = e.analyze_a3("i.2 3").unwrap();
    let call = result_call(&plan);
    assert!(
        plan.values[call.right.0]
            .roles
            .contains(ValueRole::ShapeVector)
    );

    let plan = e.analyze_a3("$1 2 3").unwrap();
    assert!(
        plan.values[plan.result.unwrap().0]
            .roles
            .contains(ValueRole::ShapeVector)
    );

    let plan = e.analyze_a3("1 { 10 20 30").unwrap();
    let call = result_call(&plan);
    let left = call.left.unwrap();
    assert!(plan.values[left.0].roles.contains(ValueRole::IndexVector));

    let plan = e.analyze_a3("2 {. 10 20 30").unwrap();
    let call = result_call(&plan);
    let left = call.left.unwrap();
    assert!(plan.values[left.0].roles.contains(ValueRole::CountVector));
}

#[test]
fn verifier_checks_resolved_instantiation_consistency() {
    let e = Engine::new();
    let mut plan = e.analyze_a3("1+2").unwrap();
    let result = plan.result.unwrap();
    call_for_value_mut(&mut plan, result).instantiation.result_rank = Some(1);

    let error = plan.verify().unwrap_err();
    assert_eq!(error.operation, Some(plan.values[result.0].producer));
    assert!(error.message.contains("instantiation result facts"));
}

#[test]
fn verifier_checks_basis_metadata_consistency() {
    let e = Engine::new();
    let mut plan = e.analyze_a3("1+2").unwrap();
    let result = plan.result.unwrap();
    let producer = plan.values[result.0].producer;
    call_for_value_mut(&mut plan, result).execution_basis.layers =
        vec![ExecutionBasisKind::Reduce];

    let error = plan.verify().unwrap_err();
    assert_eq!(error.operation, Some(producer));
    assert!(error.message.contains("outer execution basis"));
}

#[test]
fn J_syntax_records_structural_optimization_opportunities() {
    use rustj::opportunity::{OpportunitySource, StructuralTopology};

    let e = Engine::new();

    let p = e.analyze_a3("(|. @: , @: |.) 1 2 3").unwrap();
    p.verify().unwrap();
    assert_eq!(p.opportunities.len(), 1);
    let pipeline = &p.opportunities[0];
    assert_eq!(pipeline.source, OpportunitySource::Atop);
    assert!(pipeline.j_region_origin.is_some());
    assert!(
        pipeline.j_region_origin.unwrap().0 < p.j_graph_region_count,
        "execution opportunity must point back to an upstream J Graph region"
    );
    let StructuralTopology::Pipeline {
        inputs,
        stage_results,
    } = &pipeline.topology
    else {
        panic!("atop should expose pipeline topology")
    };
    assert_eq!(inputs.len(), 1);
    assert_eq!(stage_results.len(), 3);

    let p = e.analyze_a3("(+ -) 3").unwrap();
    p.verify().unwrap();
    let hook = p
        .opportunities
        .iter()
        .find(|opportunity| opportunity.source == OpportunitySource::Hook)
        .expect("hook opportunity");
    let StructuralTopology::BranchJoin {
        shared_inputs,
        branch_results,
        live_across,
        ..
    } = &hook.topology
    else {
        panic!("hook should expose branch/join topology")
    };
    assert_eq!(branch_results.len(), 2);
    assert_eq!(live_across.len(), 1);
    assert!(shared_inputs.contains(&live_across[0]));

    let p = e.analyze_a3("(+/ % #) 1 2 3 4").unwrap();
    p.verify().unwrap();
    let fork = p
        .opportunities
        .iter()
        .find(|opportunity| opportunity.source == OpportunitySource::Fork)
        .expect("fork opportunity");
    let StructuralTopology::BranchJoin {
        shared_inputs,
        branch_results,
        live_across,
        ..
    } = &fork.topology
    else {
        panic!("fork should expose branch/join topology")
    };
    assert_eq!(shared_inputs.len(), 1);
    assert_eq!(branch_results.len(), 2);
    assert_eq!(live_across, shared_inputs);
}

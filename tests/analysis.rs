use rustj::{
    Engine,
    analysis::{AccessFact, AccessRelation, CallTarget, Operation, Scope, ValueId},
    contracts::{Effect, Overflow},
    semantic::{FunctionHead, NameVersion},
};

#[test]
fn analysis_does_not_execute_or_commit() {
    let mut e = Engine::new();
    e.eval("a=:7").unwrap();
    let stats = e.output_cache_stats();
    let p = e.analyze("a=:i.9223372036854775807").unwrap();
    assert_eq!(e.binding_version("a"), Some(NameVersion(1)));
    assert_eq!(e.output_cache_stats(), stats);
    let write = p.write.unwrap();
    assert_eq!(write.previous, Some(NameVersion(1)));
    assert_eq!(write.proposed, NameVersion(2));
    assert_eq!(Some(write.value), p.result);
    assert_eq!(write.after, p.result);
    assert!(e.analyze("'a'+1").is_ok()); // Runtime domain error is not raised by analysis.
    assert!(e.analyze("NB. empty").unwrap().nodes.is_empty());
}

#[test]
fn noun_versions_and_dynamic_calls_are_distinct() {
    let mut e = Engine::new();
    e.eval("a=:3").unwrap();
    e.eval("f=:+").unwrap();
    let p = e.analyze("out=:f a+a").unwrap();
    assert_eq!(p.symbols.iter().filter(|s| s.name == "a").count(), 1);
    assert!(p.symbols.iter().all(|s| s.scope == Scope::CurrentGlobal));
    let reads: Vec<_> = p
        .nodes
        .iter()
        .filter_map(|n| match n.operation {
            Operation::ReadNoun { symbol, version } => Some((symbol, version)),
            _ => None,
        })
        .collect();
    assert_eq!(reads.len(), 2);
    assert_eq!(reads[0], reads[1]);
    assert_eq!(reads[0].1, NameVersion(1));
    let Operation::Call {
        callable, contract, ..
    } = &p.nodes[p.result.unwrap().0].operation
    else {
        panic!()
    };
    let CallTarget::Dynamic(id) = callable.target else {
        panic!()
    };
    assert_eq!(p.symbols[id.0].name, "f");
    assert_eq!(contract.effect, Effect::Unknown);
    e.eval("a=:9").unwrap();
    e.eval("f=:*").unwrap();
    assert_eq!(reads[0].1, NameVersion(1)); // Old plan is a snapshot, never silently rebound.
    let next = e.analyze("a").unwrap();
    assert!(matches!(
        next.nodes[0].operation,
        Operation::ReadNoun {
            version: NameVersion(2),
            ..
        }
    ));
}

#[test]
fn order_edges_preserve_competing_errors_and_inputs() {
    let source = "('a'+1)+(1 2+1 2 3)";
    let p = Engine::new().analyze(source).unwrap();
    let calls: Vec<_> = p
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n.operation, Operation::Call { .. }))
        .collect();
    assert_eq!(calls.len(), 3);
    assert_eq!(&source[calls[0].1.span.clone()], "1 2+1 2 3");
    assert_eq!(&source[calls[1].1.span.clone()], "'a'+1");
    assert_eq!(calls[1].1.order_after, Some(ValueId(calls[0].0)));
    assert_eq!(calls[2].1.order_after, Some(ValueId(calls[1].0)));
    for (index, node) in p.nodes.iter().enumerate() {
        if let Some(before) = node.order_after {
            assert!(before.0 < index);
        }
        if let Operation::Call { left, right, .. } = node.operation {
            assert!(right.0 < index);
            if let Some(left) = left {
                assert!(left.0 < index);
            }
        }
    }
}

#[test]
fn primitive_and_derived_contracts_are_conservative() {
    let e = Engine::new();
    let p = e.analyze("1+2").unwrap();
    let Operation::Call { contract, .. } = &p.nodes[p.result.unwrap().0].operation else {
        panic!()
    };
    assert_eq!(contract.overflow, Overflow::WholeResultPromotion);
    let p = e.analyze("+/1 2").unwrap();
    let Operation::Call { callable, .. } = &p.nodes[p.result.unwrap().0].operation else {
        panic!()
    };
    assert_eq!(
        callable.semantic.head,
        FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert)
    );
    for source in ["+/1 2", "+\"0 (1 2)", "future 3"] {
        let p = e.analyze(source).unwrap();
        let Operation::Call { contract, .. } = &p.nodes[p.result.unwrap().0].operation else {
            panic!()
        };
        assert_eq!(contract.effect, Effect::Unknown);
        assert!(!contract.allow_reassociation);
    }
    let p = e.analyze("g=:future").unwrap();
    assert!(matches!(p.nodes[0].operation, Operation::VerbReference(_)));
    assert_eq!(p.write.unwrap().after, None); // Creating a name reference does not call it.
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
        let plan = e.analyze(s).unwrap();
        let facts = &plan.nodes[plan.result.unwrap().0].facts;
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
    let p = e.analyze("max+2").unwrap();
    assert_eq!(
        p.nodes[p.result.unwrap().0].facts.dtype,
        TypeFact::IntOrFloat
    );
}

#[test]
fn agreement_is_prefix_and_rank_is_known_without_extents() {
    let mut e = Engine::new();
    for s in ["matrix=:i.2 3", "prefix=:i.2", "suffix=:i.3"] {
        e.eval(s).unwrap();
    }
    let p = e.analyze("prefix+matrix").unwrap();
    assert_eq!(p.nodes[p.result.unwrap().0].facts.shape, Some(vec![2, 3]));
    let p = e.analyze("suffix+matrix").unwrap();
    assert_eq!(p.nodes[p.result.unwrap().0].facts.shape, None);
    assert!(matches!(e.eval("suffix+matrix"), Err(rustj::Error::Length)));
    let p = e.analyze(",future 3").unwrap();
    let f = &p.nodes[p.result.unwrap().0].facts;
    assert_eq!(f.rank, Some(1));
    assert_eq!(f.shape, None);
    let p = e.analyze("+/matrix").unwrap();
    assert_eq!(p.nodes[p.result.unwrap().0].facts.shape, Some(vec![3]));
    let p = e.analyze("i.9223372036854775807").unwrap();
    assert_eq!(p.nodes[p.result.unwrap().0].facts.shape, None);
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
        let p = e.analyze(s).unwrap();
        let node = &p.nodes[p.result.unwrap().0];
        let value = e.eval(s).unwrap().unwrap();
        assert_eq!(node.facts.shape.as_deref(), Some(value.shape()), "{s}");
        assert_eq!(node.facts.rank, Some(value.shape().len()), "{s}");
        match node.facts.dtype {
            TypeFact::Exact(_) => assert_eq!(node.facts.dtype, Facts::of(&value).dtype, "{s}"),
            TypeFact::IntOrFloat => assert!(matches!(
                Facts::of(&value).dtype,
                TypeFact::Exact(DType::Int | DType::Float)
            )),
            TypeFact::Unknown => (),
        }
    }
    let p = e.analyze("a+\"1 0 b").unwrap();
    let layout = p.nodes[p.result.unwrap().0].rank_plan.as_ref().unwrap();
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
        let p = e.analyze(s).unwrap();
        let node = &p.nodes[p.result.unwrap().0];
        assert_eq!(node.facts, Facts::default());
        assert!(
            node.rank_plan
                .as_ref()
                .unwrap()
                .requires_empty_frame_prototype
        );
        assert!(matches!(e.eval(s), Err(rustj::Error::Unsupported(_))));
    }
    let p = e.analyze("a+\"1 b").unwrap();
    let node = &p.nodes[p.result.unwrap().0];
    assert_eq!(node.facts, Facts::default());
    assert_eq!(node.rank_plan.as_ref().unwrap().result_frame, None);
    assert!(matches!(e.eval("a+\"1 b"), Err(rustj::Error::Length)));
    assert!(
        !node
            .rank_plan
            .as_ref()
            .unwrap()
            .requires_empty_frame_prototype
    );
}


#[test]
fn verifier_rejects_malformed_dependencies_but_accepts_current_plans() {
    let mut e = Engine::new();
    e.eval("a=:1 2 3").unwrap();
    let mut p = e.analyze("a+1").unwrap();
    p.verify().unwrap();

    let result = p.result.unwrap();
    p.nodes[result.0].order_after = Some(result);
    let err = p.verify().unwrap_err();
    assert_eq!(err.node, Some(result));
    assert!(err.message.contains("earlier value"));
}

#[test]
fn access_knowledge_is_explicit_and_opaque_is_not_a_semantic_error() {
    let e = Engine::new();

    let p = e.analyze("1+2").unwrap();
    assert_eq!(
        p.nodes[p.result.unwrap().0].access,
        AccessFact::Known(AccessRelation::ElementwiseMap)
    );
    p.verify().unwrap();

    let p = e.analyze("+/1 2 3").unwrap();
    assert_eq!(
        p.nodes[p.result.unwrap().0].access,
        AccessFact::Known(AccessRelation::ReduceLeadingAxis)
    );
    p.verify().unwrap();
    let p = e.analyze("+/\"1 (1 2 3)").unwrap();
    assert_eq!(p.nodes[p.result.unwrap().0].access, AccessFact::Opaque);
    p.verify().unwrap();

    let p = e.analyze("|.1 2 3").unwrap();
    assert_eq!(p.nodes[p.result.unwrap().0].access, AccessFact::Opaque);
    p.verify().unwrap();
}

#[test]
fn canonical_mean_fork_lowers_to_reduce_tally_divide_in_jsource_order() {
    use rustj::primitive::PrimitiveId;
    let mut e = Engine::new();
    e.eval("y=:1 2 3 4").unwrap();
    let p = e.analyze("(+/ % #) y").unwrap();
    p.verify().unwrap();

    let calls: Vec<_> = p
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match &node.operation {
            Operation::Call { callable, .. } => Some((ValueId(index), callable)),
            _ => None,
        })
        .collect();
    assert_eq!(calls.len(), 3);
    assert!(matches!(calls[0].1.target, CallTarget::Primitive(PrimitiveId::Tally)));
    assert!(matches!(calls[1].1.target, CallTarget::Primitive(PrimitiveId::Add)));
    assert!(calls[1].1.reduce);
    assert!(matches!(calls[2].1.target, CallTarget::Primitive(PrimitiveId::Divide)));
    assert_eq!(p.nodes[calls[1].0.0].order_after, Some(calls[0].0));
    assert_eq!(p.nodes[calls[2].0.0].order_after, Some(calls[1].0));
}

#[test]
fn longer_train_analysis_uses_nested_hook_and_fork_graphs() {
    use rustj::primitive::PrimitiveId;
    let mut e = Engine::new();
    e.eval("y=:2 3 4").unwrap();
    let p = e.analyze("(+ - * %) y").unwrap();
    p.verify().unwrap();

    let calls: Vec<_> = p
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match &node.operation {
            Operation::Call { callable, .. } => Some((ValueId(index), callable)),
            _ => None,
        })
        .collect();
    assert_eq!(calls.len(), 4);
    assert!(matches!(calls[0].1.target, CallTarget::Primitive(PrimitiveId::Divide)));
    assert!(matches!(calls[1].1.target, CallTarget::Primitive(PrimitiveId::Subtract)));
    assert!(matches!(calls[2].1.target, CallTarget::Primitive(PrimitiveId::Multiply)));
    assert!(matches!(calls[3].1.target, CallTarget::Primitive(PrimitiveId::Add)));
    assert_eq!(p.nodes[calls[1].0.0].order_after, Some(calls[0].0));
    assert_eq!(p.nodes[calls[2].0.0].order_after, Some(calls[1].0));
    assert_eq!(p.nodes[calls[3].0.0].order_after, Some(calls[2].0));
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

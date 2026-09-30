use rustj::{
    Engine,
    contracts::{self, Effect, Overflow, Valence},
    semantic::{
        self, ExprKind as Expr, FunctionHead, FunctionOperand,
    },
};
#[test]
fn parse_is_execution_free_and_right_associative() {
    let p = semantic::parse("target=:missing + 2 * 3").unwrap();
    assert_eq!(p.assignment.as_deref(), Some("target"));
    let Expr::Dyad { verb, left, right } = p.expression.unwrap().kind else {
        panic!()
    };
    assert_eq!(
        verb.target,
        semantic::VerbTarget::Primitive(rustj::primitive::PrimitiveId::Add)
    );
    assert!(matches!(left.kind,Expr::ReadName(ref name) if name=="missing"));
    assert!(
        matches!(right.kind,Expr::Dyad{ref verb,..} if verb.target==semantic::VerbTarget::Primitive(rustj::primitive::PrimitiveId::Multiply))
    );
    // Shape-producing operation is represented, not run or allocated here.
    assert!(semantic::parse("i.9223372036854775807").is_ok());
    let p = semantic::parse("+/\"1 i.2 3").unwrap();
    let Some(Expr::Monad { verb, .. }) = p.expression.map(|e| e.kind) else {
        panic!()
    };
    assert!(verb.reduce);
    assert_eq!(verb.rank, Some([1, 1, 1]));

    let rank = verb.entity;
    assert_eq!(
        rank.head,
        FunctionHead::PrimitiveConjunction(rustj::primitive::ConjunctionId::Rank)
    );
    let [
        FunctionOperand::Function(insert),
        FunctionOperand::Noun { value: rank_value, .. },
    ] = rank.operands.as_slice()
    else {
        panic!("rank should retain base function and rank noun");
    };
    assert_eq!(rank_value.int_at(0).unwrap(), 1);
    assert_eq!(
        insert.head,
        FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert)
    );
    let [FunctionOperand::Function(base)] = insert.operands.as_slice() else {
        panic!("insert should retain its operand");
    };
    assert_eq!(
        base.head,
        FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Add)
    );
}
#[test]
fn semantic_reference_preserves_values_and_transactions() {
    let mut direct = Engine::new();
    let mut ir = Engine::new();
    for s in [
        "a=:i.2 3",
        "b=:a",
        "a=:|.a",
        "b",
        "a",
        "+/\"1 a",
        "10-3-2",
        "a=:1 2+1 2 3",
        "a",
        "(10 20)+\"0 1 b",
        "'ana' E. 'banana'",
        "9223372036854775807+1",
    ] {
        let normalize = |r: rustj::Result<Option<rustj::Value>>| {
            r.map(|v| v.map(|v| v.json())).map_err(|e| e.kind())
        };
        assert_eq!(
            normalize(direct.eval(s)),
            normalize(ir.eval_semantic_reference(s)),
            "{s}"
        );
    }
}
#[test]
fn unknown_contracts_are_barriers() {
    let unknown = contracts::lookup("conv", Valence::Dyad);
    assert_eq!(unknown.effect, Effect::Unknown);
    assert!(!unknown.allow_reassociation);
    let add = contracts::lookup("+", Valence::Dyad);
    assert_eq!(add.overflow, Overflow::WholeResultPromotion);
    assert!(add.preserve_evaluation_order && add.alias_requires_proof);
    assert_eq!(
        contracts::lookup("with", Valence::Dyad).effect,
        Effect::Unknown
    );
}

#[test]
fn node_spans_preserve_groups_modifiers_and_utf8_bytes() {
    let source = "  out=: +/\"1 (a) NB. ignored";
    let p = semantic::parse(source).unwrap();
    assert_eq!(&source[p.assignment_span.unwrap()], "out");
    let expression = p.expression.unwrap();
    assert_eq!(&source[expression.span], "+/\"1 (a)");
    let Expr::Monad { verb, argument } = expression.kind else {
        panic!()
    };
    assert_eq!(&source[verb.span], "+/\"1");
    assert_eq!(&source[argument.span], "(a)");
    let Expr::Group(inner) = argument.kind else {
        panic!()
    };
    assert_eq!(&source[inner.span], "a");
    let source = "  '한글'  NB. end";
    let p = semantic::parse(source).unwrap();
    assert_eq!(&source[p.expression.unwrap().span], "'한글'");
}

#[test]
fn binding_is_an_execution_free_version_snapshot() {
    use semantic::NameVersion;
    let mut engine = Engine::new();
    engine.eval("a=:1 2 3").unwrap();
    for _ in 0..2 {
        let source = "a=: (a) + a";
        let bound = engine.prepare_semantic(source).unwrap();
        assert_eq!(bound.reads.len(), 2);
        for read in bound.reads {
            assert_eq!(read.version, NameVersion(1));
            assert_eq!(&source[read.span], "a");
        }
        let write = bound.write.unwrap();
        assert_eq!(write.previous, Some(NameVersion(1)));
        assert_eq!(write.proposed, NameVersion(2));
        assert_eq!(engine.binding_version("a"), Some(NameVersion(1)));
    }
    assert!(engine.prepare_semantic("i.9223372036854775807").is_ok());
    assert_eq!(
        engine
            .prepare_semantic("missing + 1")
            .unwrap()
            .verb_references[0]
            .0,
        "missing"
    );
    assert_eq!(engine.binding_version("missing"), None);
}

#[test]
fn both_evaluators_commit_versions_only_after_success() {
    use semantic::NameVersion;
    for ir in [false, true] {
        let mut engine = Engine::new();
        engine.eval("a=:1 2 3").unwrap();
        engine.eval_semantic_reference("b=:a").unwrap();
        let before = engine.eval("a").unwrap().unwrap().json();
        for source in ["a=:1 2+1 2 3", "a=:missing+1", "a=:(", "new=:missing 1"] {
            let result = if ir {
                engine.eval_semantic_reference(source)
            } else {
                engine.eval(source)
            };
            assert!(result.is_err(), "{source}");
            assert_eq!(engine.binding_version("a"), Some(NameVersion(1)));
            assert_eq!(engine.binding_version("new"), None);
            assert_eq!(engine.eval("a").unwrap().unwrap().json(), before);
        }
        if ir {
            engine.eval_semantic_reference("a=:a+1")
        } else {
            engine.eval("a=:a+1")
        }
        .unwrap();
        assert_eq!(engine.binding_version("a"), Some(NameVersion(2)));
        assert_eq!(engine.binding_version("b"), Some(NameVersion(1)));
        assert_eq!(engine.eval("b").unwrap().unwrap().json(), before);
    }
}

#[test]
fn shared_frontend_preserves_right_hand_error_precedence() {
    for ir in [false, true] {
        let mut engine = Engine::new();
        engine.eval("a=:1 2 3").unwrap();
        let before = engine.eval("a").unwrap().unwrap().json();
        for (source, kind) in [
            ("('a'+1)+(1 2+1 2 3)", "length error"),
            ("(1 2+1 2 3)+('a'+1)", "domain error"),
            ("missing + (1 2+1 2 3)", "length error"),
            ("missing + )", "syntax error"),
        ] {
            let assignment = format!("a=:{source}");
            let error = if ir {
                engine.eval_semantic_reference(&assignment)
            } else {
                engine.eval(&assignment)
            }
            .unwrap_err();
            assert_eq!(error.kind(), kind, "{source}");
            assert_eq!(engine.binding_version("a"), Some(semantic::NameVersion(1)));
            assert_eq!(engine.eval("a").unwrap().unwrap().json(), before);
        }
    }
}

#[test]
fn parsed_tree_depth_is_bounded_before_execution_and_drop() {
    let limit = semantic::MAX_EXPR_DEPTH;
    for source in [
        format!("{}1", "- ".repeat(limit)),
        format!("{}1", "1 + ".repeat(limit)),
        format!("{}1{}", "(".repeat(limit), ")".repeat(limit)),
    ] {
        assert!(semantic::parse(&source).is_ok());
        assert!(Engine::new().eval(&source).is_ok());
    }
    for source in [
        format!("{}1", "- ".repeat(20_000)),
        format!("{}1", "1 + ".repeat(20_000)),
        format!("({}1)", "- ".repeat(limit)),
        format!("{}1{}", "(".repeat(limit + 1), ")".repeat(limit + 1)),
    ] {
        assert!(matches!(semantic::parse(&source), Err(rustj::Error::Limit)));
        let mut engine = Engine::new();
        engine.eval("a=:7").unwrap();
        assert!(matches!(
            engine.eval(&format!("a=:{source}")),
            Err(rustj::Error::Limit)
        ));
        assert_eq!(engine.binding_version("a"), Some(semantic::NameVersion(1)));
    }
}

#[test]
fn nouns_snapshot_and_named_verbs_resolve_on_call() {
    for reference in [false, true] {
        let mut engine = Engine::new();
        let eval = |e: &mut Engine, s: &str| {
            if reference {
                e.eval_semantic_reference(s)
            } else {
                e.eval(s)
            }
        };
        for s in [
            "a=:1",
            "b=:a",
            "a=:2",
            "f=:+",
            "g=:f",
            "f=:*",
            "late=:future",
            "future=:-",
        ] {
            assert!(eval(&mut engine, s).unwrap().is_none(), "{s}");
        }
        for (source, expected) in [("b", "1"), ("g 3", "* 3"), ("late 3", "_3"), ("2 g 3", "6")] {
            let value = eval(&mut engine, source).unwrap().unwrap().json();
            let expected = Engine::new().eval(expected).unwrap().unwrap().json();
            assert_eq!(value, expected, "{source}");
        }
        eval(&mut engine, "f=:7").unwrap();
        assert!(matches!(
            eval(&mut engine, "g 3"),
            Err(rustj::Error::Domain)
        ));
        eval(&mut engine, "unresolved=:notyet").unwrap();
        assert!(matches!(
            eval(&mut engine, "unresolved 3"),
            Err(rustj::Error::Value(_))
        ));
        eval(&mut engine, "cycle=:cycle").unwrap();
        assert!(matches!(
            eval(&mut engine, "cycle 3"),
            Err(rustj::Error::Limit)
        ));
        eval(&mut engine, "sum=:+/").unwrap();
        assert_eq!(
            eval(&mut engine, "sum 1 2 3").unwrap().unwrap().json(),
            Engine::new().eval("6").unwrap().unwrap().json()
        );
        eval(&mut engine, "f=:+").unwrap();
        assert!(eval(&mut engine, "f=:notyet 3").is_err());
        assert_eq!(
            eval(&mut engine, "f 3").unwrap().unwrap().json(),
            Engine::new().eval("3").unwrap().unwrap().json()
        );
    }
}

#[test]
fn analysis_separates_noun_versions_from_dynamic_verbs() {
    let mut e = Engine::new();
    e.eval("a=:3").unwrap();
    e.eval("f=:+").unwrap();
    let bound = e.prepare_semantic("f a").unwrap();
    assert_eq!(bound.reads.len(), 1);
    assert_eq!(bound.reads[0].name, "a");
    assert_eq!(bound.verb_references, vec![("f".to_owned(), 0..1)]);
    assert_eq!(bound.reads[0].version, semantic::NameVersion(1));
    assert_eq!(
        e.prepare_semantic("g=:future").unwrap().verb_references[0].0,
        "future"
    );
}

#[test]
fn large_derived_function_handles_share_the_semantic_graph() {
    let p = semantic::parse("+/\"1").unwrap();
    let Some(Expr::VerbValue(verb)) = p.expression.map(|e| e.kind) else {
        panic!()
    };
    let clone = verb.clone();
    assert!(std::sync::Arc::ptr_eq(&verb.entity, &clone.entity));

    let rank = verb.entity;
    let [FunctionOperand::Function(insert), ..] = rank.operands.as_slice() else {
        panic!()
    };
    let cloned_insert = insert.clone();
    assert!(std::sync::Arc::ptr_eq(insert, &cloned_insert));
}

#[test]
fn verb_trains_build_shared_hook_fork_graphs_right_to_left() {
    let p = semantic::parse("(+/ % #)").unwrap();
    let Some(Expr::VerbValue(verb)) = p.expression.map(|e| e.kind) else {
        panic!()
    };
    assert_eq!(verb.entity.head, FunctionHead::Fork);
    let [
        FunctionOperand::Function(f),
        FunctionOperand::Function(g),
        FunctionOperand::Function(h),
    ] = verb.entity.operands.as_slice()
    else {
        panic!()
    };
    assert_eq!(
        f.head,
        FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert)
    );
    assert_eq!(g.head, FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Divide));
    assert_eq!(h.head, FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Tally));

    let p = semantic::parse("(+ - * %)").unwrap();
    let Some(Expr::VerbValue(verb)) = p.expression.map(|e| e.kind) else { panic!() };
    assert_eq!(verb.entity.head, FunctionHead::Hook);
    let [FunctionOperand::Function(first), FunctionOperand::Function(tail)] =
        verb.entity.operands.as_slice()
    else { panic!() };
    assert_eq!(first.head, FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Add));
    assert_eq!(tail.head, FunctionHead::Fork);
}

#[test]
fn mixed_noun_sentences_do_not_prematurely_collapse_verbs_into_trains() {
    let p = semantic::parse("1 + - 2").unwrap();
    let Some(Expr::Dyad { verb, right, .. }) = p.expression.map(|e| e.kind) else {
        panic!()
    };
    assert_eq!(
        verb.target,
        semantic::VerbTarget::Primitive(rustj::primitive::PrimitiveId::Add)
    );
    assert!(matches!(right.kind, Expr::Monad { .. }));
}

#[test]
fn large_pure_verb_train_builds_iteratively_as_a_shared_graph() {
    let source = std::iter::repeat("+").take(101).collect::<Vec<_>>().join(" ");
    let p = semantic::parse(&source).unwrap();
    let Some(Expr::VerbValue(verb)) = p.expression.map(|e| e.kind) else {
        panic!()
    };
    assert_eq!(verb.entity.head, FunctionHead::Fork);

    let root = verb.entity.clone();
    let alias = root.clone();
    assert!(std::sync::Arc::ptr_eq(&root, &alias));

    let mut stack = vec![root];
    let mut functions = 0usize;
    while let Some(function) = stack.pop() {
        functions += 1;
        for operand in &function.operands {
            if let FunctionOperand::Function(child) = operand {
                stack.push(child.clone());
            }
        }
    }
    // 101 primitive leaves + 50 fork nodes.
    assert_eq!(functions, 151);
}

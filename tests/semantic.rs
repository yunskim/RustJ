use rustj::{
    Engine,
    contracts::{self, Effect, Overflow, Valence},
    semantic::{self, ExprKind as Expr, FunctionHead, FunctionOperand},
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
    let rank = verb.entity;
    assert_eq!(
        rank.head,
        FunctionHead::PrimitiveConjunction(rustj::primitive::ConjunctionId::Rank)
    );
    let [
        FunctionOperand::Function(insert),
        FunctionOperand::Noun {
            value: rank_value, ..
        },
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
fn prefix_infix_adverb_is_preserved_as_a_derived_function_entity() {
    let p = semantic::parse("(+/)\\").unwrap();
    let Some(Expr::VerbValue(verb)) = p.expression.map(|e| e.kind) else {
        panic!()
    };
    assert_eq!(
        verb.entity.head,
        FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::PrefixInfix)
    );
    let [FunctionOperand::Function(reducer)] = verb.entity.operands.as_slice() else {
        panic!("prefix/infix should retain its operand function")
    };
    assert_eq!(
        reducer.head,
        FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert)
    );
    let [FunctionOperand::Function(base)] = reducer.operands.as_slice() else {
        panic!("insert should retain +")
    };
    assert_eq!(
        base.head,
        FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Add)
    );
}

#[test]
fn key_derived_verb_keeps_operator_and_operand_without_licensing_execution() {
    use rustj::primitive::{AdverbId, PrimitiveId};

    let parsed = semantic::parse("+/.").unwrap();
    let Some(Expr::VerbValue(key)) = parsed.expression.map(|expr| expr.kind) else {
        panic!("u/. should construct a derived verb");
    };
    assert_eq!(
        key.entity.head,
        FunctionHead::PrimitiveAdverb(AdverbId::Key)
    );
    // ao.c::jtsldot installs RMAX for monad and both dyadic ranks.
    assert_eq!(key.entity.innate_ranks(), Some([63; 3]));
    let [rustj::semantic::FunctionOperand::Function(operand)] = key.entity.operands.as_slice()
    else {
        panic!("Key must preserve its construction-time verb operand");
    };
    assert_eq!(operand.head, FunctionHead::PrimitiveVerb(PrimitiveId::Add));

    for (source, expected_dyad) in [("+/. 1 2 3", false), ("1 0 1 +/. 4 5 6", true)] {
        let program = semantic::parse(source).unwrap();
        match (expected_dyad, program.expression.unwrap().kind) {
            (false, Expr::Monad { verb, .. }) | (true, Expr::Dyad { verb, .. }) => {
                assert_eq!(
                    verb.entity.head,
                    FunctionHead::PrimitiveAdverb(AdverbId::Key)
                );
            }
            _ => panic!("Key parsed with wrong application valence: {source}"),
        }
        assert_eq!(
            Engine::new().eval(source).unwrap_err().kind(),
            "unsupported"
        );
    }

    // A noun-left gerund is accepted by upstream ao.c::jtsldot only after a
    // gerund-specific audit. RustJ must not pretend arbitrary noun support.
    assert_eq!(Engine::new().eval("3/.").unwrap_err().kind(), "unsupported");
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
            if source == "missing + )" {
                // C constructs/assigns the hook before exit-parse reports the
                // unmatched ')'. A syntax failure is not a universal rollback.
                assert_eq!(engine.binding_version("a"), Some(semantic::NameVersion(2)));
                assert!(matches!(
                    engine
                        .prepare_semantic("a")
                        .unwrap()
                        .program
                        .expression
                        .unwrap()
                        .kind,
                    Expr::VerbValue(_)
                ));
            } else {
                assert_eq!(engine.binding_version("a"), Some(semantic::NameVersion(1)));
                assert_eq!(engine.eval("a").unwrap().unwrap().json(), before);
            }
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
    assert_eq!(
        g.head,
        FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Divide)
    );
    assert_eq!(
        h.head,
        FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Tally)
    );

    let p = semantic::parse("(+ - * %)").unwrap();
    let Some(Expr::VerbValue(verb)) = p.expression.map(|e| e.kind) else {
        panic!()
    };
    assert_eq!(verb.entity.head, FunctionHead::Hook);
    let [
        FunctionOperand::Function(first),
        FunctionOperand::Function(tail),
    ] = verb.entity.operands.as_slice()
    else {
        panic!()
    };
    assert_eq!(
        first.head,
        FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Add)
    );
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
    let source = std::iter::repeat_n("+", 101).collect::<Vec<_>>().join(" ");
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

#[test]
fn diagnostic_parser_retains_span_without_changing_machine_error_api() {
    let source = "1 + )";
    assert!(matches!(
        semantic::parse(source),
        Err(rustj::Error::Syntax(_))
    ));
    let error = semantic::parse_diagnostic(source).unwrap_err();
    assert_eq!(error.kind(), "syntax error");
    assert_eq!(error.span().cloned(), Some(4..5));
    let rendered = error.render("<test>", source, 1);
    assert!(rendered.contains("line 1, column 5"));
    assert!(rendered.contains("SyntaxError: unexpected )"));
}

#[test]
fn chained_conjunctions_preserve_jsource_left_to_right_association() {
    let p = semantic::parse("(|. @: , @: |.)").unwrap();
    let Some(Expr::VerbValue(verb)) = p.expression.map(|e| e.kind) else {
        panic!()
    };

    assert_eq!(
        verb.entity.head,
        FunctionHead::PrimitiveConjunction(rustj::primitive::ConjunctionId::Atop)
    );
    let [
        FunctionOperand::Function(left),
        FunctionOperand::Function(right),
    ] = verb.entity.operands.as_slice()
    else {
        panic!("outer atop should retain both function operands")
    };
    assert_eq!(
        left.head,
        FunctionHead::PrimitiveConjunction(rustj::primitive::ConjunctionId::Atop),
        "J conjunctions associate left-to-right"
    );
    let [
        FunctionOperand::Function(first),
        FunctionOperand::Function(second),
    ] = left.operands.as_slice()
    else {
        panic!("left-associated inner atop should retain both operands")
    };
    assert_eq!(
        first.head,
        FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Reverse)
    );
    assert_eq!(
        second.head,
        FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Ravel)
    );
    assert_eq!(
        right.head,
        FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Reverse)
    );
}

#[test]
fn extension_names_enter_as_names_then_join_modifier_rows_by_parser_time_pos() {
    use rustj::primitive::{
        ExtensionPrimitive, LoweringKey, PrimitiveContext, PrimitiveHandle, PrimitivePartOfSpeech,
        PrimitiveResolver, PrimitiveSemanticId, PrimitiveSemanticInfo, PrimitiveSourceOrigin,
        REGISTRY_VERSION,
    };

    let handle = |id: &'static str, pos| PrimitiveHandle {
        semantic_id: PrimitiveSemanticId::Extension(id),
        source_origin: PrimitiveSourceOrigin::Extension,
        result_pos: pos,
        semantic_info: PrimitiveSemanticInfo {
            registry_version: REGISTRY_VERSION,
        },
        lowering_key: LoweringKey::Extension(id),
    };
    let context = PrimitiveContext::new(PrimitiveResolver::with_extensions([
        ExtensionPrimitive {
            spelling: "advx",
            handle: handle("test.advx", PrimitivePartOfSpeech::Adverb),
        },
        ExtensionPrimitive {
            spelling: "conjx",
            handle: handle("test.conjx", PrimitivePartOfSpeech::Conjunction),
        },
    ]));
    let engine = Engine::with_primitive_context(context);

    for source in ["f=: + advx", "g=: + conjx *"] {
        let error = engine.prepare_semantic_diagnostic(source).unwrap_err();
        assert_eq!(error.kind(), "unsupported");
        assert_eq!(error.context().unwrap().blame_word_index, Some(3));
        assert!(error.to_string().contains("POS alone is insufficient"));
    }
    assert_eq!(engine.binding_version("f"), None);
    assert_eq!(engine.binding_version("g"), None);
    // An extension's input POS is still valid information for a value/alias;
    // it cannot by itself determine the POS/error of its application.
    let adverb = engine.prepare_semantic("advx").unwrap();
    assert!(matches!(
        adverb.program.expression.unwrap().kind,
        Expr::ModifierValue(_)
    ));
}

#[test]
fn final_assignment_uses_row_seven_but_mid_sentence_assignment_is_not_rebound_to_the_root() {
    let program = semantic::parse("target=: +/ % #").unwrap();
    assert_eq!(program.assignment.as_deref(), Some("target"));
    assert_eq!(
        program
            .assignment_span
            .as_ref()
            .map(|span| &program.source[span.clone()]),
        Some("target")
    );
    assert!(matches!(
        program.expression.map(|expr| expr.kind),
        Some(Expr::VerbValue(_))
    ));

    let error = semantic::parse("1 + target=:2").unwrap_err();
    assert!(matches!(
        error.into_unlocated(),
        rustj::Error::Unsupported(_)
    ));
}

#[test]
fn nested_parentheses_preserve_completed_noun_operands() {
    for source in ["+\"((1))", "(((7))) + *"] {
        assert!(semantic::parse(source).is_ok(), "{source}");
    }
}

#[test]
fn rank_constructor_checks_noun_rank_before_length_or_domain() {
    for (noun, expected) in [
        ("1 1 $ 0", "rank error"),
        ("2 2 $ 0", "rank error"),
        ("0 4 $ 0", "rank error"),
        ("1 1 $ 'a'", "rank error"),
        ("0 $ 0", "length error"),
        ("4 $ 0", "length error"),
        ("'a'", "domain error"),
        ("1.5", "domain error"),
    ] {
        let mut engine = Engine::default();
        engine.eval(&format!("r=:{noun}")).unwrap();
        let error = engine.eval("f=:+\"r").unwrap_err();
        assert_eq!(error.kind(), expected, "{noun}");
    }
}

#[test]
fn rank_noun_audit_accepts_j_infinite_and_tolerantly_integral_values() {
    for rank in [
        "_",
        "__",
        "_.",
        "1e100",
        "_1e100",
        "1.00000000000001",
        "_1.00000000000001",
        "0 1 _",
    ] {
        let mut engine = Engine::default();
        engine.eval(&format!("f=:+\"{rank}")).unwrap();
        assert_eq!(
            engine.eval("f 3").unwrap().unwrap().int_at(0).unwrap(),
            3,
            "{rank}"
        );
        let analysis = engine.analyze(&format!("(+\"{rank}) 3"));
        assert!(analysis.is_ok(), "{rank}: {analysis:?}");
    }
}

#[test]
fn invalid_modifier_operands_report_constructor_domain_errors() {
    for source in ["3/", "+@:3", "3@:+"] {
        assert_eq!(
            semantic::parse(source).unwrap_err().kind(),
            "domain error",
            "{source}"
        );
    }
}

#[test]
fn modifier_bidents_and_tridents_preserve_actual_pos_and_ordered_operands() {
    use rustj::semantic::{ExprKind, FunctionHead, FunctionPartOfSpeech as Pos};
    for (source, expected, count) in [
        ("+\"", Pos::Adverb, 2),
        ("\"1", Pos::Adverb, 2),
        ("3\"", Pos::Adverb, 2),
        ("/+", Pos::Adverb, 2),
        ("/\\", Pos::Adverb, 2),
        ("/@:", Pos::Adverb, 2),
        ("@:/", Pos::Conjunction, 2),
        ("@:@:", Pos::Conjunction, 2),
        ("/ / /", Pos::Adverb, 3),
        ("/ / +", Pos::Conjunction, 3),
        ("/ + *", Pos::Adverb, 3),
        ("@: + *", Pos::Conjunction, 3),
        ("+ @: /", Pos::Adverb, 3),
        ("+ @: @:", Pos::Conjunction, 3),
    ] {
        let program = semantic::parse(source).unwrap_or_else(|e| panic!("{source}: {e:?}"));
        let ExprKind::ModifierValue(entity) = program.expression.unwrap().kind else {
            panic!("{source}");
        };
        assert_eq!(entity.head, FunctionHead::ModifierTrain, "{source}");
        assert_eq!(entity.result_pos, expected, "{source}");
        assert_eq!(entity.operands.len(), count, "{source}");
        assert_eq!(entity.span, 0..source.len());
        let row = program
            .reductions
            .iter()
            .find(|r| r.row == rustj::parser::ParseRow::Hook)
            .unwrap();
        assert_eq!(row.inputs.len(), count);
        // p.c row 6 takes f's token for a bident and g's for a trident.
        assert_eq!(row.result.blame_word_index, if count == 2 { 0 } else { 1 });
    }
}

#[test]
fn nested_modifier_train_keeps_completed_child_and_noun_payload() {
    use rustj::semantic::{ExprKind, FunctionHead, FunctionOperand};
    let program = semantic::parse("(/ /) /").unwrap();
    let ExprKind::ModifierValue(root) = program.expression.unwrap().kind else {
        panic!();
    };
    assert_eq!(root.operands.len(), 2);
    let FunctionOperand::Function(child) = &root.operands[0] else {
        panic!();
    };
    assert_eq!(child.head, FunctionHead::ModifierTrain);
    assert_eq!(child.operands.len(), 2);
    let program = semantic::parse("\"1 2").unwrap();
    let ExprKind::ModifierValue(root) = program.expression.unwrap().kind else {
        panic!();
    };
    let FunctionOperand::Noun { value, span } = &root.operands[1] else {
        panic!();
    };
    assert_eq!(value.shape(), &[2]);
    assert_eq!(*span, 1..4);
    assert_eq!(value.int_at(1).unwrap(), 2);
}

#[test]
fn recognized_gerund_construction_does_not_claim_unimplemented_execution() {
    let mut engine = rustj::Engine::new();
    engine.eval("protected=:+").unwrap();
    let version = engine.binding_version("protected");
    let source = "protected=: (, <'!') (\\ @: +)";
    let report = engine.eval_captured(source);
    assert!(report.result.unwrap().is_none(), "{source}");
    report.capture.verify().unwrap();
    assert_ne!(engine.binding_version("protected"), version);
    assert_eq!(
        engine.eval("protected 3").unwrap_err().kind(),
        "unsupported"
    );
    assert_eq!(
        engine.prepare_semantic(source).unwrap_err().kind(),
        "unsupported"
    );
}

#[test]
fn right_bound_conjunction_bident_matches_direct_construction_and_calls() {
    let mut engine = rustj::Engine::new();
    for (source, direct) in [
        ("(+ (\"1)) i.2 3", "(+\"1) i.2 3"),
        ("(- (\"1 2)) i.2 3", "(-\"1 2) i.2 3"),
    ] {
        let expected = engine.eval(direct).unwrap().unwrap().json();
        for semantic in [false, true] {
            let actual = if semantic {
                engine.eval_semantic_reference(source)
            } else {
                engine.eval(source)
            };
            assert_eq!(actual.unwrap().unwrap().json(), expected, "{source}");
        }
    }
}

#[test]
fn named_bound_modifier_alias_is_frozen_and_failures_leave_target_unchanged() {
    let mut engine = rustj::Engine::new();
    for source in [
        "bound=: \"1",
        "alias=:bound",
        "bound=:1",
        "fn=: - alias",
        "protected=:+",
    ] {
        engine.eval(source).unwrap();
    }
    assert_eq!(
        engine.eval("fn i.4").unwrap().unwrap().json(),
        engine.eval("-i.4").unwrap().unwrap().json()
    );
    let graph = engine.analyze_j_graph("- alias i.4").unwrap();
    graph.verify().unwrap();
    assert_eq!(graph.modifier_snapshots[0].name, "alias");
    let version = engine.binding_version("protected");
    for (source, kind) in [
        ("protected=: - (\"'a')", "domain error"),
        ("protected=: - (\"1 2 3 4)", "length error"),
        ("protected=: - (@:3)", "domain error"),
    ] {
        let report = engine.eval_captured(source);
        report.capture.verify().unwrap();
        assert_eq!(report.result.unwrap_err().kind(), kind, "{source}");
        assert_eq!(engine.binding_version("protected"), version);
        assert_eq!(engine.prepare_semantic(source).unwrap_err().kind(), kind);
    }
}

#[test]
fn right_bound_verb_operand_preserves_construction_separately_from_executor_coverage() {
    use rustj::semantic::{ExprKind, FunctionHead, FunctionOperand};
    for source in ["+ (\"-)", "+ (@:-)"] {
        let program = semantic::parse(source).unwrap();
        let ExprKind::VerbValue(verb) = program.expression.unwrap().kind else {
            panic!();
        };
        assert!(matches!(
            verb.entity.head,
            FunctionHead::PrimitiveConjunction(_)
        ));
        assert_eq!(verb.entity.operands.len(), 2);
        let FunctionOperand::Function(right) = &verb.entity.operands[1] else {
            panic!();
        };
        assert_eq!(
            right.head,
            FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Subtract)
        );
    }
}

#[test]
fn left_bound_bident_uses_noun_input_as_rank_operand_and_alias_is_frozen() {
    let mut engine = rustj::Engine::new();
    for source in [
        "left=: -\"",
        "alias=:left",
        "left=:1",
        "rankval=:1",
        "fn=:rankval alias",
    ] {
        engine.eval(source).unwrap();
    }
    for (source, direct) in [
        ("fn i.2 3", "(-\"1) i.2 3"),
        ("(1 (-\")) i.2 3", "(-\"1) i.2 3"),
    ] {
        let expected = engine.eval(direct).unwrap().unwrap().json();
        assert_eq!(engine.eval(source).unwrap().unwrap().json(), expected);
        assert_eq!(
            engine
                .eval_semantic_reference(source)
                .unwrap()
                .unwrap()
                .json(),
            expected
        );
    }
    let graph = engine.analyze_j_graph("(1 alias) i.4").unwrap();
    graph.verify().unwrap();
    assert_eq!(graph.modifier_snapshots[0].name, "alias");
}

#[test]
fn successive_adverbs_keep_completed_nested_insert_and_hook_entities() {
    use rustj::semantic::{ExprKind, FunctionHead, FunctionOperand};
    for (source, head, count) in [
        (
            "+ (/ /)",
            FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert),
            1,
        ),
        ("+ (/ +)", FunctionHead::Hook, 2),
        (
            "+ (/ / /)",
            FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert),
            1,
        ),
    ] {
        let program = semantic::parse(source).unwrap();
        let ExprKind::VerbValue(verb) = program.expression.unwrap().kind else {
            panic!();
        };
        assert_eq!(verb.entity.head, head, "{source}");
        assert_eq!(verb.entity.operands.len(), count);
        let FunctionOperand::Function(child) = &verb.entity.operands[0] else {
            panic!();
        };
        assert_eq!(
            child.head,
            FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert)
        );
        assert_eq!(verb.span, 0..source.len());
    }
}

#[test]
fn left_binding_and_sequential_errors_stop_before_assignment() {
    let mut engine = rustj::Engine::new();
    engine.eval("protected=:+").unwrap();
    let version = engine.binding_version("protected");
    for (expression, kind) in [
        ("'a' (-\")", "domain error"),
        ("1 2 3 4 (-\")", "length error"),
        ("'a' ((-\") /)", "domain error"),
        ("3 (/@:)", "domain error"),
    ] {
        let source = format!("protected=: {expression}");
        let report = engine.eval_captured(&source);
        report.capture.verify().unwrap();
        assert_eq!(report.result.unwrap_err().kind(), kind, "{source}");
        assert_eq!(engine.binding_version("protected"), version);
        assert_eq!(engine.prepare_semantic(&source).unwrap_err().kind(), kind);
    }
}

#[test]
fn adverbial_hook_reuses_original_input_not_the_first_result() {
    use rustj::semantic::{ExprKind, FunctionHead, FunctionOperand};
    let program = semantic::parse("+ (/@:)").unwrap();
    let ExprKind::VerbValue(verb) = program.expression.unwrap().kind else {
        panic!();
    };
    assert_eq!(
        verb.entity.head,
        FunctionHead::PrimitiveConjunction(rustj::primitive::ConjunctionId::Atop)
    );
    let [
        FunctionOperand::Function(insert),
        FunctionOperand::Function(original),
    ] = verb.entity.operands.as_slice()
    else {
        panic!();
    };
    assert_eq!(
        insert.head,
        FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert)
    );
    let FunctionOperand::Function(input) = &insert.operands[0] else {
        panic!();
    };
    assert!(std::sync::Arc::ptr_eq(input, original));
    assert_eq!(
        original.head,
        FunctionHead::PrimitiveVerb(rustj::primitive::PrimitiveId::Add)
    );
}

#[test]
fn conjunction_actions_produce_completed_insert_hook_and_fork() {
    use rustj::semantic::{ExprKind, FunctionHead};
    for (source, head, count) in [
        (
            "+ (@:/) -",
            FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert),
            1,
        ),
        ("+ (@:@:) -", FunctionHead::Hook, 2),
        ("+ (/ / +) *", FunctionHead::Fork, 3),
        (
            "+ ((@:/) -)",
            FunctionHead::PrimitiveAdverb(rustj::primitive::AdverbId::Insert),
            1,
        ),
    ] {
        let program = semantic::parse(source).unwrap();
        let ExprKind::VerbValue(verb) = program.expression.unwrap().kind else {
            panic!();
        };
        assert_eq!(verb.entity.head, head, "{source}");
        assert_eq!(verb.entity.operands.len(), count);
        assert_eq!(
            verb.entity.result_pos,
            rustj::semantic::FunctionPartOfSpeech::Verb
        );
    }
}

#[test]
fn conjunction_sequence_errors_preserve_order_and_do_not_assign() {
    let mut engine = rustj::Engine::new();
    engine.eval("protected=:+").unwrap();
    let version = engine.binding_version("protected");
    for (expression, kind) in [
        ("+ (\" @:) 1 2 3 4", "length error"),
        ("+ (@: \") 1 2 3 4", "domain error"),
        ("3 (/@:)", "domain error"),
        ("3 (/ / +) 3", "domain error"),
    ] {
        let source = format!("protected=: {expression}");
        let report = engine.eval_captured(&source);
        report.capture.verify().unwrap();
        assert_eq!(report.result.unwrap_err().kind(), kind, "{source}");
        assert_eq!(engine.binding_version("protected"), version);
        assert_eq!(engine.prepare_semantic(&source).unwrap_err().kind(), kind);
    }
}

#[test]
fn modifier_trident_actions_preserve_intermediate_function_structure() {
    use rustj::primitive::{AdverbId, ConjunctionId};
    use rustj::semantic::{ExprKind, FunctionHead};
    for (source, expected) in [
        ("+ (+ + @:) -", FunctionHead::Fork),
        ("+ (3 + @:) -", FunctionHead::Fork),
        ("+ (@: + @:) -", FunctionHead::Fork),
        (
            "+ (@: / /) -",
            FunctionHead::PrimitiveAdverb(AdverbId::Insert),
        ),
        (
            "+ (+ @: /)",
            FunctionHead::PrimitiveConjunction(ConjunctionId::Atop),
        ),
        (
            "+ (+ @: @:) -",
            FunctionHead::PrimitiveConjunction(ConjunctionId::Atop),
        ),
        ("+ (/ + -)", FunctionHead::Fork),
        (
            "+ (/ @: -)",
            FunctionHead::PrimitiveConjunction(ConjunctionId::Atop),
        ),
        (
            "+ (/ @: /) -",
            FunctionHead::PrimitiveConjunction(ConjunctionId::Atop),
        ),
        (
            "+ (/ @: @:) -",
            FunctionHead::PrimitiveConjunction(ConjunctionId::Atop),
        ),
        ("+ (@: + -) *", FunctionHead::Fork),
        (
            "+ (@: @: -) *",
            FunctionHead::PrimitiveConjunction(ConjunctionId::Atop),
        ),
        (
            "+ (@: @: /) -",
            FunctionHead::PrimitiveConjunction(ConjunctionId::Atop),
        ),
    ] {
        let program = semantic::parse(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let ExprKind::VerbValue(verb) = program.expression.unwrap().kind else {
            panic!("{source}");
        };
        assert_eq!(verb.entity.head, expected, "{source}");
        assert_eq!(
            verb.entity.result_pos,
            rustj::semantic::FunctionPartOfSpeech::Verb
        );
    }
}

#[test]
fn trident_conjunction_branches_share_original_input_identity() {
    use rustj::semantic::{ExprKind, FunctionOperand};
    let program = semantic::parse("+ (@: + @:) -").unwrap();
    let ExprKind::VerbValue(verb) = program.expression.unwrap().kind else {
        panic!();
    };
    let FunctionOperand::Function(first) = &verb.entity.operands[0] else {
        panic!();
    };
    let FunctionOperand::Function(last) = &verb.entity.operands[2] else {
        panic!();
    };
    for index in 0..2 {
        let (FunctionOperand::Function(a), FunctionOperand::Function(b)) =
            (&first.operands[index], &last.operands[index])
        else {
            panic!();
        };
        assert!(std::sync::Arc::ptr_eq(a, b));
    }
}

#[test]
fn modifier_trident_errors_stop_before_later_actions_and_assignment() {
    let mut engine = rustj::Engine::new();
    engine.eval("protected=:+").unwrap();
    let version = engine.binding_version("protected");
    for (expression, kind) in [
        ("+ (\" + @:) 1 2 3 4", "length error"),
        ("+ (@: + \") 1 2 3 4", "domain error"),
        ("3 (+ @: /)", "domain error"),
        ("3 (/ @: /) +", "domain error"),
        ("3 (/ @: @:) 1 2 3 4", "domain error"),
        ("+ (\" @: /) 1 2 3 4", "length error"),
    ] {
        let source = format!("protected=: {expression}");
        let report = engine.eval_captured(&source);
        report.capture.verify().unwrap();
        assert_eq!(report.result.unwrap_err().kind(), kind, "{source}");
        assert_eq!(engine.binding_version("protected"), version);
        assert_eq!(engine.prepare_semantic(&source).unwrap_err().kind(), kind);
    }
}

#[test]
fn noun_left_rank_preserves_constant_and_rank_operands() {
    use rustj::semantic::{ExprKind, FunctionOperand};
    for source in ["3\"0", "1 2 3\"1 2", "'abc'\"_", "3\"+"] {
        let program = semantic::parse(source).unwrap();
        let ExprKind::VerbValue(verb) = program.expression.unwrap().kind else {
            panic!("{source}");
        };
        assert!(
            matches!(verb.entity.operands[0], FunctionOperand::Noun { .. }),
            "{source}"
        );
        assert_eq!(verb.entity.operands.len(), 2);
        assert_eq!(
            verb.entity.result_pos,
            rustj::semantic::FunctionPartOfSpeech::Verb
        );
    }
}

#[test]
fn noun_left_rank_audits_right_rank_before_gerund_and_assignment() {
    let mut engine = rustj::Engine::new();
    engine.eval("protected=:+").unwrap();
    let version = engine.binding_version("protected");
    for (expression, kind) in [
        ("3\"'a'", "domain error"),
        ("3\"1 2 3 4", "length error"),
        ("3\"(2 2$0)", "rank error"),
        ("(,<'bad')\"1 2 3 4", "length error"),
    ] {
        let source = format!("protected=: {expression}");
        let report = engine.eval_captured(&source);
        report.capture.verify().unwrap();
        assert_eq!(report.result.unwrap_err().kind(), kind, "{source}");
        assert_eq!(engine.binding_version("protected"), version);
    }
}

#[test]
fn noun_prefix_gerund_audit_matches_rank_length_domain_precedence() {
    let mut engine = rustj::Engine::new();
    engine.eval("protected=:+").unwrap();
    let version = engine.binding_version("protected");
    for (expression, kind) in [
        ("3\\", "domain error"),
        ("'abc'\\", "domain error"),
        ("(0$0)\\", "length error"),
        ("''\\", "length error"),
        ("(2 2$0)\\", "rank error"),
        ("(0 2$0)\\", "rank error"),
        ("(0$<0)\\", "length error"),
        ("(2 2$<0)\\", "rank error"),
    ] {
        let source = format!("protected=: {expression}");
        let report = engine.eval_captured(&source);
        assert_eq!(report.result.unwrap_err().kind(), kind, "{source}");
        report.capture.verify().unwrap();
        assert_eq!(engine.binding_version("protected"), version);
    }
    for (source, kind) in [("3\\", "domain error"), ("''\\", "length error")] {
        assert_eq!(semantic::parse(source).unwrap_err().kind(), kind);
    }
}

#[test]
fn primitive_gerund_construction_preserves_noun_and_fallback_constant() {
    use rustj::semantic::FunctionOperand;
    let mut engine = rustj::Engine::new();
    for source in [
        "gerundfn=: (, <'+')\\",
        "gerundfn=: ((<'+'),<'-')\\",
        "gerundfn=: (, <'+')\"0",
        "constantfn=: (, <3)\"0",
    ] {
        let report = engine.eval_captured(source);
        report
            .result
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        report.capture.verify().unwrap();
        assert!(report.capture.events.iter().any(|event| matches!(event,
            rustj::parser_capture::CaptureEvent::ConstructionSuccess { function, .. }
                if matches!(function.operands.first(), Some(FunctionOperand::Noun { .. })))));
    }
}

#[test]
fn gerund_leaf_audit_errors_preserve_order_and_quiet_rank_fallback() {
    let mut engine = rustj::Engine::new();
    engine.eval("protected=:+").unwrap();
    let version = engine.binding_version("protected");
    for (expression, kind) in [
        ("(, <3)\\", "domain error"),
        ("(, <'')\\", "length error"),
        ("(, <'/')\\", "domain error"),
        ("(, <'@:')\\", "domain error"),
        ("(, <(2 2$'+'))\\", "rank error"),
        ("((<3),<'')\\", "domain error"),
        ("((<''),<3)\\", "length error"),
    ] {
        let report = engine.eval_captured(&format!("protected=: {expression}"));
        assert_eq!(report.result.unwrap_err().kind(), kind, "{expression}");
        report.capture.verify().unwrap();
        assert_eq!(engine.binding_version("protected"), version);
    }
    for expression in ["(, <3)\"0", "(, <'')\"0", "(, <'/')\"0"] {
        engine.eval(&format!("constantfn=: {expression}")).unwrap();
    }
}

#[test]
fn compound_gerund_ar_success_preserves_original_noun_and_assignment() {
    use rustj::semantic::FunctionOperand;
    let mut engine = rustj::Engine::new();
    for representation in [
        "(<'/'),<(, <'+')",
        "(<'2'),<((<'+'),<'-')",
        "(<'3'),<((<'+'),(<'%'),<'#')",
        "(<'4'),<((<'+'),<'/')",
        "(<'\"'),<((<'+'),<((<'0'),<1))",
        "(<((<'4'),<((<'/'),<'/'))),<(, <'+')",
    ] {
        engine
            .eval(&format!("compoundar=: {representation}"))
            .unwrap();
        let report = engine.eval_captured("compoundfn=: (,<compoundar)\\");
        report
            .result
            .unwrap_or_else(|error| panic!("{representation}: {error:?}"));
        report.capture.verify().unwrap();
        let value = engine.eval("compoundar").unwrap().unwrap().json();
        let entity = report
            .capture
            .events
            .iter()
            .find_map(|event| match event {
                rustj::parser_capture::CaptureEvent::ConstructionSuccess { function, .. } => {
                    Some(function)
                }
                _ => None,
            })
            .unwrap();
        let FunctionOperand::Noun { value: noun, .. } = &entity.operands[0] else {
            panic!();
        };
        let rustj::Data::Boxed(_) = noun.data() else {
            panic!();
        };
        let rustj::Data::Boxed(elements) = noun.data() else {
            panic!();
        };
        assert_eq!(elements[0].json(), value);
    }
}

#[test]
fn compound_ar_errors_follow_constructor_order_and_quiet_rank_fallback() {
    let mut engine = rustj::Engine::new();
    engine.eval("protected=:+").unwrap();
    let version = engine.binding_version("protected");
    for (representation, kind) in [
        ("(<'2'),<((<3),<'')", "length error"),
        ("(<'2'),<((<''),<3)", "domain error"),
        ("(<'3'),<((<3),(<''),<'+')", "domain error"),
        ("(<'4'),<((<3),(<''),<3)", "domain error"),
        ("(<'2'),<(, <'+')", "length error"),
        ("(<'/'),<(, <'/')", "domain error"),
        ("(<'3'),<((<'+'),(<'/'),<'*')", "syntax error"),
    ] {
        engine
            .eval(&format!("compoundar=: {representation}"))
            .unwrap();
        let report = engine.eval_captured("protected=: (,<compoundar)\\");
        assert_eq!(report.result.unwrap_err().kind(), kind, "{representation}");
        report.capture.verify().unwrap();
        assert_eq!(engine.binding_version("protected"), version);
        engine.eval("constantfn=: (,<compoundar)\"0").unwrap();
    }
}

#[test]
fn gerund_names_use_current_pos_and_allow_undefined_verb_references() {
    let mut engine = rustj::Engine::new();
    for source in [
        "namedfn=: (,<'futureverb')\\",
        "namedfn=: (,<'futureverb')\"0",
        "futureverb=:+",
        "namedfn=: (,<'futureverb')\\",
        "futureverb=:-",
        "namedfn=: (,<'futureverb')\\",
        "gerund_alias=:futureverb",
        "futureverb=:1",
        "namedfn=: (,<'gerund_alias')\\",
    ] {
        let report = engine.eval_captured(source);
        report
            .result
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        report.capture.verify().unwrap();
    }
    engine.eval("protected=:+").unwrap();
    let version = engine.binding_version("protected");
    for binding in ["futureverb=:1", "futureverb=:/", "futureverb=:@:"] {
        engine.eval(binding).unwrap();
        let report = engine.eval_captured("protected=: (,<'futureverb')\\");
        assert_eq!(
            report.result.unwrap_err().kind(),
            "domain error",
            "{binding}"
        );
        report.capture.verify().unwrap();
        assert_eq!(engine.binding_version("protected"), version);
        engine.eval("rankfn=: (,<'futureverb')\"0").unwrap();
    }
}

#[test]
fn gerund_name_audit_rejects_malformed_names_before_lookup() {
    let mut engine = rustj::Engine::new();
    engine.eval("protected=:+").unwrap();
    let version = engine.binding_version("protected");
    for name in ["bad+", "bad name", "bad_", "a@"] {
        let source = format!("protected=: (,<'{name}')\\");
        let report = engine.eval_captured(&source);
        assert_eq!(
            report.result.unwrap_err().kind(),
            "ill-formed name",
            "{source}"
        );
        report.capture.verify().unwrap();
        assert_eq!(engine.binding_version("protected"), version);
    }
    let report = engine.eval_captured("protected=: (,<'fn_base_')\\");
    assert_eq!(report.result.unwrap_err().kind(), "unsupported");
    report.capture.verify().unwrap();
}

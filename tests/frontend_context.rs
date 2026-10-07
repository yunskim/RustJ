use rustj::{
    Engine,
    frontend_context::{
        ItemId, ItemProducer, NameEvidence, NamePolicy, NodeId, NodeKind, ParseStep, WordId,
    },
    parser::{ParseClass, ParseRow, match_parse_row, parse_frontend},
    semantic::ExprKind,
};
use std::sync::Arc;

#[test]
fn all_nine_actions_emit_replayable_pre_and_post_reduction_items() {
    for (source, row) in [
        ("+1", ParseRow::MonadEdge),
        ("+ - 1", ParseRow::MonadVVN),
        ("1+2*3", ParseRow::DyadNVN),
        ("+/", ParseRow::Adverb),
        ("+\"1", ParseRow::Conjunction),
        ("(+/ % #)", ParseRow::Fork),
        ("(+ #)", ParseRow::Hook),
        ("out=:1+2", ParseRow::Assignment),
        ("(+/ % #)", ParseRow::Parenthesis),
    ] {
        let program = parse_frontend(source).unwrap();
        let context = program.frontend.as_ref().unwrap();
        context
            .verify()
            .unwrap_or_else(|error| panic!("{source}: {error}"));
        let reduction = context.reductions.iter().find(|r| r.row == row).unwrap();
        assert!(
            reduction
                .consumed
                .iter()
                .all(|item| item.0 < reduction.produced.0)
        );
        assert_eq!(context.reductions.len(), program.reductions.len());
        assert!(context.complete);
    }
}

#[test]
fn deferred_arithmetic_links_exact_operand_occurrences_without_array_results() {
    let program = parse_frontend("a+c*d").unwrap();
    let context = program.frontend.as_ref().unwrap();
    context.verify().unwrap();
    let uses = &context.name_uses;
    assert_eq!(
        uses.iter().map(|u| u.word).collect::<Vec<_>>(),
        [WordId(4), WordId(2), WordId(0)]
    );
    for use_record in uses {
        assert_ne!(use_record.input, use_record.output);
        assert_eq!(context.items[use_record.input.0].class, ParseClass::Name);
        assert_eq!(use_record.policy, NamePolicy::CaptureAtRead);
        assert_eq!(use_record.evidence, NameEvidence::DiagnosticAssumption);
    }
    let multiplication = &context.reductions[0];
    let addition = &context.reductions[1];
    assert_eq!(addition.consumed[2], multiplication.produced);
    assert!(matches!(
        program.expression.unwrap().kind,
        ExprKind::Dyad { .. }
    ));
    assert_eq!(
        context
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Literal))
            .count(),
        0
    );
    assert_eq!(
        context
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Dyad { .. }))
            .count(),
        2
    );
}

#[test]
fn groups_preserve_multiple_items_for_one_semantic_result() {
    let program = parse_frontend("((1+2))").unwrap();
    let context = program.frontend.unwrap();
    context.verify().unwrap();
    let root = context.items[context.root.unwrap().0].semantic.unwrap();
    let origin = &context.origins[root.0];
    assert_eq!(origin.reductions.len(), 3);
    assert_eq!(origin.items.len(), 3);
    assert_eq!(
        context
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Dyad { .. }))
            .count(),
        1
    );
}

#[test]
fn failed_constructor_preserves_deferred_prefix_and_unconsumed_window() {
    let failure = parse_frontend("(+/\"(1+0)) 1 2").unwrap_err();
    assert_eq!(failure.error.kind(), "unsupported");
    let context = failure.context;
    context.verify().unwrap();
    assert!(!context.complete);
    assert_eq!(context.pending.as_ref().unwrap().row, ParseRow::Conjunction);
    assert!(
        context
            .nodes
            .iter()
            .any(|node| matches!(node.kind, NodeKind::Dyad { .. }))
    );
    assert!(
        !context
            .reductions
            .iter()
            .any(|r| r.row == ParseRow::Conjunction)
    );
    let error = parse_frontend("(+/)\"'bad'").unwrap_err();
    assert_eq!(error.error.kind(), "domain error");
    error.context.verify().unwrap();
    assert!(!error.context.complete);
}

#[test]
fn assignment_keeps_target_copula_value_and_namespace_order() {
    let program = parse_frontend("out=:1+2").unwrap();
    let context = program.frontend.unwrap();
    context.verify().unwrap();
    let root = context.items[context.root.unwrap().0].semantic.unwrap();
    let NodeKind::WriteName {
        target,
        copula,
        value,
    } = context.nodes[root.0].kind
    else {
        panic!("write result")
    };
    assert_eq!(
        context.items[target.0].producer,
        ItemProducer::Word(WordId(0))
    );
    assert_eq!(copula, WordId(1));
    assert!(context.words[copula.0].flags.global_assignment);
    assert!(matches!(context.nodes[value.0].kind, NodeKind::Dyad { .. }));
    assert_eq!(context.name_uses.len(), 0); // assignment target is not a read
    assert!(matches!(context.steps.last(), Some(ParseStep::Reduce(_))));
}

#[test]
fn graph_and_a3_keep_parser_context_and_reject_lost_origins() {
    let mut engine = Engine::new();
    engine.eval("a=:1 2 3").unwrap();
    let graph = engine.analyze_j_graph("a+a*a").unwrap();
    graph.verify().unwrap();
    let context = graph.frontend.as_ref().unwrap();
    assert_eq!(context.name_uses.len(), 3);
    assert!(
        context
            .name_uses
            .iter()
            .all(|u| u.evidence == NameEvidence::CatalogClass)
    );
    let mut bad = graph.clone();
    bad.parser_origins[0].clear();
    assert!(bad.verify().is_err());
    let a3 = engine.analyze_a3("a+a*a").unwrap();
    a3.verify().unwrap();
    let provenance = a3.parser_provenance.as_ref().unwrap();
    assert_eq!(provenance.context.name_uses.len(), 3);
    assert_eq!(provenance.graph_nodes.len(), a3.j_graph_node_count);
    assert!(a3.operations.iter().all(|op| op.j_origin.is_some()));
    let mut bad_a3 = a3.clone();
    bad_a3.parser_provenance.as_mut().unwrap().graph_nodes[0].clear();
    assert!(bad_a3.verify().is_err());
    assert!(Arc::ptr_eq(
        &provenance.context,
        &bad_a3.parser_provenance.as_ref().unwrap().context
    ));
}

#[test]
fn named_function_remains_late_and_nested_constructor_inputs_remain_reachable() {
    let mut engine = Engine::new();
    engine.eval("f=:+").unwrap();
    engine.eval("a=:1 2 3").unwrap();
    let program = engine.prepare_semantic("(f/ % #) a").unwrap().program;
    let context = program.frontend.unwrap();
    context.verify().unwrap();
    let named = context
        .name_uses
        .iter()
        .find(|u| context.words[u.word.0].name.as_deref() == Some("f"))
        .unwrap();
    assert_eq!(named.policy, NamePolicy::LateAtCall);
    let source_node = context.items[named.output.0].semantic.unwrap();
    assert!(context.nodes.iter().any(|node| matches!(&node.kind, NodeKind::Construct { inputs, .. } if inputs.contains(&source_node))));
    engine.eval("g=:f").unwrap();
    engine.eval("f=:*").unwrap();
    assert_eq!(engine.eval("g 3").unwrap().unwrap().int_at(0).unwrap(), 1);
}

#[test]
fn captured_modifier_policy_and_shared_derived_identity_survive_parsing() {
    let mut engine = Engine::new();
    engine.eval("adv=:/").unwrap();
    engine.eval("a=:1 2 3").unwrap();
    let bound = engine.prepare_semantic("+adv a").unwrap();
    let context = bound.program.frontend.as_ref().unwrap();
    context.verify().unwrap();
    let use_record = &context
        .name_uses
        .iter()
        .find(|u| context.words[u.word.0].name.as_deref() == Some("adv"))
        .unwrap();
    assert_eq!(use_record.policy, NamePolicy::CaptureAtRead);
    assert!(use_record.binding_version.is_some());
    let ExprKind::Monad { verb, .. } = &bound.program.expression.as_ref().unwrap().kind else {
        panic!("derived call")
    };
    assert!(context.nodes.iter().any(|node| matches!(&node.kind, NodeKind::Construct { function: Some(function), .. } if Arc::ptr_eq(function, &verb.entity))));
}

#[test]
fn trillion_element_catalog_keeps_only_structure_without_materializing_arrays() {
    use rustj::{
        facts::TypeFact, j_graph_ir::GraphFacts, static_analysis::StaticAnalyzer, types::DType,
    };
    let mut analyzer = StaticAnalyzer::new();
    analyzer
        .declare_noun(
            "a",
            GraphFacts {
                dtype: TypeFact::Exact(DType::Float),
                rank: Some(1),
                shape: Some(vec![1_000_000_000_000]),
            },
        )
        .unwrap();
    let analysis = analyzer.analyze("a+a*a").unwrap();
    let context = analysis.graph.frontend.as_ref().unwrap();
    context.verify().unwrap();
    assert_eq!(context.name_uses.len(), 3);
    assert!(
        !context
            .nodes
            .iter()
            .any(|node| matches!(node.kind, NodeKind::Literal))
    );
    assert!(
        analysis
            .graph
            .nodes
            .iter()
            .all(|node| !matches!(node.kind, rustj::j_graph_ir::NodeKind::Literal(_)))
    );
}

#[test]
fn malformed_context_links_are_errors_rather_than_panics_or_silent_repair() {
    let original = parse_frontend("1+2*3").unwrap().frontend.unwrap();
    let mut bad = (*original).clone();
    bad.reductions[0].produced = ItemId(usize::MAX);
    assert!(bad.verify().is_err());
    let mut bad = (*original).clone();
    bad.reductions[0].window.swap(1, 3);
    assert!(bad.verify().is_err());
    let mut bad = (*original).clone();
    bad.origins[0].items.clear();
    assert!(bad.verify().is_err());
    let mut bad = (*original).clone();
    let root = bad.items[bad.root.unwrap().0].semantic.unwrap();
    let NodeKind::Dyad { ref mut right, .. } = bad.nodes[root.0].kind else {
        panic!("dyad")
    };
    *right = NodeId(0); // valid ID, incorrect operand; range checks alone would pass
    assert!(bad.verify().is_err());
}

#[test]
fn empty_input_and_expanded_definition_words_have_valid_distinct_occurrences() {
    for source in ["", "NB. empty", "f=:{{ y + 1 }}"] {
        let program = parse_frontend(source).unwrap();
        let context = program.frontend.unwrap();
        context.verify().unwrap();
        assert!(context.complete);
        let queued = context
            .items
            .iter()
            .filter(|item| matches!(item.producer, ItemProducer::Word(_)))
            .count();
        assert_eq!(queued, context.words.len());
    }
}

#[test]
fn all_6561_class_windows_match_the_pinned_c_cases_table() {
    // Literal transcription of jsource 13994ffa jsrc/p.c::cases rows 0..8.
    // Masks and independent first-row search avoid calling Rust row predicates.
    use ParseClass::*;
    let n = 1u16 << 0;
    let v = 1u16 << 1;
    let a = 1u16 << 2;
    let c = 1u16 << 3;
    let name = 1u16 << 4;
    let assign = 1u16 << 5;
    let l = 1u16 << 6;
    let r = 1u16 << 7;
    let m = 1u16 << 8;
    let any = (1u16 << 9) - 1;
    let edge = m | assign | l;
    let avn = a | v | n;
    let cavn = c | avn;
    let rows = [
        [edge, v, n, any],
        [edge | avn, v, v, n],
        [edge | avn, n, v, n],
        [edge | avn, v | n, a, any],
        [edge | avn, v | n, c, v | n],
        [edge | avn, v | n, v, v],
        [edge, cavn, cavn, any],
        [name | n, assign, cavn, any],
        [l, cavn, r, any],
    ];
    let classes = [
        Noun,
        Verb,
        Adverb,
        Conjunction,
        Name,
        Assignment,
        LParen,
        RParen,
        Mark,
    ];
    for i0 in 0..9 {
        for i1 in 0..9 {
            for i2 in 0..9 {
                for i3 in 0..9 {
                    let indices = [i0, i1, i2, i3];
                    let expected = rows.iter().position(|row| {
                        (0..4).all(|slot| row[slot] & (1u16 << indices[slot]) != 0)
                    });
                    assert_eq!(
                        match_parse_row(indices.map(|i| classes[i])).map(|r| r as usize),
                        expected
                    );
                }
            }
        }
    }
}

fn definition_code(source: &str) -> Arc<rustj::definition_code::DefinitionCode> {
    let program = parse_frontend(source).unwrap();
    let function = match program.expression.unwrap().kind {
        ExprKind::VerbValue(verb) => verb.entity,
        ExprKind::ModifierValue(function) => function,
        _ => panic!("definition function"),
    };
    let rustj::semantic::FunctionHead::ExplicitDefinition(code) = &function.head else {
        panic!("definition code")
    };
    code.clone()
}

#[test]
fn explicit_and_direct_body_name_plans_preserve_local_global_roles_without_pos_freezing() {
    use rustj::definition_code::DefinitionNameRole as Role;
    for source in [
        "f=:1 : 0\nt=.t+u\npublished=:t\nt\n)",
        "f=:{{ t=.t+u\npublished=:t\nt }}",
    ] {
        let code = definition_code(source);
        code.verify().unwrap();
        assert_eq!(code.name_plan.monad.local_declarations, ["t"]);
        let roles: Vec<_> = code
            .name_plan
            .monad
            .occurrences
            .iter()
            .map(|name| (&code.body[name.span.clone()], name.role))
            .collect();
        assert!(roles.contains(&("t", Role::LocalAssignmentTarget)));
        assert!(roles.contains(&("published", Role::GlobalAssignmentTarget)));
        assert!(roles.contains(&("t", Role::ReadCurrentFrameThenGlobal)));
        assert!(roles.contains(&("u", Role::ReadCurrentFrameThenGlobal)));
        let mut bad = (*code).clone();
        bad.name_plan.monad.local_declarations.clear();
        assert!(bad.verify().is_err());
    }
}

#[test]
fn definition_local_declarations_are_per_valence_and_dynamic_targets_are_explicit() {
    let code = definition_code("f=:3 : 0\na=.y\na\n:\nb=.x+y\nb\n)");
    assert_eq!(code.name_plan.monad.local_declarations, ["a"]);
    assert_eq!(code.name_plan.dyad.local_declarations, ["b"]);
    let code = definition_code("f=:1 : '''computed''=.u'");
    assert_eq!(code.name_plan.monad.dynamic_assignment_targets.len(), 1);
    assert!(code.name_plan.monad.local_declarations.is_empty());
}

#[test]
fn explicit_and_direct_locals_fall_back_without_leaking_or_capturing_function_closures() {
    for definition in ["make=:1 : 0\nt=.u\nt/\n)", "make=:{{ t=.u\nt/ }}"] {
        let mut engine = Engine::new();
        engine.eval("t=:+").unwrap();
        let global_version = engine.binding_version("t");
        engine.eval(definition).unwrap();
        engine.eval("result=:-make").unwrap();
        assert_eq!(
            engine
                .eval("result 1 2 3")
                .unwrap()
                .unwrap()
                .int_at(0)
                .unwrap(),
            6
        );
        assert_eq!(engine.binding_version("t"), global_version);
        engine.eval("t=:-").unwrap();
        assert_eq!(
            engine
                .eval("result 1 2 3")
                .unwrap()
                .unwrap()
                .int_at(0)
                .unwrap(),
            2
        );
    }
    for definition in ["add=:1 : 't=.t+u'", "add=:{{ t=.t+u }}"] {
        let mut engine = Engine::new();
        engine.eval("t=:10").unwrap();
        engine.eval(definition).unwrap();
        assert_eq!(
            engine.eval("2 add").unwrap().unwrap().int_at(0).unwrap(),
            12
        );
        assert_eq!(
            engine.eval("3 add").unwrap().unwrap().int_at(0).unwrap(),
            13
        );
        assert_eq!(engine.eval("t").unwrap().unwrap().int_at(0).unwrap(), 10);
    }
}

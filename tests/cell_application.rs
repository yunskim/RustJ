use rustj::{
    Engine,
    contracts::{RankContract, RankSpec},
    facts::{CellApplyBoundary, RepeatedSide},
    logical_ir::{OpKind, Plan},
};
fn geometry(plan: &Plan) -> rustj::facts::CellApplicationPlan {
    let value = plan.result.unwrap();
    let op = &plan.operations[plan.values[value.0].producer.0];
    let call = match &op.kind {
        OpKind::Basis { call, .. } | OpKind::SemanticCall(call) => call,
        _ => panic!("call"),
    };
    call.cell_application(
        call.left.map(|v| &plan.values[v.0].facts),
        &plan.values[call.right.0].facts,
    )
    .unwrap()
}
#[test]
fn nested_explicit_and_innate_boundaries_remain_distinct() {
    let mut e = Engine::new();
    e.eval("a=:i.2 3 4").unwrap();
    let p = e.analyze_a3("(+\"0)\"1 a").unwrap();
    let g = geometry(&p);
    assert_eq!(g.layers.len(), 3);
    assert_eq!(g.layers[0].boundary, CellApplyBoundary::Explicit);
    assert_eq!(g.layers[0].right_frame, vec![2, 3]);
    assert_eq!(g.layers[1].right_frame, vec![4]);
    assert_eq!(g.layers[2].boundary, CellApplyBoundary::Innate);
    let p = e.analyze_a3("% a").unwrap();
    assert_eq!(geometry(&p).layers[0].right_frame, vec![2, 3, 4]);
}
#[test]
fn prefix_repetition_and_incompatible_frames_are_preserved() {
    let mut e = Engine::new();
    for s in ["a=:i.2 3", "b=:i.2 3 4", "c=:i.5 4"] {
        e.eval(s).unwrap();
    }
    let p = e.analyze_a3("a+\"1 0 b").unwrap();
    let g = geometry(&p);
    let l = &g.layers[0];
    assert_eq!(l.common_frame_prefix, Some(vec![2]));
    assert_eq!(l.repeated_side, Some(RepeatedSide::Left));
    assert_eq!(l.iteration_count, Some(24));
    assert_eq!(l.right_residual_frame, vec![3, 4]);
    let p = e.analyze_a3("a+\"1 c").unwrap();
    let g = geometry(&p);
    assert_eq!(g.layers[0].result_frame, None);
    assert_eq!(g.layers.len(), 1);
}
#[test]
fn empty_frames_require_fill_and_empty_cells_still_execute() {
    let mut e = Engine::new();
    for s in ["a=:i.0 3", "b=:i.2 0"] {
        e.eval(s).unwrap();
    }
    let g = geometry(&e.analyze_a3("+/\"1 a").unwrap());
    assert!(g.layers[0].requires_empty_frame_prototype);
    assert_eq!(g.layers[0].iteration_count, Some(0));
    let g = geometry(&e.analyze_a3("+/\"1 b").unwrap());
    assert!(!g.layers[0].requires_empty_frame_prototype);
    assert_eq!(g.layers[0].iteration_count, Some(2));
    assert_eq!(g.layers[0].right_cell, vec![0]);
}
#[test]
fn ranks_are_resolved_without_sentinel_arithmetic_or_runtime_changes() {
    assert_eq!(RankSpec::Infinite.resolve(3), 3);
    assert_eq!(RankSpec::Relative(i64::MIN).resolve(3), 0);
    assert_eq!(RankSpec::Relative(-1).resolve(3), 2);
    assert_eq!(
        RankContract::from_normalized([63; 3]).monad,
        RankSpec::Infinite
    );
    let mut e = Engine::new();
    e.eval("a=:i.2 3").unwrap();
    let g = geometry(&e.analyze_a3("+\"_1 a").unwrap());
    assert_eq!(g.layers[0].effective_monad_rank, Some(1));
    let g = geometry(&e.analyze_a3("+\"_ a").unwrap());
    assert_eq!(g.layers[0].requested.monad, RankSpec::Infinite);
}

#[test]
fn unresolved_shapes_do_not_synthesize_geometry_or_execution_proof() {
    let mut e = Engine::new();
    e.eval("a=:i.2 3").unwrap();
    let p = e.analyze_a3("% a").unwrap();
    let op = &p.operations[p.values[p.result.unwrap().0].producer.0];
    let call = match &op.kind {
        OpKind::Basis { call, .. } | OpKind::SemanticCall(call) => call,
        _ => panic!(),
    };
    assert!(
        call.cell_application(None, &rustj::facts::Facts::default())
            .is_none()
    );
    assert_eq!(
        rustj::contracts::innate_rank(rustj::primitive::PrimitiveId::Divide).monad,
        RankSpec::Absolute(0)
    );
}

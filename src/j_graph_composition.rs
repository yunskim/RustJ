//! Overlapping, analysis-only composition relations derived from J Graph.
//! Common-input candidates do not prove independence, purity or reordering.
//! Ordered relations refer to completion of entire invocations (including any
//! nested regions), not just the final scalar operation of each invocation.

use crate::{
    j_graph_ir::{GraphForm, NodeKind, Plan, RegionId, RegionKind, ValueId},
    semantic::{ForkSemantics, FunctionEntity, FunctionHead, FunctionOperand},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputSlot {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionConstructor {
    Composition,
    Hook,
    OrdinaryFork,
    CappedFork,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NestedBoundary {
    RankCell,
    Reduction,
    PrefixWindow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositionRelation {
    /// Exact input occurrence, including repeated use by both dyadic slots.
    Vertical {
        producer: ValueId,
        consumer: ValueId,
        slot: InputSlot,
    },
    /// Source-level common-input branches only. ObservableOrder remains a
    /// dependency until an independent effect/error legality witness exists.
    HorizontalCandidate {
        region: RegionId,
        inputs: Vec<ValueId>,
        branch_results: Vec<ValueId>,
    },
    /// Preserved J order, even when operation effects/errors are unknown.
    ObservableOrder {
        region: RegionId,
        constructor: RegionConstructor,
        completion_order: Vec<ValueId>,
        live_across: Vec<ValueId>,
    },
    /// A function boundary inside this applied node, not an invented applied
    /// inner ValueId. Operand paths address the original immutable entity DAG.
    Nested {
        owner: ValueId,
        function_path: Vec<usize>,
        boundary: NestedBoundary,
    },
    /// Noun-left graph expansion remains unsupported. The original snapshot
    /// stays in FunctionEntity; there is no fabricated f branch or parallelism.
    RetainedNounBoundary {
        owner: ValueId,
        operand_path: Vec<usize>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionAnalysis {
    pub relations: Vec<CompositionRelation>,
    /// Includes plan result/write consumers, following Plan::use_counts.
    pub use_counts: Vec<usize>,
}

impl CompositionAnalysis {
    pub fn from_plan(plan: &Plan) -> Result<Self, String> {
        plan.verify()?;
        Ok(Self::derive(plan))
    }

    /// Re-derive from verified topology, rejecting missing/extra/stale edges,
    /// constructor identities, input occurrences, order and fan-out summaries.
    /// The sidecar is not an executable proof or a reusable binding guard.
    pub fn verify(&self, plan: &Plan) -> Result<(), String> {
        plan.verify()?;
        if *self != Self::derive(plan) {
            return Err("composition sidecar does not match verified J graph topology".into());
        }
        Ok(())
    }

    fn derive(plan: &Plan) -> Self {
        let mut relations = Vec::new();
        for (index, node) in plan.nodes.iter().enumerate() {
            let NodeKind::Apply {
                function,
                left,
                right,
                ..
            } = &node.kind
            else {
                continue;
            };
            let owner = ValueId(index);
            if let Some(producer) = left {
                relations.push(CompositionRelation::Vertical {
                    producer: *producer,
                    consumer: owner,
                    slot: InputSlot::Left,
                });
            }
            relations.push(CompositionRelation::Vertical {
                producer: *right,
                consumer: owner,
                slot: InputSlot::Right,
            });
            nested(function, owner, &mut Vec::new(), &mut relations);
            if function.head == FunctionHead::Fork
                && function.fork_semantics != Some(ForkSemantics::Capped)
                && matches!(
                    function.operands.first(),
                    Some(FunctionOperand::Noun { .. })
                )
            {
                relations.push(CompositionRelation::RetainedNounBoundary {
                    owner,
                    operand_path: vec![0],
                });
            }
        }
        for (index, region) in plan.regions.iter().enumerate() {
            let id = RegionId(index);
            let (constructor, completion_order, live_across) = match &region.kind {
                RegionKind::Pipeline { stage_results } => (
                    if region.function.fork_semantics == Some(ForkSemantics::Capped) {
                        RegionConstructor::CappedFork
                    } else {
                        RegionConstructor::Composition
                    },
                    stage_results.clone(),
                    Vec::new(),
                ),
                RegionKind::Hook {
                    branch_results,
                    join_result,
                    live_across,
                } => (
                    RegionConstructor::Hook,
                    vec![branch_results[1], *join_result],
                    live_across.clone(),
                ),
                RegionKind::Fork {
                    branch_results,
                    join_result,
                    live_across,
                } => {
                    relations.push(CompositionRelation::HorizontalCandidate {
                        region: id,
                        inputs: region.inputs.clone(),
                        branch_results: branch_results.clone(),
                    });
                    let mut order = branch_results.clone();
                    order.push(*join_result);
                    (RegionConstructor::OrdinaryFork, order, live_across.clone())
                }
            };
            relations.push(CompositionRelation::ObservableOrder {
                region: id,
                constructor,
                completion_order,
                live_across,
            });
        }
        Self {
            relations,
            use_counts: plan.use_counts(),
        }
    }
}

fn nested(
    function: &std::sync::Arc<FunctionEntity>,
    owner: ValueId,
    path: &mut Vec<usize>,
    relations: &mut Vec<CompositionRelation>,
) {
    let (form, _) = crate::j_graph_ir::classify_function(function);
    let boundary = match &form {
        GraphForm::Rank { .. } => Some(NestedBoundary::RankCell),
        GraphForm::Reduce { .. } => Some(NestedBoundary::Reduction),
        GraphForm::PrefixInfix { .. } => Some(NestedBoundary::PrefixWindow),
        _ => None,
    };
    if let Some(boundary) = boundary {
        relations.push(CompositionRelation::Nested {
            owner,
            function_path: path.clone(),
            boundary,
        });
    }
    for (index, operand) in function.operands.iter().enumerate() {
        // Copy Rank's right function supplies a constructor header; its body
        // is not executed by this call. Do not invent a nested computation.
        if matches!(form, GraphForm::Rank { .. }) && index != 0 {
            continue;
        }
        if let FunctionOperand::Function(inner) = operand {
            path.push(index);
            nested(inner, owner, path, relations);
            path.pop();
        }
    }
}

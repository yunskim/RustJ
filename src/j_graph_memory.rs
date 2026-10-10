//! Static logical-memory analysis for the JAXA-style J Graph IR.
//!
//! J Graph can often determine array extents and lifetimes before choosing a
//! physical representation.  This module therefore reports logical atom counts,
//! liveness and materialization opportunities first.  Byte counts are evaluated
//! only through an explicit representation model; register/shared-memory/buffer
//! placement still belongs to later planning.

use crate::{
    facts::TypeFact,
    j_graph_ir::{NodeKind, Plan, RegionKind, ValueId},
    types::DType,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalExtent {
    pub shape: Vec<usize>,
    pub atoms: usize,
    pub dtype: TypeFact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphOrderLiveRange {
    pub value: ValueId,
    pub defined_at: usize,
    pub last_use: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaterializationOpportunity {
    /// Internal @: stage result may be forwarded directly to the next stage.
    PipelineIntermediate,
    /// Hook/fork input must stay live across sibling computation, but that does
    /// not imply an external-memory buffer.
    RetainedAcrossBranch,
    /// Hook/fork branch result may flow directly into the join.
    BranchIntermediate,
    /// Static reindex/view candidate may remain a virtual index mapping.
    VirtualView,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OpportunityValue {
    pub value: ValueId,
    pub kind: MaterializationOpportunity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticMemoryAnalysis {
    /// Extent is None when shape or atom count is not statically known.
    pub extents: Vec<Option<LogicalExtent>>,
    /// Lifetime intervals in canonical J-graph evaluation order. Physical
    /// schedules must recompute scheduled liveness rather than reuse these as
    /// allocation intervals.
    pub live_ranges: Vec<GraphOrderLiveRange>,
    pub opportunities: Vec<OpportunityValue>,
    /// Sum of known logical atoms for values that exist in the graph. This is
    /// not peak memory and not an allocation requirement.
    pub known_logical_atoms: usize,
}

pub trait AtomRepresentation {
    /// Bytes per atom for this chosen representation, if fixed and known.
    /// Variable-width/boxed/sparse representations should return None.
    fn bytes_per_atom(&self, dtype: DType) -> Option<usize>;
}

impl StaticMemoryAnalysis {
    pub fn extent(&self, value: ValueId) -> Option<&LogicalExtent> {
        self.extents.get(value.0)?.as_ref()
    }

    /// Evaluate one logical value under an explicit representation model.
    /// Returning None is intentional when dtype/representation is not fixed.
    pub fn represented_bytes(
        &self,
        value: ValueId,
        representation: &impl AtomRepresentation,
    ) -> Option<usize> {
        let extent = self.extent(value)?;
        let TypeFact::Exact(dtype) = extent.dtype else {
            return None;
        };
        extent
            .atoms
            .checked_mul(representation.bytes_per_atom(dtype)?)
    }

    /// Peak simultaneous bytes in the canonical J-graph evaluation order.
    /// This is a comparison baseline, not a schedule-independent upper bound:
    /// a legal optimizer may reorder pure operations and change the peak.
    /// It also assumes every included logical value is materialized.
    pub fn graph_order_peak_materialized_bytes(
        &self,
        representation: &impl AtomRepresentation,
    ) -> Option<usize> {
        if self.live_ranges.is_empty() {
            return Some(0);
        }
        let end = self
            .live_ranges
            .iter()
            .map(|range| range.last_use)
            .max()
            .unwrap_or(0);
        let mut peak = 0usize;
        for point in 0..=end {
            let mut live = 0usize;
            for range in &self.live_ranges {
                if range.defined_at <= point && point <= range.last_use {
                    live =
                        live.checked_add(self.represented_bytes(range.value, representation)?)?;
                }
            }
            peak = peak.max(live);
        }
        Some(peak)
    }
}

fn atom_count(shape: &[usize]) -> Option<usize> {
    if shape.contains(&0) {
        return Some(0);
    }
    shape
        .iter()
        .try_fold(1usize, |count, axis| count.checked_mul(*axis))
}

fn add_opportunity(
    opportunities: &mut Vec<OpportunityValue>,
    value: ValueId,
    kind: MaterializationOpportunity,
) {
    let item = OpportunityValue { value, kind };
    if !opportunities.contains(&item) {
        opportunities.push(item);
    }
}

pub fn analyze(plan: &Plan) -> StaticMemoryAnalysis {
    let extents = plan
        .nodes
        .iter()
        .map(|node| {
            let shape = node.facts.shape.clone()?;
            Some(LogicalExtent {
                atoms: atom_count(&shape)?,
                shape,
                dtype: node.facts.dtype,
            })
        })
        .collect::<Vec<_>>();

    let mut last_use = (0..plan.nodes.len()).collect::<Vec<_>>();
    for (consumer, node) in plan.nodes.iter().enumerate() {
        if let NodeKind::Apply { left, right, .. } = &node.kind {
            last_use[right.0] = last_use[right.0].max(consumer);
            if let Some(left) = left {
                last_use[left.0] = last_use[left.0].max(consumer);
            }
        }
    }

    // Returned/written values remain live through the end of the J graph.
    let graph_end = plan.nodes.len().saturating_sub(1);
    if let Some(result) = plan.result {
        last_use[result.0] = last_use[result.0].max(graph_end);
    }
    if let Some(write) = &plan.write {
        last_use[write.value.0] = last_use[write.value.0].max(graph_end);
    }

    let mut opportunities = Vec::new();
    for region in &plan.regions {
        match &region.kind {
            RegionKind::Pipeline { stage_results } => {
                for value in stage_results
                    .iter()
                    .copied()
                    .take(stage_results.len().saturating_sub(1))
                {
                    add_opportunity(
                        &mut opportunities,
                        value,
                        MaterializationOpportunity::PipelineIntermediate,
                    );
                }
            }
            RegionKind::Hook {
                branch_results,
                join_result,
                live_across,
            }
            | RegionKind::Fork {
                branch_results,
                join_result,
                live_across,
            } => {
                // Region topology extends retained inputs through the join even
                // when ordinary use-def alone would make their last use look earlier.
                for value in live_across {
                    last_use[value.0] = last_use[value.0].max(join_result.0);
                    add_opportunity(
                        &mut opportunities,
                        *value,
                        MaterializationOpportunity::RetainedAcrossBranch,
                    );
                }
                for value in branch_results {
                    if *value != *join_result && !region.inputs.contains(value) {
                        add_opportunity(
                            &mut opportunities,
                            *value,
                            MaterializationOpportunity::BranchIntermediate,
                        );
                    }
                }
            }
        }
    }

    for (index, node) in plan.nodes.iter().enumerate() {
        if let NodeKind::Apply { hints, .. } = &node.kind {
            if hints.contains(crate::j_graph_ir::GraphHint::VirtualIndexingCandidate) {
                add_opportunity(
                    &mut opportunities,
                    ValueId(index),
                    MaterializationOpportunity::VirtualView,
                );
            }
        }
    }

    let live_ranges = last_use
        .into_iter()
        .enumerate()
        .map(|(index, last_use)| GraphOrderLiveRange {
            value: ValueId(index),
            defined_at: index,
            last_use,
        })
        .collect::<Vec<_>>();

    let known_logical_atoms = extents
        .iter()
        .filter_map(|extent| extent.as_ref().map(|extent| extent.atoms))
        .fold(0usize, |sum, atoms| sum.saturating_add(atoms));

    StaticMemoryAnalysis {
        extents,
        live_ranges,
        opportunities,
        known_logical_atoms,
    }
}

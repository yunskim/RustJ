//! Symbolic resource composition over J Graph IR.
//!
//! This is the graph-side counterpart of the historical JAXA
//! compose_pipeline/compose_reduction/compose_branch/compose_join idea.
//! It composes logical extents/liveness and operation requirements without
//! choosing a concrete target schedule or memory space.

use crate::{
    j_graph_ir::{
        GraphOperationContract, NodeKind, Plan, RegionKind, ResourceCompositionRule,
        SymbolicResourceExpr, ValueId,
    },
    j_graph_memory::{MaterializationOpportunity, StaticMemoryAnalysis},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KnownAtoms {
    pub known: usize,
    pub has_unknown: bool,
}

impl KnownAtoms {
    fn add(&mut self, atoms: Option<usize>) {
        match atoms {
            Some(atoms) => self.known = self.known.saturating_add(atoms),
            None => self.has_unknown = true,
        }
    }

    fn max_with(&mut self, atoms: Option<usize>) {
        match atoms {
            Some(atoms) => self.known = self.known.max(atoms),
            None => self.has_unknown = true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeResourceSummary {
    pub value: ValueId,
    pub output_atoms: Option<usize>,
    pub temporary: SymbolicResourceExpr,
    pub accumulator: SymbolicResourceExpr,
    pub scratchpad: SymbolicResourceExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionResourceSummary {
    pub composition: ResourceCompositionRule,
    /// Logical internal edge volume which exists as values in the graph.
    pub internal_atoms: KnownAtoms,
    /// Subset of internal edge volume which may avoid external materialization
    /// if fusion/virtualization is selected.
    pub elidable_materialization_atoms: KnownAtoms,
    /// Values which must remain live across sibling/region work.
    pub retained_live_atoms: KnownAtoms,
    /// Peak logical atoms simultaneously live in canonical J-graph evaluation
    /// order. This is not schedule-independent; physical planning recomputes
    /// liveness after any legal reordering/fusion.
    pub graph_order_peak_live_atoms: KnownAtoms,
    /// Whether any child operation carries an accumulator requirement.
    pub has_reduction_accumulator: bool,
    /// Whether symbolic resource details remain unknown.
    pub has_unknown_resource_requirement: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphResourceSummary {
    pub nodes: Vec<NodeResourceSummary>,
    pub regions: Vec<RegionResourceSummary>,
}

fn extent_atoms(memory: &StaticMemoryAnalysis, value: ValueId) -> Option<usize> {
    memory.extent(value).map(|extent| extent.atoms)
}

fn node_contract(plan: &Plan, value: ValueId) -> Option<&GraphOperationContract> {
    match &plan.nodes.get(value.0)?.kind {
        NodeKind::Apply { contract, .. } => Some(contract),
        _ => None,
    }
}

fn region_graph_order_peak_live_atoms(
    memory: &StaticMemoryAnalysis,
    values: impl IntoIterator<Item = ValueId>,
) -> KnownAtoms {
    let values = values.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        return KnownAtoms::default();
    }

    let start = values
        .iter()
        .map(|value| memory.live_ranges[value.0].defined_at)
        .min()
        .unwrap_or(0);
    let end = values
        .iter()
        .map(|value| memory.live_ranges[value.0].last_use)
        .max()
        .unwrap_or(start);

    let mut peak = KnownAtoms::default();
    for point in start..=end {
        let mut live = KnownAtoms::default();
        for value in &values {
            let range = memory.live_ranges[value.0];
            if range.defined_at <= point && point <= range.last_use {
                live.add(extent_atoms(memory, *value));
            }
        }
        peak.known = peak.known.max(live.known);
        peak.has_unknown |= live.has_unknown;
    }
    peak
}

pub fn analyze(plan: &Plan, memory: &StaticMemoryAnalysis) -> GraphResourceSummary {
    let nodes = plan
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let (temporary, accumulator, scratchpad) = match &node.kind {
                NodeKind::Apply { contract, .. } => (
                    contract.temporary,
                    contract.accumulator,
                    contract.scratchpad,
                ),
                _ => (
                    SymbolicResourceExpr::Unknown,
                    SymbolicResourceExpr::Unknown,
                    SymbolicResourceExpr::Unknown,
                ),
            };
            NodeResourceSummary {
                value: ValueId(index),
                output_atoms: memory.extent(ValueId(index)).map(|extent| extent.atoms),
                temporary,
                accumulator,
                scratchpad,
            }
        })
        .collect::<Vec<_>>();

    let regions = plan
        .regions
        .iter()
        .map(|region| {
            let mut internal_atoms = KnownAtoms::default();
            let mut elidable = KnownAtoms::default();
            let mut retained = KnownAtoms::default();
            let mut has_accumulator = false;
            let mut has_unknown_resource = false;

            let region_values = match &region.kind {
                RegionKind::Pipeline { stage_results } => stage_results.clone(),
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
                    for value in live_across {
                        retained.add(extent_atoms(memory, *value));
                    }
                    let mut values = branch_results.clone();
                    if !values.contains(join_result) {
                        values.push(*join_result);
                    }
                    values
                }
            };

            for value in &region_values {
                if *value != region.result {
                    internal_atoms.add(extent_atoms(memory, *value));
                }
                if let Some(contract) = node_contract(plan, *value) {
                    has_accumulator |=
                        contract.accumulator == SymbolicResourceExpr::ReductionAccumulator;
                    has_unknown_resource |= matches!(
                        contract.temporary,
                        SymbolicResourceExpr::Unknown
                    ) || matches!(contract.accumulator, SymbolicResourceExpr::Unknown)
                        || matches!(contract.scratchpad, SymbolicResourceExpr::Unknown);
                } else {
                    has_unknown_resource = true;
                }
            }

            for opportunity in &memory.opportunities {
                if region_values.contains(&opportunity.value)
                    && matches!(
                        opportunity.kind,
                        MaterializationOpportunity::PipelineIntermediate
                            | MaterializationOpportunity::BranchIntermediate
                            | MaterializationOpportunity::VirtualView
                    )
                {
                    elidable.add(extent_atoms(memory, opportunity.value));
                }
            }

            let mut live_values = region_values.clone();
            for input in &region.inputs {
                if !live_values.contains(input) {
                    live_values.push(*input);
                }
            }

            RegionResourceSummary {
                composition: region.resource_composition,
                internal_atoms,
                elidable_materialization_atoms: elidable,
                retained_live_atoms: retained,
                graph_order_peak_live_atoms: region_graph_order_peak_live_atoms(memory, live_values),
                has_reduction_accumulator: has_accumulator,
                has_unknown_resource_requirement: has_unknown_resource,
            }
        })
        .collect();

    GraphResourceSummary { nodes, regions }
}

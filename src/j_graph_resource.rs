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
    j_graph_rewrite::{
        GraphRewriteCandidate, RewriteInput, RewriteNodeSemantics,
    },
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResourceExprId(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceExprNode {
    Zero,
    Unknown,
    /// Logical atom extent of a J Graph value.
    ValueAtoms(ValueId),
    /// A symbolic resource requirement whose concrete extent depends on later
    /// schedule/target decisions (for example an accumulator or window state).
    Requirement {
        value: ValueId,
        kind: SymbolicResourceExpr,
    },
    Sum(Vec<ResourceExprId>),
    Max(Vec<ResourceExprId>),
    Scale {
        factor: usize,
        input: ResourceExprId,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourceExprGraph {
    pub nodes: Vec<ResourceExprNode>,
}

impl ResourceExprGraph {
    fn push(&mut self, node: ResourceExprNode) -> ResourceExprId {
        let id = ResourceExprId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    fn zero(&mut self) -> ResourceExprId {
        self.push(ResourceExprNode::Zero)
    }

    fn unknown(&mut self) -> ResourceExprId {
        self.push(ResourceExprNode::Unknown)
    }

    fn value_atoms(&mut self, value: ValueId) -> ResourceExprId {
        self.push(ResourceExprNode::ValueAtoms(value))
    }

    fn requirement(
        &mut self,
        value: ValueId,
        kind: SymbolicResourceExpr,
    ) -> ResourceExprId {
        match kind {
            SymbolicResourceExpr::None => self.zero(),
            SymbolicResourceExpr::Unknown => self.unknown(),
            other => self.push(ResourceExprNode::Requirement { value, kind: other }),
        }
    }

    fn sum(&mut self, inputs: Vec<ResourceExprId>) -> ResourceExprId {
        match inputs.as_slice() {
            [] => self.zero(),
            [only] => *only,
            _ => self.push(ResourceExprNode::Sum(inputs)),
        }
    }

    fn max(&mut self, inputs: Vec<ResourceExprId>) -> ResourceExprId {
        match inputs.as_slice() {
            [] => self.zero(),
            [only] => *only,
            _ => self.push(ResourceExprNode::Max(inputs)),
        }
    }

    fn scale(&mut self, factor: usize, input: ResourceExprId) -> ResourceExprId {
        match factor {
            0 => self.zero(),
            1 => input,
            _ => self.push(ResourceExprNode::Scale { factor, input }),
        }
    }

    pub fn verify(&self, plan: &Plan) -> Result<(), &'static str> {
        for (index, node) in self.nodes.iter().enumerate() {
            match node {
                ResourceExprNode::ValueAtoms(value)
                | ResourceExprNode::Requirement { value, .. } => {
                    if value.0 >= plan.nodes.len() {
                        return Err("resource expression references an invalid J Graph value");
                    }
                }
                ResourceExprNode::Sum(inputs) | ResourceExprNode::Max(inputs) => {
                    if inputs.iter().any(|input| input.0 >= index) {
                        return Err("resource expression must reference earlier expression nodes");
                    }
                }
                ResourceExprNode::Scale { input, .. } => {
                    if input.0 >= index {
                        return Err("resource expression must reference an earlier expression node");
                    }
                }
                ResourceExprNode::Zero | ResourceExprNode::Unknown => {}
            }
        }
        Ok(())
    }

    pub fn evaluate_atoms(
        &self,
        root: ResourceExprId,
        memory: &StaticMemoryAnalysis,
    ) -> KnownAtoms {
        let mut values = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let value = match node {
                ResourceExprNode::Zero => KnownAtoms::default(),
                ResourceExprNode::Unknown | ResourceExprNode::Requirement { .. } => {
                    KnownAtoms {
                        known: 0,
                        has_unknown: true,
                    }
                }
                ResourceExprNode::ValueAtoms(value) => {
                    let mut atoms = KnownAtoms::default();
                    atoms.add(extent_atoms(memory, *value));
                    atoms
                }
                ResourceExprNode::Sum(inputs) => {
                    let mut out = KnownAtoms::default();
                    for input in inputs {
                        let item = values[input.0];
                        out.known = out.known.saturating_add(item.known);
                        out.has_unknown |= item.has_unknown;
                    }
                    out
                }
                ResourceExprNode::Max(inputs) => {
                    let mut out = KnownAtoms::default();
                    for input in inputs {
                        let item = values[input.0];
                        out.known = out.known.max(item.known);
                        out.has_unknown |= item.has_unknown;
                    }
                    out
                }
                ResourceExprNode::Scale { factor, input } => {
                    let item = values[input.0];
                    KnownAtoms {
                        known: item.known.saturating_mul(*factor),
                        has_unknown: item.has_unknown,
                    }
                }
            };
            values.push(value);
        }
        values.get(root.0).copied().unwrap_or(KnownAtoms {
            known: 0,
            has_unknown: true,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeResourceFormula {
    pub temporary: ResourceExprId,
    pub accumulator: ResourceExprId,
    pub working_state: ResourceExprId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResourceStateKind {
    Temporary,
    Accumulator,
    WorkingState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceStateLiveRange {
    pub owner: ValueId,
    pub kind: ResourceStateKind,
    pub requirement: ResourceExprId,
    /// Canonical J-graph order lower-bound lifetime. Physical fusion/scheduling
    /// may extend this interval but must not shorten semantic dependencies.
    pub defined_at: usize,
    pub last_use: usize,
    /// True when a legal fused realization may need to carry this state across
    /// neighboring operations/region boundaries.
    pub may_extend_across_fusion: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegionResourceFormula {
    pub internal_atoms: ResourceExprId,
    pub elidable_materialization_atoms: ResourceExprId,
    pub retained_live_atoms: ResourceExprId,
    pub graph_order_peak_live_atoms: ResourceExprId,
    /// Baseline logical traffic if every internal edge is written and read.
    pub unfused_internal_traffic_atoms: ResourceExprId,
    /// Logical read+write traffic which can potentially disappear if all
    /// currently-marked elidable edges stay virtual/fused.
    pub elidable_traffic_atoms: ResourceExprId,
    /// Structural temporary/accumulator/window-state requirements of child
    /// operations. Concrete sizes remain symbolic.
    /// Conservative structural upper bound: all child state requirements
    /// considered simultaneously live.
    pub operation_state_requirements: ResourceExprId,
    /// Peak requirement in canonical J-graph order using current lower-bound
    /// state live ranges.
    pub canonical_peak_operation_state_requirements: ResourceExprId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeResourceSummary {
    pub value: ValueId,
    pub output_atoms: Option<usize>,
    pub composition: ResourceCompositionRule,
    pub temporary: SymbolicResourceExpr,
    pub accumulator: SymbolicResourceExpr,
    pub working_state: SymbolicResourceExpr,
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
    /// Read+write atom traffic if all internal edges materialize.
    pub unfused_internal_traffic_atoms: KnownAtoms,
    /// Read+write atom traffic associated with currently-elidable edges.
    pub elidable_traffic_atoms: KnownAtoms,
    /// Whether any child operation carries an accumulator requirement.
    pub has_reduction_accumulator: bool,
    /// Whether symbolic resource details remain unknown.
    pub has_unknown_resource_requirement: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphResourceSummary {
    pub nodes: Vec<NodeResourceSummary>,
    pub regions: Vec<RegionResourceSummary>,
    /// Symbolic provenance for node requirements and region volume/liveness
    /// composition. Concrete target bytes are intentionally absent.
    pub expressions: ResourceExprGraph,
    pub node_formulas: Vec<NodeResourceFormula>,
    pub region_formulas: Vec<RegionResourceFormula>,
    pub state_live_ranges: Vec<ResourceStateLiveRange>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteAlternativeResourceProfile {
    /// Semantic output extent. Equivalence lets the replacement reuse the
    /// source-result extent even when internal candidate extents remain symbolic.
    pub output_atoms: Option<usize>,
    /// True when graph-visible formulas do not cover implementation-local
    /// resource behavior. This prevents a graph-visible zero from being read
    /// as a proven zero implementation cost.
    pub has_unknown_implementation_resource: bool,
    /// Candidate-local intermediates which would be materialized by an unfused
    /// realization. Unknown is explicit when the rewrite graph does not yet
    /// carry enough shape facts.
    pub internal_materialization_atoms: KnownAtoms,
    /// Baseline read+write traffic for those internal materialized values.
    pub unfused_internal_traffic_atoms: KnownAtoms,
    /// Portion of internal traffic which may disappear when the candidate is
    /// realized virtually/fused.
    pub elidable_internal_traffic_atoms: KnownAtoms,
    /// Target-independent operation-state requirements exposed by this
    /// alternative. Concrete size/placement remains downstream.
    pub state_requirements: Vec<SymbolicResourceExpr>,
    pub state_requirement_atoms: Vec<RewriteStateRequirement>,
    pub has_unknown_state_requirement: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RewriteStateRequirement {
    pub kind: SymbolicResourceExpr,
    pub atoms: KnownAtoms,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteResourceEvaluation {
    pub source_value: ValueId,
    pub source: RewriteAlternativeResourceProfile,
    pub replacement: RewriteAlternativeResourceProfile,
    /// This merely reports the rule contract. Resource comparison never grants
    /// pruning permission by itself.
    pub early_pruning_allowed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceMetricOrdering {
    SourceLower,
    Equal,
    ReplacementLower,
    Incomparable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RewriteResourceComparison {
    pub unfused_internal_traffic: ResourceMetricOrdering,
    pub elidable_internal_traffic: ResourceMetricOrdering,
}

fn compare_known_atoms(source: KnownAtoms, replacement: KnownAtoms) -> ResourceMetricOrdering {
    if source.has_unknown || replacement.has_unknown {
        return ResourceMetricOrdering::Incomparable;
    }
    match source.known.cmp(&replacement.known) {
        std::cmp::Ordering::Less => ResourceMetricOrdering::SourceLower,
        std::cmp::Ordering::Equal => ResourceMetricOrdering::Equal,
        std::cmp::Ordering::Greater => ResourceMetricOrdering::ReplacementLower,
    }
}

impl RewriteResourceEvaluation {
    pub fn comparison(&self) -> RewriteResourceComparison {
        let implementation_unknown = self.source.has_unknown_implementation_resource
            || self.replacement.has_unknown_implementation_resource;
        let compare = |source: KnownAtoms, replacement: KnownAtoms| {
            if implementation_unknown {
                ResourceMetricOrdering::Incomparable
            } else {
                compare_known_atoms(source, replacement)
            }
        };
        RewriteResourceComparison {
            unfused_internal_traffic: compare(
                self.source.unfused_internal_traffic_atoms,
                self.replacement.unfused_internal_traffic_atoms,
            ),
            elidable_internal_traffic: compare(
                self.source.elidable_internal_traffic_atoms,
                self.replacement.elidable_internal_traffic_atoms,
            ),
        }
    }
}


fn profile_state_requirements(
    temporary: SymbolicResourceExpr,
    accumulator: SymbolicResourceExpr,
    working_state: SymbolicResourceExpr,
) -> (Vec<SymbolicResourceExpr>, bool) {
    let mut out = Vec::new();
    let mut has_unknown = false;
    for kind in [temporary, accumulator, working_state] {
        match kind {
            SymbolicResourceExpr::None => {}
            SymbolicResourceExpr::Unknown => has_unknown = true,
            other => out.push(other),
        }
    }
    (out, has_unknown)
}

fn merge_state_requirement(
    requirements: &mut Vec<SymbolicResourceExpr>,
    has_unknown: &mut bool,
    kind: SymbolicResourceExpr,
) {
    match kind {
        SymbolicResourceExpr::None => {}
        SymbolicResourceExpr::Unknown => *has_unknown = true,
        other => {
            if !requirements.contains(&other) {
                requirements.push(other);
            }
        }
    }
}

fn graph_value_atoms(plan: &Plan, value: ValueId) -> Option<usize> {
    let shape = plan.nodes.get(value.0)?.facts.shape.as_ref()?;
    shape
        .iter()
        .try_fold(1usize, |atoms, dim| atoms.checked_mul(*dim))
}

fn rewrite_state_requirement_atoms(
    plan: &Plan,
    node: &crate::j_graph_rewrite::RewriteNode,
    kind: SymbolicResourceExpr,
) -> KnownAtoms {
    match (node.semantics, kind) {
        (
            RewriteNodeSemantics::WindowByPatternShape,
            SymbolicResourceExpr::WindowWorkingSet,
        ) => {
            let pattern = node.inputs.get(1).copied();
            let atoms = match pattern {
                Some(RewriteInput::Source(value)) => graph_value_atoms(plan, value),
                _ => None,
            };
            match atoms {
                Some(known) => KnownAtoms {
                    known,
                    has_unknown: false,
                },
                None => KnownAtoms {
                    known: 0,
                    has_unknown: true,
                },
            }
        }
        (_, SymbolicResourceExpr::None) => KnownAtoms::default(),
        _ => KnownAtoms {
            known: 0,
            has_unknown: true,
        },
    }
}

fn rewrite_node_atoms(node: &crate::j_graph_rewrite::RewriteNode) -> Option<usize> {
    let shape = node.facts.shape.as_ref()?;
    shape
        .iter()
        .try_fold(1usize, |atoms, dim| atoms.checked_mul(*dim))
}

fn rewrite_node_resource_requirements(
    semantics: RewriteNodeSemantics,
) -> (
    SymbolicResourceExpr,
    SymbolicResourceExpr,
    SymbolicResourceExpr,
) {
    match semantics {
        RewriteNodeSemantics::WindowByPatternShape => (
            SymbolicResourceExpr::None,
            SymbolicResourceExpr::None,
            SymbolicResourceExpr::WindowWorkingSet,
        ),
        RewriteNodeSemantics::MatchPatternCell => (
            SymbolicResourceExpr::StructuralComposition,
            SymbolicResourceExpr::None,
            SymbolicResourceExpr::None,
        ),
    }
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


fn resource_may_extend(kind: SymbolicResourceExpr) -> bool {
    matches!(
        kind,
        SymbolicResourceExpr::Unknown
            | SymbolicResourceExpr::StructuralComposition
            | SymbolicResourceExpr::ReductionAccumulator
            | SymbolicResourceExpr::WindowWorkingSet
    )
}

fn push_state_range(
    ranges: &mut Vec<ResourceStateLiveRange>,
    owner: ValueId,
    kind: ResourceStateKind,
    requirement_kind: SymbolicResourceExpr,
    requirement: ResourceExprId,
) {
    if matches!(requirement_kind, SymbolicResourceExpr::None) {
        return;
    }
    ranges.push(ResourceStateLiveRange {
        owner,
        kind,
        requirement,
        defined_at: owner.0,
        last_use: owner.0,
        may_extend_across_fusion: resource_may_extend(requirement_kind),
    });
}

fn canonical_peak_state_formula(
    expressions: &mut ResourceExprGraph,
    ranges: &[ResourceStateLiveRange],
) -> ResourceExprId {
    if ranges.is_empty() {
        return expressions.zero();
    }
    let start = ranges.iter().map(|range| range.defined_at).min().unwrap_or(0);
    let end = ranges.iter().map(|range| range.last_use).max().unwrap_or(start);
    let mut points = Vec::new();
    for point in start..=end {
        let active = ranges
            .iter()
            .filter(|range| range.defined_at <= point && point <= range.last_use)
            .map(|range| range.requirement)
            .collect::<Vec<_>>();
        points.push(expressions.sum(active));
    }
    expressions.max(points)
}

fn scale_known_atoms(input: KnownAtoms, factor: usize) -> KnownAtoms {
    KnownAtoms {
        known: input.known.saturating_mul(factor),
        has_unknown: input.has_unknown,
    }
}

fn sum_value_atoms(
    expressions: &mut ResourceExprGraph,
    values: impl IntoIterator<Item = ValueId>,
) -> ResourceExprId {
    let inputs = values
        .into_iter()
        .map(|value| expressions.value_atoms(value))
        .collect::<Vec<_>>();
    expressions.sum(inputs)
}

fn region_graph_order_peak_formula(
    expressions: &mut ResourceExprGraph,
    memory: &StaticMemoryAnalysis,
    values: &[ValueId],
) -> ResourceExprId {
    if values.is_empty() {
        return expressions.zero();
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

    let mut points = Vec::new();
    for point in start..=end {
        let live = values
            .iter()
            .copied()
            .filter(|value| {
                let range = memory.live_ranges[value.0];
                range.defined_at <= point && point <= range.last_use
            })
            .collect::<Vec<_>>();
        points.push(sum_value_atoms(expressions, live));
    }
    expressions.max(points)
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


pub fn evaluate_rewrite_candidate(
    plan: &Plan,
    memory: &StaticMemoryAnalysis,
    summary: &GraphResourceSummary,
    candidate: &GraphRewriteCandidate,
) -> Result<RewriteResourceEvaluation, &'static str> {
    candidate.verify(plan)?;
    let source_value = candidate.provenance.source_value;
    let source_summary = summary
        .nodes
        .get(source_value.0)
        .ok_or("rewrite source resource summary is missing")?;
    let (source_state_requirements, source_unknown_state) = profile_state_requirements(
        source_summary.temporary,
        source_summary.accumulator,
        source_summary.working_state,
    );
    let source = RewriteAlternativeResourceProfile {
        output_atoms: source_summary.output_atoms,
        has_unknown_implementation_resource: source_unknown_state,
        internal_materialization_atoms: KnownAtoms::default(),
        unfused_internal_traffic_atoms: KnownAtoms::default(),
        elidable_internal_traffic_atoms: KnownAtoms::default(),
        state_requirements: source_state_requirements,
        state_requirement_atoms: Vec::new(),
        has_unknown_state_requirement: source_unknown_state,
    };

    let mut internal = KnownAtoms::default();
    let mut elidable = KnownAtoms::default();
    let mut state_requirements = Vec::new();
    let mut state_requirement_atoms = Vec::new();
    let mut has_unknown_state = false;

    for (index, node) in candidate.replacement.nodes.iter().enumerate() {
        let (temporary, accumulator, working_state) =
            rewrite_node_resource_requirements(node.semantics);
        merge_state_requirement(
            &mut state_requirements,
            &mut has_unknown_state,
            temporary,
        );
        merge_state_requirement(
            &mut state_requirements,
            &mut has_unknown_state,
            accumulator,
        );
        merge_state_requirement(
            &mut state_requirements,
            &mut has_unknown_state,
            working_state,
        );
        for kind in [temporary, accumulator, working_state] {
            if matches!(kind, SymbolicResourceExpr::None | SymbolicResourceExpr::Unknown) {
                continue;
            }
            let atoms = rewrite_state_requirement_atoms(plan, node, kind);
            if !state_requirement_atoms
                .iter()
                .any(|item: &RewriteStateRequirement| item.kind == kind)
            {
                state_requirement_atoms.push(RewriteStateRequirement { kind, atoms });
            }
        }

        if index == candidate.replacement.output.0 {
            continue;
        }

        // A candidate-local intermediate has no canonical J Graph ValueId yet.
        // Keep its extent symbolic unless a rewrite-specific derivation exists.
        // WindowByPatternShape is intentionally virtualizable, so the same
        // unknown volume is also tracked as elidable rather than forced materialization.
        let atoms = match rewrite_node_atoms(node) {
            Some(known) => KnownAtoms {
                known,
                has_unknown: false,
            },
            None => KnownAtoms {
                known: 0,
                has_unknown: true,
            },
        };
        match node.semantics {
            RewriteNodeSemantics::WindowByPatternShape => {
                internal.known = internal.known.saturating_add(atoms.known);
                internal.has_unknown |= atoms.has_unknown;
                elidable.known = elidable.known.saturating_add(atoms.known);
                elidable.has_unknown |= atoms.has_unknown;
            }
            RewriteNodeSemantics::MatchPatternCell => {
                // Non-output match intermediates are not expected in the current
                // rule; retain the generic fact-driven accounting for future rules.
                internal.known = internal.known.saturating_add(atoms.known);
                internal.has_unknown |= atoms.has_unknown;
            }
        }
    }

    let replacement = RewriteAlternativeResourceProfile {
        output_atoms: extent_atoms(memory, source_value),
        has_unknown_implementation_resource:
            internal.has_unknown
                || has_unknown_state
                || state_requirement_atoms.iter().any(|item| item.atoms.has_unknown),
        internal_materialization_atoms: internal,
        unfused_internal_traffic_atoms: scale_known_atoms(internal, 2),
        elidable_internal_traffic_atoms: scale_known_atoms(elidable, 2),
        state_requirements,
        state_requirement_atoms,
        has_unknown_state_requirement: has_unknown_state,
    };

    let rule = crate::j_graph_rewrite::RULES
        .iter()
        .find(|rule| rule.id == candidate.rule)
        .ok_or("rewrite rule is not registered")?;

    Ok(RewriteResourceEvaluation {
        source_value,
        source,
        replacement,
        early_pruning_allowed: rule.pruning.sound_for_early_pruning(),
    })
}

pub fn evaluate_rewrite_candidates(
    plan: &Plan,
    candidates: &[GraphRewriteCandidate],
) -> Result<Vec<RewriteResourceEvaluation>, &'static str> {
    let memory = plan.static_memory_analysis();
    let summary = analyze(plan, &memory);
    candidates
        .iter()
        .map(|candidate| evaluate_rewrite_candidate(plan, &memory, &summary, candidate))
        .collect()
}

pub fn analyze(plan: &Plan, memory: &StaticMemoryAnalysis) -> GraphResourceSummary {
    let mut expressions = ResourceExprGraph::default();
    let mut node_formulas = Vec::with_capacity(plan.nodes.len());
    let mut state_live_ranges = Vec::new();
    let nodes = plan
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let (composition, temporary, accumulator, working_state) = match &node.kind {
                NodeKind::Apply {
                    contract,
                    resource_composition,
                    ..
                } => (
                    *resource_composition,
                    contract.temporary,
                    contract.accumulator,
                    contract.working_state,
                ),
                _ => (
                    ResourceCompositionRule::Unknown,
                    SymbolicResourceExpr::None,
                    SymbolicResourceExpr::None,
                    SymbolicResourceExpr::None,
                ),
            };
            let value = ValueId(index);
            let temporary_formula = expressions.requirement(value, temporary);
            let accumulator_formula = expressions.requirement(value, accumulator);
            let working_state_formula = expressions.requirement(value, working_state);
            node_formulas.push(NodeResourceFormula {
                temporary: temporary_formula,
                accumulator: accumulator_formula,
                working_state: working_state_formula,
            });
            push_state_range(
                &mut state_live_ranges,
                value,
                ResourceStateKind::Temporary,
                temporary,
                temporary_formula,
            );
            push_state_range(
                &mut state_live_ranges,
                value,
                ResourceStateKind::Accumulator,
                accumulator,
                accumulator_formula,
            );
            push_state_range(
                &mut state_live_ranges,
                value,
                ResourceStateKind::WorkingState,
                working_state,
                working_state_formula,
            );
            NodeResourceSummary {
                value,
                output_atoms: memory.extent(value).map(|extent| extent.atoms),
                composition,
                temporary,
                accumulator,
                working_state,
            }
        })
        .collect::<Vec<_>>();

    let mut region_formulas = Vec::with_capacity(plan.regions.len());
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
                if *value != region.result && !region.inputs.contains(value) {
                    internal_atoms.add(extent_atoms(memory, *value));
                }
                if region.inputs.contains(value) {
                    continue;
                }
                if let Some(contract) = node_contract(plan, *value) {
                    has_accumulator |=
                        contract.accumulator == SymbolicResourceExpr::ReductionAccumulator;
                    has_unknown_resource |= matches!(
                        contract.temporary,
                        SymbolicResourceExpr::Unknown
                    ) || matches!(contract.accumulator, SymbolicResourceExpr::Unknown)
                        || matches!(contract.working_state, SymbolicResourceExpr::Unknown);
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

            let internal_values = region_values
                .iter()
                .copied()
                .filter(|value| *value != region.result && !region.inputs.contains(value))
                .collect::<Vec<_>>();
            let elidable_values = memory
                .opportunities
                .iter()
                .filter(|opportunity| {
                    region_values.contains(&opportunity.value)
                        && matches!(
                            opportunity.kind,
                            MaterializationOpportunity::PipelineIntermediate
                                | MaterializationOpportunity::BranchIntermediate
                                | MaterializationOpportunity::VirtualView
                        )
                })
                .map(|opportunity| opportunity.value)
                .collect::<Vec<_>>();
            let retained_values = match &region.kind {
                RegionKind::Pipeline { .. } => Vec::new(),
                RegionKind::Hook { live_across, .. }
                | RegionKind::Fork { live_across, .. } => live_across.clone(),
            };
            let peak_formula =
                region_graph_order_peak_formula(&mut expressions, memory, &live_values);
            let internal_formula = sum_value_atoms(&mut expressions, internal_values);
            let elidable_formula = sum_value_atoms(&mut expressions, elidable_values);
            let retained_formula = sum_value_atoms(&mut expressions, retained_values);
            let unfused_traffic_formula = expressions.scale(2, internal_formula);
            let elidable_traffic_formula = expressions.scale(2, elidable_formula);

            let mut state_terms = Vec::new();
            for value in &region_values {
                if let Some(formula) = node_formulas.get(value.0) {
                    state_terms.push(formula.temporary);
                    state_terms.push(formula.accumulator);
                    state_terms.push(formula.working_state);
                }
            }
            let state_formula = expressions.sum(state_terms);
            let region_state_ranges = state_live_ranges
                .iter()
                .copied()
                .filter(|range| region_values.contains(&range.owner))
                .collect::<Vec<_>>();
            let canonical_peak_state_formula =
                canonical_peak_state_formula(&mut expressions, &region_state_ranges);

            region_formulas.push(RegionResourceFormula {
                internal_atoms: internal_formula,
                elidable_materialization_atoms: elidable_formula,
                retained_live_atoms: retained_formula,
                graph_order_peak_live_atoms: peak_formula,
                unfused_internal_traffic_atoms: unfused_traffic_formula,
                elidable_traffic_atoms: elidable_traffic_formula,
                operation_state_requirements: state_formula,
                canonical_peak_operation_state_requirements: canonical_peak_state_formula,
            });

            RegionResourceSummary {
                composition: region.resource_composition,
                internal_atoms,
                elidable_materialization_atoms: elidable,
                retained_live_atoms: retained,
                graph_order_peak_live_atoms: region_graph_order_peak_live_atoms(memory, live_values),
                unfused_internal_traffic_atoms: scale_known_atoms(internal_atoms, 2),
                elidable_traffic_atoms: scale_known_atoms(elidable, 2),
                has_reduction_accumulator: has_accumulator,
                has_unknown_resource_requirement: has_unknown_resource,
            }
        })
        .collect();

    debug_assert!(expressions.verify(plan).is_ok());
    GraphResourceSummary {
        nodes,
        regions,
        expressions,
        node_formulas,
        region_formulas,
        state_live_ranges,
    }
}

//! Algebraic rewrite candidates over J Graph IR.
//!
//! This layer sits above execution-basis expansion.  It never mutates or
//! destroys the source J Graph node.  A rewrite is an optional candidate with
//! an explicit equivalence witness and source provenance.

use crate::{
    j_graph_ir::{GraphBasis, GraphBasisKind, GraphFacts, NodeKind, Plan, ValueId},
    primitive::PrimitiveId,
};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RewriteNodeId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RewriteInput {
    Source(ValueId),
    Node(RewriteNodeId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RewriteNodeSemantics {
    /// Logical windows of y whose cell shape is the pattern x shape.
    WindowByPatternShape,
    /// Match x against each logical window.  This is graph semantics, not a
    /// commitment to a particular loop/kernel realization.
    MatchPatternCell,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteNode {
    pub basis: GraphBasisKind,
    pub inputs: Vec<RewriteInput>,
    pub semantics: RewriteNodeSemantics,
    /// Target-independent facts for this candidate-local value. Unknown is
    /// explicit; rewrite discovery must not invent shapes merely to aid costing.
    pub facts: GraphFacts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteGraph {
    pub nodes: Vec<RewriteNode>,
    pub output: RewriteNodeId,
}

impl RewriteGraph {
    pub fn verify(&self) -> Result<(), &'static str> {
        if self.output.0 >= self.nodes.len() {
            return Err("rewrite output is out of bounds");
        }
        for (index, node) in self.nodes.iter().enumerate() {
            for input in &node.inputs {
                if let RewriteInput::Node(source) = input {
                    if source.0 >= index {
                        return Err("rewrite node input must reference an earlier rewrite node");
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GraphRewriteRuleId {
    FindViaWindowMatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GraphEquivalenceWitness {
    /// J Dictionary identity already used by the execution expansion:
    /// x E. y  <->  ($x) x&-: ;.3 y
    JFindCutMatchIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RewriteFactRuleId {
    FindViaWindowMatchFacts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceBoundLocality {
    /// The rule has no proof that the relevant bound can be decided from the
    /// partial/local candidate alone.
    Unknown,
    /// A proof exists that the pruning bound is local to this rewrite.
    Local,
    /// The bound depends on surrounding fusion/tile/liveness context.
    GlobalContextDependent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PruningMonotonicity {
    /// A candidate may become better again after subsequent rewrites.
    Unproven,
    /// A proof exists that the relevant bound can only stay equal or worsen
    /// below this candidate in the search tree.
    ProvenMonotone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourcePruningContract {
    pub locality: ResourceBoundLocality,
    pub monotonicity: PruningMonotonicity,
}

impl ResourcePruningContract {
    pub const fn sound_for_early_pruning(self) -> bool {
        matches!(self.locality, ResourceBoundLocality::Local)
            && matches!(self.monotonicity, PruningMonotonicity::ProvenMonotone)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphRewriteRule {
    pub id: GraphRewriteRuleId,
    pub witness: GraphEquivalenceWitness,
    pub fact_rule: RewriteFactRuleId,
    pub source_outer_basis: GraphBasisKind,
    pub replacement_outer_basis: GraphBasisKind,
    /// Early resource pruning is forbidden unless this contract explicitly
    /// carries both a locality proof and a monotonicity proof.
    pub pruning: ResourcePruningContract,
}

/// Registry order is deterministic and is not a profitability ranking.
pub const RULES: &[GraphRewriteRule] = &[GraphRewriteRule {
    id: GraphRewriteRuleId::FindViaWindowMatch,
    witness: GraphEquivalenceWitness::JFindCutMatchIdentity,
    fact_rule: RewriteFactRuleId::FindViaWindowMatchFacts,
    source_outer_basis: GraphBasisKind::Search,
    replacement_outer_basis: GraphBasisKind::Window,
    pruning: ResourcePruningContract {
        locality: ResourceBoundLocality::GlobalContextDependent,
        monotonicity: PruningMonotonicity::Unproven,
    },
}];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GraphOptimizationPhase {
    BasisDiscovery,
    RewriteCandidateGeneration,
    EquivalenceValidation,
    CandidateResourceEvaluation,
    SoundResourcePruning,
}

pub const GRAPH_OPTIMIZATION_ORDER: &[GraphOptimizationPhase] = &[
    GraphOptimizationPhase::BasisDiscovery,
    GraphOptimizationPhase::RewriteCandidateGeneration,
    GraphOptimizationPhase::EquivalenceValidation,
    GraphOptimizationPhase::CandidateResourceEvaluation,
    GraphOptimizationPhase::SoundResourcePruning,
];

fn input_graph_facts(
    plan: &Plan,
    local_facts: &[GraphFacts],
    input: RewriteInput,
) -> Result<GraphFacts, &'static str> {
    match input {
        RewriteInput::Source(value) => plan
            .nodes
            .get(value.0)
            .map(|node| node.facts.clone())
            .ok_or("rewrite input references an invalid J Graph value"),
        RewriteInput::Node(value) => local_facts
            .get(value.0)
            .cloned()
            .ok_or("rewrite input references unavailable local facts"),
    }
}

fn infer_rewrite_facts(
    plan: &Plan,
    source_value: ValueId,
    rule: RewriteFactRuleId,
    replacement: &RewriteGraph,
) -> Result<Vec<GraphFacts>, &'static str> {
    let source_facts = plan
        .nodes
        .get(source_value.0)
        .map(|node| node.facts.clone())
        .ok_or("rewrite source facts are unavailable")?;
    let mut facts = Vec::with_capacity(replacement.nodes.len());

    for (index, node) in replacement.nodes.iter().enumerate() {
        let inferred = match (rule, node.semantics) {
            (
                RewriteFactRuleId::FindViaWindowMatchFacts,
                RewriteNodeSemantics::WindowByPatternShape,
            ) => {
                let Some(source_input) = node.inputs.first().copied() else {
                    return Err("window rewrite node has no source input");
                };
                let right = input_graph_facts(plan, &facts, source_input)?;
                let pattern = node
                    .inputs
                    .get(1)
                    .copied()
                    .map(|input| input_graph_facts(plan, &facts, input))
                    .transpose()?;

                // Current RustJ E. reference semantics are rank <= 1.  In that
                // supported subset the logical window family has one window per
                // right-argument position, with cell shape equal to the pattern
                // shape.  Trailing non-fitting windows remain logical positions
                // whose Match result is false; they do not disappear.
                let shape = match (
                    &right.shape,
                    pattern.as_ref().and_then(|x| x.shape.as_ref()),
                ) {
                    (Some(right_shape), Some(pattern_shape))
                        if right_shape.len() <= 1
                            && pattern_shape.len() <= right_shape.len()
                            && pattern_shape.len() <= 1 =>
                    {
                        let mut shape = right_shape.clone();
                        shape.extend_from_slice(pattern_shape);
                        Some(shape)
                    }
                    _ => None,
                };
                let rank = shape.as_ref().map(Vec::len);
                GraphFacts {
                    dtype: right.dtype,
                    shape,
                    rank,
                }
            }
            (
                RewriteFactRuleId::FindViaWindowMatchFacts,
                RewriteNodeSemantics::MatchPatternCell,
            ) => {
                if index != replacement.output.0 {
                    return Err("find rewrite match node must be the candidate output");
                }
                source_facts.clone()
            }
        };
        facts.push(inferred);
    }
    Ok(facts)
}

fn assign_rewrite_facts(
    plan: &Plan,
    source_value: ValueId,
    rule: RewriteFactRuleId,
    replacement: &mut RewriteGraph,
) -> Result<(), &'static str> {
    let inferred = infer_rewrite_facts(plan, source_value, rule, replacement)?;
    for (node, facts) in replacement.nodes.iter_mut().zip(inferred) {
        node.facts = facts;
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphRewriteProvenance {
    pub source_value: ValueId,
    pub source_span: Range<usize>,
    pub source_basis: GraphBasis,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphRewriteCandidate {
    pub provenance: GraphRewriteProvenance,
    pub rule: GraphRewriteRuleId,
    pub witness: GraphEquivalenceWitness,
    pub replacement: RewriteGraph,
}

impl GraphRewriteCandidate {
    pub fn verify(&self, plan: &Plan) -> Result<(), &'static str> {
        self.replacement.verify()?;
        let Some(source) = plan.nodes.get(self.provenance.source_value.0) else {
            return Err("rewrite source value is out of bounds");
        };
        if source.span != self.provenance.source_span {
            return Err("rewrite source provenance span is stale");
        }
        let NodeKind::Apply { basis, .. } = &source.kind else {
            return Err("rewrite source is not an applied J Graph operation");
        };
        if basis != &self.provenance.source_basis {
            return Err("rewrite source basis provenance is stale");
        }
        let Some(rule) = RULES.iter().find(|rule| rule.id == self.rule) else {
            return Err("rewrite rule is not registered");
        };
        if rule.witness != self.witness {
            return Err("rewrite equivalence witness does not match rule registry");
        }
        let expected_facts = infer_rewrite_facts(
            plan,
            self.provenance.source_value,
            rule.fact_rule,
            &self.replacement,
        )?;
        if self
            .replacement
            .nodes
            .iter()
            .map(|node| &node.facts)
            .ne(expected_facts.iter())
        {
            return Err("rewrite-local facts do not match registered fact rule");
        }
        if basis.layers.first().copied() != Some(rule.source_outer_basis) {
            return Err("rewrite source basis does not satisfy registered rule");
        }
        if self.replacement.nodes.first().map(|node| node.basis)
            != Some(rule.replacement_outer_basis)
        {
            return Err("rewrite replacement basis does not satisfy registered rule");
        }
        let output = &self.replacement.nodes[self.replacement.output.0];
        if output.facts != source.facts {
            return Err("rewrite output facts do not match equivalent source result");
        }
        Ok(())
    }
}

fn find_via_window_match(
    plan: &Plan,
    source_value: ValueId,
    source_basis: &GraphBasis,
    left: ValueId,
    right: ValueId,
) -> GraphRewriteCandidate {
    let window = RewriteNodeId(0);
    let matched = RewriteNodeId(1);
    let mut replacement = RewriteGraph {
        nodes: vec![
            RewriteNode {
                basis: GraphBasisKind::Window,
                inputs: vec![RewriteInput::Source(right), RewriteInput::Source(left)],
                semantics: RewriteNodeSemantics::WindowByPatternShape,
                facts: GraphFacts::default(),
            },
            RewriteNode {
                basis: GraphBasisKind::CellApply,
                inputs: vec![RewriteInput::Source(left), RewriteInput::Node(window)],
                semantics: RewriteNodeSemantics::MatchPatternCell,
                facts: GraphFacts::default(),
            },
        ],
        output: matched,
    };
    assign_rewrite_facts(
        plan,
        source_value,
        RewriteFactRuleId::FindViaWindowMatchFacts,
        &mut replacement,
    )
    .expect("registered find rewrite facts must be derivable");

    GraphRewriteCandidate {
        provenance: GraphRewriteProvenance {
            source_value,
            source_span: plan.nodes[source_value.0].span.clone(),
            source_basis: source_basis.clone(),
        },
        rule: GraphRewriteRuleId::FindViaWindowMatch,
        witness: GraphEquivalenceWitness::JFindCutMatchIdentity,
        replacement,
    }
}

/// Discover algebraic candidates without choosing or applying them.
///
/// Candidate generation is target-independent. Resource/cost analysis may
/// inspect these later, but cannot erase candidates unless a sound pruning
/// contract is separately proven.
pub fn discover(plan: &Plan) -> Vec<GraphRewriteCandidate> {
    let mut candidates = Vec::new();

    for (index, node) in plan.nodes.iter().enumerate() {
        let NodeKind::Apply {
            function,
            basis,
            left,
            right,
            ..
        } = &node.kind
        else {
            continue;
        };
        if !matches!(
            &function.head,
            crate::semantic::FunctionHead::PrimitiveVerb(PrimitiveId::Find)
        ) {
            continue;
        }
        let Some(left) = *left else {
            continue;
        };
        let source_value = ValueId(index);
        let candidate = find_via_window_match(plan, source_value, basis, left, *right);
        debug_assert!(candidate.verify(plan).is_ok());
        candidates.push(candidate);
    }

    candidates
}

impl Plan {
    pub fn rewrite_candidates(&self) -> Vec<GraphRewriteCandidate> {
        discover(self)
    }
}

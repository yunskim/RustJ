//! Algebraic rewrite candidates over J Graph IR.
//!
//! This layer sits above execution-basis expansion.  It never mutates or
//! destroys the source J Graph node.  A rewrite is an optional candidate with
//! an explicit equivalence witness and source provenance.

use crate::{
    j_graph_ir::{GraphBasis, GraphBasisKind, NodeKind, Plan, ValueId},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphRewriteRule {
    pub id: GraphRewriteRuleId,
    pub witness: GraphEquivalenceWitness,
    pub source_outer_basis: GraphBasisKind,
    pub replacement_outer_basis: GraphBasisKind,
}

/// Registry order is deterministic and is not a profitability ranking.
pub const RULES: &[GraphRewriteRule] = &[GraphRewriteRule {
    id: GraphRewriteRuleId::FindViaWindowMatch,
    witness: GraphEquivalenceWitness::JFindCutMatchIdentity,
    source_outer_basis: GraphBasisKind::Search,
    replacement_outer_basis: GraphBasisKind::Window,
}];

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
        if basis.layers.first().copied() != Some(rule.source_outer_basis) {
            return Err("rewrite source basis does not satisfy registered rule");
        }
        if self
            .replacement
            .nodes
            .first()
            .map(|node| node.basis)
            != Some(rule.replacement_outer_basis)
        {
            return Err("rewrite replacement basis does not satisfy registered rule");
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
    GraphRewriteCandidate {
        provenance: GraphRewriteProvenance {
            source_value,
            source_span: plan.nodes[source_value.0].span.clone(),
            source_basis: source_basis.clone(),
        },
        rule: GraphRewriteRuleId::FindViaWindowMatch,
        witness: GraphEquivalenceWitness::JFindCutMatchIdentity,
        replacement: RewriteGraph {
            nodes: vec![
                RewriteNode {
                    basis: GraphBasisKind::Window,
                    inputs: vec![RewriteInput::Source(right), RewriteInput::Source(left)],
                    semantics: RewriteNodeSemantics::WindowByPatternShape,
                },
                RewriteNode {
                    basis: GraphBasisKind::CellApply,
                    inputs: vec![RewriteInput::Source(left), RewriteInput::Node(window)],
                    semantics: RewriteNodeSemantics::MatchPatternCell,
                },
            ],
            output: matched,
        },
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
            function.head,
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

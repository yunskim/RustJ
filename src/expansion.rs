//! Optional execution-basis expansions for structured/semantic operations.
//!
//! Expansions are sidecar candidates: the original semantic operation remains
//! in the A3 plan until an optimizer explicitly chooses and proves an expansion.

use crate::{
    analysis::{ExecutionBasisKind, CallTarget},
    j_graph_rewrite::{GraphEquivalenceWitness, GraphRewriteRuleId},
    logical_ir::{ConstraintSet, OpId, OpKind, Plan, ValueId},
    primitive::PrimitiveId,
};

pub use crate::j_graph_rewrite::{
    GraphEquivalenceWitness as EquivalenceWitness,
    GraphRewriteRuleId as ExpansionRuleId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExpansionNodeId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExpansionInput {
    Source(ValueId),
    Node(ExpansionNodeId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExpansionNodeSemantics {
    /// Produce the tessellation/window family whose window shape is the
    /// pattern's shape.  This is the ;.3 side of the J Dictionary identity.
    WindowByPatternShape,
    /// Apply J Match between the source pattern and each logical window.
    MatchPatternCell,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpansionNode {
    pub basis: ExecutionBasisKind,
    pub inputs: Vec<ExpansionInput>,
    pub semantics: ExpansionNodeSemantics,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpansionGraph {
    pub nodes: Vec<ExpansionNode>,
    pub output: ExpansionNodeId,
}

impl ExpansionGraph {
    pub fn verify(&self) -> Result<(), &'static str> {
        if self.output.0 >= self.nodes.len() {
            return Err("expansion output is out of bounds");
        }
        for (index, node) in self.nodes.iter().enumerate() {
            for input in &node.inputs {
                if let ExpansionInput::Node(source) = input {
                    if source.0 >= index {
                        return Err("expansion node input must reference an earlier node");
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionBasisExpansion {
    pub source_op: OpId,
    pub source_result: ValueId,
    pub rule: GraphRewriteRuleId,
    pub graph: ExpansionGraph,
    pub applicability: ConstraintSet,
    pub witness: GraphEquivalenceWitness,
}

fn find_expansion(
    source_op: OpId,
    source_result: ValueId,
    left: ValueId,
    right: ValueId,
) -> ExecutionBasisExpansion {
    let window = ExpansionNodeId(0);
    let matched = ExpansionNodeId(1);
    ExecutionBasisExpansion {
        source_op,
        source_result,
        rule: GraphRewriteRuleId::FindViaWindowMatch,
        graph: ExpansionGraph {
            nodes: vec![
                ExpansionNode {
                    basis: ExecutionBasisKind::WindowView,
                    inputs: vec![
                        ExpansionInput::Source(right),
                        ExpansionInput::Source(left),
                    ],
                    semantics: ExpansionNodeSemantics::WindowByPatternShape,
                },
                ExpansionNode {
                    basis: ExecutionBasisKind::CellApply,
                    inputs: vec![
                        ExpansionInput::Source(left),
                        ExpansionInput::Node(window),
                    ],
                    semantics: ExpansionNodeSemantics::MatchPatternCell,
                },
            ],
            output: matched,
        },
        applicability: ConstraintSet::default(),
        witness: GraphEquivalenceWitness::JFindCutMatchIdentity,
    }
}

/// Discover optional expansions without rewriting the plan.
///
/// The source operation/result identity remains available for a direct native
/// implementation, a specialized library route, or runtime fallback.
pub fn discover(plan: &Plan) -> Vec<ExecutionBasisExpansion> {
    let mut expansions = Vec::new();

    for (index, operation) in plan.operations.iter().enumerate() {
        let OpKind::SemanticCall(call) = &operation.kind else {
            continue;
        };
        let CallTarget::Primitive(PrimitiveId::Find) = call.callable.target else {
            continue;
        };
        let Some(left) = call.left else {
            continue;
        };
        let Some(source_result) = operation.results.first().copied() else {
            continue;
        };

        let expansion = find_expansion(OpId(index), source_result, left, call.right);
        debug_assert!(expansion.graph.verify().is_ok());
        expansions.push(expansion);
    }

    expansions
}

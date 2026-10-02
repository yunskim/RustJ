//! Optional execution-basis expansions for structured/semantic operations.
//!
//! Expansions are sidecar candidates: the original semantic operation remains
//! in the A3 plan until an optimizer explicitly chooses and proves an expansion.

use crate::{
    Data, Error, Value,
    analysis::{CallTarget, ExecutionBasisKind},
    j_graph_rewrite::{GraphEquivalenceWitness, GraphRewriteCandidate, GraphRewriteRuleId},
    logical_ir::{
        ConstraintSet, ExecutionBasisPayload, OpId, OpKind, Plan, ValueId, WindowShapeSpec,
    },
    primitive::PrimitiveId,
};

pub use crate::j_graph_rewrite::{
    GraphEquivalenceWitness as EquivalenceWitness, GraphRewriteRuleId as ExpansionRuleId,
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
    /// Exact execution-basis semantics required by any later lowering.
    pub payload: ExecutionBasisPayload,
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

            match (node.semantics, &node.payload, node.inputs.as_slice()) {
                (
                    ExpansionNodeSemantics::WindowByPatternShape,
                    ExecutionBasisPayload::WindowView {
                        source,
                        shape: WindowShapeSpec::PatternShape { pattern },
                    },
                    [
                        ExpansionInput::Source(input_source),
                        ExpansionInput::Source(input_pattern),
                    ],
                ) if source == input_source && pattern == input_pattern => {}
                (
                    ExpansionNodeSemantics::MatchPatternCell,
                    ExecutionBasisPayload::CellApply,
                    [
                        ExpansionInput::Source(match_pattern),
                        ExpansionInput::Node(window_node),
                    ],
                ) => {
                    let Some(window) = self.nodes.get(window_node.0) else {
                        return Err("match expansion references a missing window node");
                    };
                    let ExecutionBasisPayload::WindowView {
                        shape: WindowShapeSpec::PatternShape { pattern },
                        ..
                    } = &window.payload
                    else {
                        return Err("match expansion input is not a WindowView");
                    };
                    if pattern != match_pattern {
                        return Err("match expansion pattern does not match WindowView pattern");
                    }
                }
                _ => {
                    return Err("expansion node semantic payload does not match its inputs");
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
                    inputs: vec![ExpansionInput::Source(right), ExpansionInput::Source(left)],
                    semantics: ExpansionNodeSemantics::WindowByPatternShape,
                    payload: ExecutionBasisPayload::WindowView {
                        source: right,
                        shape: WindowShapeSpec::PatternShape { pattern: left },
                    },
                },
                ExpansionNode {
                    basis: ExecutionBasisKind::CellApply,
                    inputs: vec![ExpansionInput::Source(left), ExpansionInput::Node(window)],
                    semantics: ExpansionNodeSemantics::MatchPatternCell,
                    payload: ExecutionBasisPayload::CellApply,
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

/// Resolve the execution expansion corresponding to an already-discovered
/// J Graph rewrite using provenance, not source reparsing/pattern recovery.
pub fn for_graph_rewrite(
    plan: &Plan,
    candidate: &GraphRewriteCandidate,
) -> Result<ExecutionBasisExpansion, &'static str> {
    let mut found = None;
    for expansion in discover(plan) {
        let Some(operation) = plan.operations.get(expansion.source_op.0) else {
            return Err("expansion source operation is out of bounds");
        };
        if operation.j_origin != Some(candidate.provenance.source_value)
            || expansion.rule != candidate.rule
            || expansion.witness != candidate.witness
        {
            continue;
        }
        if found.is_some() {
            return Err("multiple execution expansions match one graph rewrite");
        }
        found = Some(expansion);
    }
    found.ok_or("no execution expansion matches the graph rewrite provenance")
}

#[derive(Clone)]
enum ReferenceExpansionValue {
    WindowFamily { pattern: Value, source: Value },
    Value(Value),
}

fn source_value(values: &[Option<Value>], id: ValueId) -> crate::Result<Value> {
    values
        .get(id.0)
        .and_then(Option::as_ref)
        .cloned()
        .ok_or_else(|| Error::Unsupported("reference expansion source value is unavailable".into()))
}

fn find_window_family(pattern: Value, source: Value) -> crate::Result<ReferenceExpansionValue> {
    if pattern.is_sparse() || source.is_sparse() {
        return Err(Error::Unsupported("sparse dyad E.".into()));
    }
    if matches!(pattern.data(), Data::Boxed(_)) || matches!(source.data(), Data::Boxed(_)) {
        return Err(Error::Unsupported("boxed search".into()));
    }
    if pattern.shape().len() > source.shape().len() {
        return Err(Error::Rank);
    }
    if pattern.shape().len() > 1 || source.shape().len() > 1 {
        return Err(Error::Unsupported("E. multidimensional pattern".into()));
    }
    Ok(ReferenceExpansionValue::WindowFamily { pattern, source })
}

fn match_window_family(pattern: &Value, source: &Value) -> crate::Result<Value> {
    let width = pattern.len();
    Value::new(
        source.shape().to_vec(),
        Data::Bool(crate::storage::CpuStorage::generate(source.len(), |i| {
            (width <= source.len() - i
                && (0..width).all(|k| crate::index_ops::atom_eq(pattern, k, source, i + k)))
                as u8
        })?),
    )
}

/// Execute a witnessed expansion through a small reference realization.
///
/// This is deliberately separate from ordinary A3 basis execution: expansion
/// nodes may carry virtual descriptors (WindowFamily) rather than materialized
/// J values. It exists to validate/host a composite lowering family without
/// pretending every basis node already has a standalone native kernel.
pub fn execute_reference(
    expansion: &ExecutionBasisExpansion,
    source_values: &[Option<Value>],
) -> crate::Result<Value> {
    expansion
        .graph
        .verify()
        .map_err(|message| Error::Unsupported(message.into()))?;

    let mut local = Vec::with_capacity(expansion.graph.nodes.len());
    for node in &expansion.graph.nodes {
        let value = match (node.semantics, node.inputs.as_slice()) {
            (
                ExpansionNodeSemantics::WindowByPatternShape,
                [
                    ExpansionInput::Source(source),
                    ExpansionInput::Source(pattern),
                ],
            ) => find_window_family(
                source_value(source_values, *pattern)?,
                source_value(source_values, *source)?,
            )?,
            (
                ExpansionNodeSemantics::MatchPatternCell,
                [ExpansionInput::Source(_), ExpansionInput::Node(window)],
            ) => {
                let Some(ReferenceExpansionValue::WindowFamily { pattern, source }) =
                    local.get(window.0)
                else {
                    return Err(Error::Unsupported(
                        "reference expansion match input is not a window family".into(),
                    ));
                };
                ReferenceExpansionValue::Value(match_window_family(pattern, source)?)
            }
            _ => {
                return Err(Error::Unsupported(
                    "reference expansion does not implement this node".into(),
                ));
            }
        };
        local.push(value);
    }

    match local.get(expansion.graph.output.0) {
        Some(ReferenceExpansionValue::Value(value)) => Ok(value.clone()),
        Some(ReferenceExpansionValue::WindowFamily { .. }) => Err(Error::Unsupported(
            "reference expansion cannot return a virtual window family".into(),
        )),
        None => Err(Error::Unsupported(
            "reference expansion output is unavailable".into(),
        )),
    }
}

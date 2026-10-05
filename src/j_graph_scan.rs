//! Conservative Scan identity candidates, preserving the original J graph.
//! This does not implement a prefix executor or permit reassociation/scheduling.

use crate::{
    facts::TypeFact,
    j_graph_ir::{GraphBasis, GraphBasisKind, GraphFacts, GraphForm, NodeKind, Plan, ValueId},
    primitive::PrimitiveId,
    semantic::FunctionHead,
    types::DType,
};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanBoundaryReason {
    DyadicInfix,
    NonPrimitiveInsert,
    UnknownOrNonBooleanInput,
    UnknownOrInconsistentShape,
    UnprovenNumericOrErrorSemantics,
    ExtentOverflow,
    NestedCallRequiresRankFacts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanBoundary {
    pub source: ValueId,
    pub reason: ScanBoundaryReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanWitness {
    /// Exact Bool inputs and known extents bound every integer sum by the
    /// number of leading items; Bool multiplication stays in {0,1}.
    /// Atomic reducers have no name lookup, user effects or domain failures.
    ExactBooleanAtomicPrefixV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanContract {
    pub reducer: PrimitiveId,
    pub input_facts: GraphFacts,
    pub output_facts: GraphFacts,
    /// Prefixes grow from length 1 through n along the leading item axis.
    /// Rank-zero input is one item and produces a vector of length one.
    pub items: usize,
    pub atoms: usize,
    /// C ap.c::jtpscan returns the input on empty/single-item paths (reshaping
    /// scalars). In particular a singleton Boolean sum stays Boolean.
    pub returns_input_atoms: bool,
    /// No artificial identity element is inserted, including on empty input.
    pub inject_identity: bool,
    /// Candidate identity is distinct from permission to reorder a J call.
    pub preserves_source_order: bool,
    pub parallel_prefix_authorized: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanCandidate {
    pub source: ValueId,
    pub input: ValueId,
    pub source_span: Range<usize>,
    pub basis: GraphBasis,
    pub contract: ScanContract,
    pub witness: ScanWitness,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanAnalysis {
    pub candidates: Vec<ScanCandidate>,
    /// Analysis coverage boundaries, never J language errors or execution
    /// conformance passes. Source Window -> operand remains intact.
    pub boundaries: Vec<ScanBoundary>,
}

impl ScanAnalysis {
    pub fn from_plan(plan: &Plan) -> Result<Self, String> {
        plan.verify()?;
        Ok(Self::derive(plan))
    }

    pub fn verify(&self, plan: &Plan) -> Result<(), String> {
        plan.verify()?;
        if *self != Self::derive(plan) {
            return Err("Scan identity witness does not match verified J graph".into());
        }
        Ok(())
    }

    fn derive(plan: &Plan) -> Self {
        let mut analysis = Self {
            candidates: Vec::new(),
            boundaries: Vec::new(),
        };
        for (index, node) in plan.nodes.iter().enumerate() {
            let NodeKind::Apply {
                form,
                basis,
                left,
                right,
                ..
            } = &node.kind
            else {
                continue;
            };
            let source = ValueId(index);
            let GraphForm::PrefixInfix { operand } = form else {
                if basis.layers.contains(&GraphBasisKind::Window) {
                    analysis.boundaries.push(ScanBoundary {
                        source,
                        reason: ScanBoundaryReason::NestedCallRequiresRankFacts,
                    });
                }
                continue;
            };
            let recognized = (|| {
                use ScanBoundaryReason::*;
                if left.is_some() {
                    return Err(DyadicInfix);
                }
                let (inner, _) = crate::j_graph_ir::classify_function(operand);
                let GraphForm::Reduce { operand: reducer } = inner else {
                    return Err(NonPrimitiveInsert);
                };
                let FunctionHead::PrimitiveVerb(id) = reducer.head else {
                    return Err(NonPrimitiveInsert);
                };
                if !reducer.operands.is_empty() {
                    return Err(NonPrimitiveInsert);
                }
                if !matches!(id, PrimitiveId::Add | PrimitiveId::Multiply) {
                    return Err(UnprovenNumericOrErrorSemantics);
                }
                let facts = &plan.nodes[right.0].facts;
                if facts.dtype != TypeFact::Exact(DType::Bool) {
                    return Err(UnknownOrNonBooleanInput);
                }
                let shape = facts.shape.as_ref().ok_or(UnknownOrInconsistentShape)?;
                if facts.rank != Some(shape.len()) {
                    return Err(UnknownOrInconsistentShape);
                }
                let atoms = shape
                    .iter()
                    .try_fold(1usize, |n, d| n.checked_mul(*d))
                    .ok_or(ExtentOverflow)?;
                let items = shape.first().copied().unwrap_or(1);
                if items > i64::MAX as usize {
                    return Err(ExtentOverflow);
                }
                let returns_input_atoms = items < 2 || atoms == 0;
                let output_shape = if shape.is_empty() {
                    vec![1]
                } else {
                    shape.clone()
                };
                let output_dtype = if id == PrimitiveId::Add && !returns_input_atoms {
                    DType::Int
                } else {
                    DType::Bool
                };
                Ok(ScanCandidate {
                    source,
                    input: *right,
                    source_span: node.span.clone(),
                    basis: GraphBasis {
                        layers: vec![GraphBasisKind::Scan],
                    },
                    contract: ScanContract {
                        reducer: id,
                        input_facts: facts.clone(),
                        output_facts: GraphFacts {
                            dtype: TypeFact::Exact(output_dtype),
                            rank: Some(output_shape.len()),
                            shape: Some(output_shape),
                        },
                        items,
                        atoms,
                        returns_input_atoms,
                        inject_identity: false,
                        preserves_source_order: true,
                        parallel_prefix_authorized: false,
                    },
                    witness: ScanWitness::ExactBooleanAtomicPrefixV1,
                })
            })();
            match recognized {
                Ok(candidate) => analysis.candidates.push(candidate),
                Err(reason) => analysis.boundaries.push(ScanBoundary { source, reason }),
            }
        }
        analysis
    }
}

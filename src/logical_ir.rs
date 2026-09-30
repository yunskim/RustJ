//! A3-v0 single-block logical IR.
//!
//! This module deliberately separates operations from SSA values.  It is built
//! from the existing inspection plan during migration; it does not execute
//! kernels and does not encode physical scheduling decisions.

use crate::{
    Value,
    analysis::{
        self, AccessFact, BasisKind, CallTarget, Callable, ResolvedInstantiation, Symbol,
        SymbolId,
    },
    contracts::{Contract, Valence},
    facts::{Facts, RankPlan, ValueRoleFacts},
    semantic::NameVersion,
};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OpId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticErrorKind {
    Domain,
    Length,
    Rank,
    Index,
    Limit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Constraint {
    PrefixAgreement {
        left: ValueId,
        right: ValueId,
    },
    CellFrameAgreement {
        left: ValueId,
        right: ValueId,
    },
    IndicesInBounds {
        indices: ValueId,
        source: ValueId,
    },
}

impl Constraint {
    fn values(&self) -> [Option<ValueId>; 2] {
        match *self {
            Self::PrefixAgreement { left, right }
            | Self::CellFrameAgreement { left, right } => [Some(left), Some(right)],
            Self::IndicesInBounds { indices, source } => [Some(indices), Some(source)],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FactWitness {
    PrefixAgreement {
        left_shape: Vec<usize>,
        right_shape: Vec<usize>,
    },
    CellFrameAgreement {
        result_frame: Vec<usize>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstraintFact {
    pub constraint: Constraint,
    pub witness: Option<FactWitness>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConstraintSet {
    pub facts: Vec<ConstraintFact>,
}

impl ConstraintSet {
    pub fn unresolved(&self) -> impl Iterator<Item = &ConstraintFact> {
        self.facts.iter().filter(|fact| fact.witness.is_none())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticCheck {
    pub constraint: Constraint,
    pub error: SemanticErrorKind,
    pub origin: Range<usize>,
}

#[derive(Clone, Debug)]
pub struct CallOp {
    pub callable: Callable,
    pub left: Option<ValueId>,
    pub right: ValueId,
    pub contract: Contract,
    pub instantiation: ResolvedInstantiation,
    pub rank_plan: Option<RankPlan>,
    pub access: AccessFact,
    pub constraints: ConstraintSet,
}

#[derive(Clone, Debug)]
pub enum OpKind {
    Literal(Value),
    ReadNoun {
        symbol: SymbolId,
        version: NameVersion,
    },
    VerbReference(Callable),
    Basis {
        kind: BasisKind,
        call: CallOp,
    },
    /// Correctness-preserving fallback for calls not normalized to a basis op.
    SemanticCall(CallOp),
    /// A J-visible precondition check.  It intentionally has no SSA result.
    SemanticCheck(SemanticCheck),
}

#[derive(Clone, Debug)]
pub struct Operation {
    pub kind: OpKind,
    pub results: Vec<ValueId>,
    pub span: Range<usize>,
    /// Explicit observable ordering dependency.  Data dependencies are checked
    /// separately through value operands.
    pub order_after: Option<OpId>,
}

#[derive(Clone, Debug)]
pub struct ValueData {
    pub producer: OpId,
    pub facts: Facts,
    pub roles: ValueRoleFacts,
}

#[derive(Clone, Debug)]
pub struct Write {
    pub symbol: SymbolId,
    pub value: ValueId,
    pub previous: Option<NameVersion>,
    pub proposed: NameVersion,
    pub span: Range<usize>,
    pub after: Option<OpId>,
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub source: String,
    pub symbols: Vec<Symbol>,
    pub operations: Vec<Operation>,
    pub values: Vec<ValueData>,
    pub result: Option<ValueId>,
    pub write: Option<Write>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyError {
    pub operation: Option<OpId>,
    pub message: String,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(operation) = self.operation {
            write!(
                f,
                "A3 logical IR verification failed at op {}: {}",
                operation.0, self.message
            )
        } else {
            write!(f, "A3 logical IR verification failed: {}", self.message)
        }
    }
}

impl std::error::Error for VerifyError {}

fn prefix_agrees(left: &[usize], right: &[usize]) -> bool {
    let (short, long) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    long.starts_with(short)
}

fn map_value(id: analysis::ValueId, values: &[ValueId]) -> ValueId {
    values[id.0]
}

fn call_constraints(
    node: &analysis::Node,
    left: Option<ValueId>,
    right: ValueId,
    transition: &analysis::LogicalPlan,
) -> ConstraintSet {
    let mut set = ConstraintSet::default();

    if let Some(left_value) = left {
        if node.basis == Some(BasisKind::CellApply) {
            let witness = node.rank_plan.as_ref().and_then(|plan| {
                plan.result_frame
                    .clone()
                    .map(|result_frame| FactWitness::CellFrameAgreement { result_frame })
            });
            set.facts.push(ConstraintFact {
                constraint: Constraint::CellFrameAgreement {
                    left: left_value,
                    right,
                },
                witness,
            });
        } else if node.basis == Some(BasisKind::Elementwise) {
            let analysis::Operation::Call {
                left: Some(old_left),
                right: old_right,
                ..
            } = &node.operation
            else {
                unreachable!("dyadic elementwise call must have two operands")
            };
            let left_shape = transition.nodes[old_left.0].facts.shape.clone();
            let right_shape = transition.nodes[old_right.0].facts.shape.clone();
            let witness = left_shape.zip(right_shape).and_then(|(left_shape, right_shape)| {
                prefix_agrees(&left_shape, &right_shape).then_some(
                    FactWitness::PrefixAgreement {
                        left_shape,
                        right_shape,
                    },
                )
            });
            set.facts.push(ConstraintFact {
                constraint: Constraint::PrefixAgreement {
                    left: left_value,
                    right,
                },
                witness,
            });
        }
    }

    if node.basis == Some(BasisKind::Gather) {
        if let Some(indices) = left {
            set.facts.push(ConstraintFact {
                constraint: Constraint::IndicesInBounds {
                    indices,
                    source: right,
                },
                witness: None,
            });
        }
    }

    set
}

fn error_for(constraint: &Constraint) -> SemanticErrorKind {
    match constraint {
        Constraint::PrefixAgreement { .. } | Constraint::CellFrameAgreement { .. } => {
            SemanticErrorKind::Length
        }
        Constraint::IndicesInBounds { .. } => SemanticErrorKind::Index,
    }
}

impl Plan {
    /// Convert the current inspection plan into the A3-v0 op/value-separated
    /// single-block representation.  The transition plan remains available as
    /// the compatibility API while migration proceeds.
    pub fn from_transition(transition: &analysis::LogicalPlan) -> Self {
        let mut plan = Self {
            source: transition.source.clone(),
            symbols: transition.symbols.clone(),
            operations: Vec::new(),
            values: Vec::new(),
            result: None,
            write: None,
        };
        let mut value_map = Vec::with_capacity(transition.nodes.len());
        let mut producer_map = Vec::with_capacity(transition.nodes.len());

        for node in &transition.nodes {
            let old_order = node
                .order_after
                .and_then(|value| producer_map.get(value.0).copied());

            let (base_kind, constraints) = match &node.operation {
                analysis::Operation::Literal(value) => {
                    (OpKind::Literal(value.clone()), ConstraintSet::default())
                }
                analysis::Operation::ReadNoun { symbol, version } => (
                    OpKind::ReadNoun {
                        symbol: *symbol,
                        version: *version,
                    },
                    ConstraintSet::default(),
                ),
                analysis::Operation::VerbReference(callable) => (
                    OpKind::VerbReference(callable.clone()),
                    ConstraintSet::default(),
                ),
                analysis::Operation::Call {
                    callable,
                    left,
                    right,
                    contract,
                } => {
                    let left = left.map(|value| map_value(value, &value_map));
                    let right = map_value(*right, &value_map);
                    let constraints = call_constraints(node, left, right, transition);
                    let instantiation = node
                        .instantiation
                        .clone()
                        .expect("verified transition call must have instantiation");
                    let call = CallOp {
                        callable: callable.clone(),
                        left,
                        right,
                        contract: *contract,
                        instantiation,
                        rank_plan: node.rank_plan.clone(),
                        access: node.access,
                        constraints: constraints.clone(),
                    };
                    let kind = match node.basis {
                        Some(kind) => OpKind::Basis { kind, call },
                        None => OpKind::SemanticCall(call),
                    };
                    (kind, constraints)
                }
            };

            let mut order_after = old_order;
            for fact in constraints.unresolved() {
                let check_id = OpId(plan.operations.len());
                plan.operations.push(Operation {
                    kind: OpKind::SemanticCheck(SemanticCheck {
                        constraint: fact.constraint.clone(),
                        error: error_for(&fact.constraint),
                        origin: node.span.clone(),
                    }),
                    results: Vec::new(),
                    span: node.span.clone(),
                    order_after,
                });
                order_after = Some(check_id);
            }

            let op_id = OpId(plan.operations.len());
            let value_id = ValueId(plan.values.len());
            plan.values.push(ValueData {
                producer: op_id,
                facts: node.facts.clone(),
                roles: node.roles.clone(),
            });
            plan.operations.push(Operation {
                kind: base_kind,
                results: vec![value_id],
                span: node.span.clone(),
                order_after,
            });
            value_map.push(value_id);
            producer_map.push(op_id);
        }

        plan.result = transition.result.map(|value| value_map[value.0]);
        plan.write = transition.write.as_ref().map(|write| Write {
            symbol: write.symbol,
            value: value_map[write.value.0],
            previous: write.previous,
            proposed: write.proposed,
            span: write.span.clone(),
            after: write
                .after
                .and_then(|value| producer_map.get(value.0).copied()),
        });
        plan
    }

    pub fn verify(&self) -> std::result::Result<(), VerifyError> {
        let fail = |operation: Option<OpId>, message: String| VerifyError {
            operation,
            message,
        };
        let source_len = self.source.len();

        for (index, operation) in self.operations.iter().enumerate() {
            let op_id = OpId(index);
            if operation.span.start > operation.span.end
                || operation.span.end > source_len
                || !self.source.is_char_boundary(operation.span.start)
                || !self.source.is_char_boundary(operation.span.end)
            {
                return Err(fail(Some(op_id), "invalid source span".into()));
            }
            if let Some(before) = operation.order_after {
                if before.0 >= index {
                    return Err(fail(
                        Some(op_id),
                        "order edge must reference an earlier operation".into(),
                    ));
                }
            }

            let check_value = |value: ValueId, label: &str| {
                let Some(data) = self.values.get(value.0) else {
                    return Err(fail(Some(op_id), format!("{label} value is out of bounds")));
                };
                if data.producer.0 >= index {
                    return Err(fail(
                        Some(op_id),
                        format!("{label} must be produced by an earlier operation"),
                    ));
                }
                Ok(())
            };

            match &operation.kind {
                OpKind::Literal(_) | OpKind::ReadNoun { .. } | OpKind::VerbReference(_) => {}
                OpKind::Basis { call, .. } | OpKind::SemanticCall(call) => {
                    if let Some(left) = call.left {
                        check_value(left, "left input")?;
                    }
                    check_value(call.right, "right input")?;
                    let expected_valence = if call.left.is_some() {
                        Valence::Dyad
                    } else {
                        Valence::Monad
                    };
                    if call.instantiation.valence != expected_valence {
                        return Err(fail(
                            Some(op_id),
                            "call instantiation valence mismatch".into(),
                        ));
                    }
                    if call.instantiation.target != call.callable.target {
                        return Err(fail(
                            Some(op_id),
                            "call instantiation target mismatch".into(),
                        ));
                    }
                    for fact in &call.constraints.facts {
                        for value in fact.constraint.values().into_iter().flatten() {
                            check_value(value, "constraint input")?;
                        }
                    }
                }
                OpKind::SemanticCheck(check) => {
                    if !operation.results.is_empty() {
                        return Err(fail(
                            Some(op_id),
                            "SemanticCheck must not produce SSA values".into(),
                        ));
                    }
                    for value in check.constraint.values().into_iter().flatten() {
                        check_value(value, "check input")?;
                    }
                }
            }

            for result in &operation.results {
                let Some(data) = self.values.get(result.0) else {
                    return Err(fail(Some(op_id), "result value is out of bounds".into()));
                };
                if data.producer != op_id {
                    return Err(fail(
                        Some(op_id),
                        "result value producer does not match operation".into(),
                    ));
                }
            }
        }

        for (index, value) in self.values.iter().enumerate() {
            let Some(operation) = self.operations.get(value.producer.0) else {
                return Err(fail(None, format!("value {index} producer is out of bounds")));
            };
            if !operation.results.contains(&ValueId(index)) {
                return Err(fail(
                    Some(value.producer),
                    format!("producer does not list value {index} as a result"),
                ));
            }
            if let Some(rank) = value.facts.rank {
                if let Some(shape) = &value.facts.shape {
                    if rank != shape.len() {
                        return Err(fail(
                            Some(value.producer),
                            format!("value {index} rank does not match shape"),
                        ));
                    }
                }
            }
        }

        if let Some(result) = self.result {
            if result.0 >= self.values.len() {
                return Err(fail(None, "plan result is out of bounds".into()));
            }
        }
        if let Some(write) = &self.write {
            if write.symbol.0 >= self.symbols.len() {
                return Err(fail(None, "write symbol is out of bounds".into()));
            }
            if write.value.0 >= self.values.len() {
                return Err(fail(None, "write value is out of bounds".into()));
            }
            if let Some(after) = write.after {
                if after.0 >= self.operations.len() {
                    return Err(fail(None, "write order dependency is out of bounds".into()));
                }
            }
        }

        Ok(())
    }
}

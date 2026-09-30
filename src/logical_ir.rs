//! A3-v0 single-block logical IR.
//!
//! This module deliberately separates operations from SSA values.  It is built
//! from the existing inspection plan during migration; it does not execute
//! kernels and does not encode physical scheduling decisions.

use crate::{
    Value,
    analysis::{
        self, AccessFact, BasisKind, Callable, ResolvedInstantiation, Symbol,
        SymbolId,
    },
    contracts::{Contract, Effect, Valence},
    facts::{Facts, RankPlan, ValueRoleFacts},
    semantic::NameVersion,
};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IrSchemaVersion {
    pub major: u16,
    pub minor: u16,
}

pub const A3_SCHEMA_VERSION: IrSchemaVersion = IrSchemaVersion { major: 0, minor: 1 };

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrProvenance {
    pub compiler_version: String,
    pub primitive_registry_version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrHeader {
    pub schema: IrSchemaVersion,
    pub provenance: IrProvenance,
}

impl IrHeader {
    fn current() -> Self {
        Self {
            schema: A3_SCHEMA_VERSION,
            provenance: IrProvenance {
                compiler_version: env!("CARGO_PKG_VERSION").to_owned(),
                primitive_registry_version: crate::primitive::REGISTRY_VERSION,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OpId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FunctionId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticErrorKind {
    Domain,
    Length,
    Rank,
    Index,
    Limit,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PossibleErrors {
    pub known: Vec<SemanticErrorKind>,
    pub unknown: bool,
}

impl PossibleErrors {
    fn from_contract(contract: Contract) -> Self {
        Self {
            known: Vec::new(),
            unknown: contract.may_error,
        }
    }

    pub fn may_raise(&self) -> bool {
        self.unknown || !self.known.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestinationRelation {
    FreshResult,
    MayReuse(ValueId),
    EquivalentView(ValueId),
    Unknown,
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
        left_rank: i64,
        right_rank: i64,
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
            | Self::CellFrameAgreement {
                left,
                right,
                ..
            } => [Some(left), Some(right)],
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IterationAxisKind {
    Parallel,
    Reduction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisRole {
    Output,
    Frame,
    Reduction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IterationAxis {
    pub position: usize,
    pub extent: Option<usize>,
    pub kind: IterationAxisKind,
    pub role: AxisRole,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IterationDomain {
    pub axes: Vec<IterationAxis>,
}

fn axes_from_shape(
    shape: Option<&[usize]>,
    rank: Option<usize>,
    role: AxisRole,
) -> Vec<IterationAxis> {
    let rank = shape.map_or(rank.unwrap_or(0), |shape| shape.len());
    (0..rank)
        .map(|position| IterationAxis {
            position,
            extent: shape.and_then(|shape| shape.get(position).copied()),
            kind: IterationAxisKind::Parallel,
            role,
        })
        .collect()
}

fn iteration_domain(
    kind: Option<BasisKind>,
    node: &analysis::Node,
    transition: &analysis::LogicalPlan,
) -> IterationDomain {
    if kind == Some(BasisKind::Reduce) {
        let analysis::Operation::Call { right, .. } = &node.operation else {
            return IterationDomain::default();
        };
        let input = &transition.nodes[right.0].facts;
        let rank = input.shape.as_ref().map_or(input.rank.unwrap_or(0), Vec::len);
        let mut axes = Vec::with_capacity(rank);
        for position in 0..rank {
            axes.push(IterationAxis {
                position,
                extent: input
                    .shape
                    .as_ref()
                    .and_then(|shape| shape.get(position).copied()),
                kind: if position == 0 {
                    IterationAxisKind::Reduction
                } else {
                    IterationAxisKind::Parallel
                },
                role: if position == 0 {
                    AxisRole::Reduction
                } else {
                    AxisRole::Output
                },
            });
        }
        return IterationDomain { axes };
    }

    if kind == Some(BasisKind::CellApply) {
        if let Some(plan) = &node.rank_plan {
            let frame = plan.result_frame.as_deref();
            return IterationDomain {
                axes: axes_from_shape(frame, frame.map(|shape| shape.len()), AxisRole::Frame),
            };
        }
    }

    IterationDomain {
        axes: axes_from_shape(
            node.facts.shape.as_deref(),
            node.facts.rank,
            AxisRole::Output,
        ),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectSummary {
    Pure,
    Unknown,
}

impl EffectSummary {
    pub fn from_contract(contract: Contract) -> Self {
        match contract.effect {
            Effect::Pure => Self::Pure,
            Effect::Unknown => Self::Unknown,
        }
    }

    pub fn is_pure(self) -> bool {
        self == Self::Pure
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeculationSemantics {
    pub may_raise_observable_error: bool,
    pub preserve_evaluation_order: bool,
}

impl SpeculationSemantics {
    pub fn from_contract(contract: Contract) -> Self {
        Self {
            may_raise_observable_error: contract.may_error,
            preserve_evaluation_order: contract.preserve_evaluation_order,
        }
    }

    pub fn freely_speculatable(self) -> bool {
        !self.may_raise_observable_error && !self.preserve_evaluation_order
    }
}

#[derive(Clone, Debug)]
pub struct CallOp {
    pub callable: Callable,
    pub left: Option<ValueId>,
    pub right: ValueId,
    pub contract: Contract,
    pub iteration_domain: IterationDomain,
    pub effect: EffectSummary,
    pub speculation: SpeculationSemantics,
    pub possible_errors: PossibleErrors,
    pub destination: DestinationRelation,
    pub instantiation: ResolvedInstantiation,
    pub rank_plan: Option<RankPlan>,
    pub access: AccessFact,
    pub constraints: ConstraintSet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReindexKind {
    Reshape,
    Ravel,
    Reverse,
    Transpose,
    Take,
    Drop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReductionAxis {
    LeadingCellAxis,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BasisPayload {
    IndexSpace {
        shape_spec: ValueId,
    },
    Elementwise,
    CellApply,
    StaticReindex {
        kind: ReindexKind,
    },
    Gather {
        indices: ValueId,
        source: ValueId,
    },
    Reduce {
        axis: ReductionAxis,
    },
    ConcatAssemble,
    ReplicateCompactExpand,
    LookupClassify,
    /// Later basis families can retain their identity before their richer
    /// family-specific payload is implemented.
    Deferred,
}

fn basis_payload(kind: BasisKind, call: &CallOp) -> BasisPayload {
    use crate::primitive::PrimitiveId;

    match kind {
        BasisKind::IndexSpace => BasisPayload::IndexSpace {
            shape_spec: call.right,
        },
        BasisKind::Elementwise => BasisPayload::Elementwise,
        BasisKind::CellApply => BasisPayload::CellApply,
        BasisKind::Reduce => BasisPayload::Reduce {
            axis: ReductionAxis::LeadingCellAxis,
        },
        BasisKind::StaticReindex => {
            let reindex = match call.callable.target {
                analysis::CallTarget::Primitive(PrimitiveId::Shape) => ReindexKind::Reshape,
                analysis::CallTarget::Primitive(PrimitiveId::Ravel) => ReindexKind::Ravel,
                analysis::CallTarget::Primitive(PrimitiveId::Reverse) => ReindexKind::Reverse,
                analysis::CallTarget::Primitive(PrimitiveId::Transpose) => {
                    ReindexKind::Transpose
                }
                analysis::CallTarget::Primitive(PrimitiveId::Take) => ReindexKind::Take,
                analysis::CallTarget::Primitive(PrimitiveId::Drop) => ReindexKind::Drop,
                _ => return BasisPayload::Deferred,
            };
            BasisPayload::StaticReindex { kind: reindex }
        }
        BasisKind::Gather => {
            let Some(indices) = call.left else {
                return BasisPayload::Deferred;
            };
            BasisPayload::Gather {
                indices,
                source: call.right,
            }
        }
        BasisKind::ConcatAssemble => BasisPayload::ConcatAssemble,
        BasisKind::ReplicateCompactExpand => BasisPayload::ReplicateCompactExpand,
        BasisKind::LookupClassify => BasisPayload::LookupClassify,
        _ => BasisPayload::Deferred,
    }
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
        payload: BasisPayload,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Terminator {
    Return(Option<ValueId>),
}

#[derive(Clone, Debug)]
pub struct Block {
    pub operations: Range<usize>,
    pub terminator: Terminator,
}

#[derive(Clone, Debug)]
pub struct Region {
    pub blocks: Vec<BlockId>,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub body: RegionId,
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub header: IrHeader,
    pub source: String,
    pub symbols: Vec<Symbol>,
    pub functions: Vec<Function>,
    pub regions: Vec<Region>,
    pub blocks: Vec<Block>,
    pub entry: FunctionId,
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
            let ranks = match &node.operation {
                analysis::Operation::Call { callable, .. } => callable.rank.unwrap_or([0, 0, 0]),
                _ => [0, 0, 0],
            };
            set.facts.push(ConstraintFact {
                constraint: Constraint::CellFrameAgreement {
                    left: left_value,
                    right,
                    left_rank: ranks[1],
                    right_rank: ranks[2],
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
            header: IrHeader::current(),
            source: transition.source.clone(),
            symbols: transition.symbols.clone(),
            functions: Vec::new(),
            regions: Vec::new(),
            blocks: Vec::new(),
            entry: FunctionId(0),
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
                        iteration_domain: iteration_domain(node.basis, node, transition),
                        effect: EffectSummary::from_contract(*contract),
                        speculation: SpeculationSemantics::from_contract(*contract),
                        possible_errors: PossibleErrors::from_contract(*contract),
                        destination: DestinationRelation::Unknown,
                        instantiation,
                        rank_plan: node.rank_plan.clone(),
                        access: node.access,
                        constraints: constraints.clone(),
                    };
                    let kind = match node.basis {
                        Some(kind) => {
                            let payload = basis_payload(kind, &call);
                            OpKind::Basis {
                                kind,
                                payload,
                                call,
                            }
                        }
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

        let block = BlockId(0);
        let region = RegionId(0);
        plan.blocks.push(Block {
            operations: 0..plan.operations.len(),
            terminator: Terminator::Return(plan.result),
        });
        plan.regions.push(Region {
            blocks: vec![block],
        });
        plan.functions.push(Function { body: region });
        plan
    }

    pub fn verify(&self) -> std::result::Result<(), VerifyError> {
        let fail = |operation: Option<OpId>, message: String| VerifyError {
            operation,
            message,
        };
        let source_len = self.source.len();

        if self.header.schema != A3_SCHEMA_VERSION {
            return Err(fail(None, "unsupported A3 IR schema version".into()));
        }
        if self.header.provenance.primitive_registry_version
            != crate::primitive::REGISTRY_VERSION
        {
            return Err(fail(
                None,
                "A3 IR primitive registry provenance does not match compiler".into(),
            ));
        }

        if self.functions.len() != 1 || self.regions.len() != 1 || self.blocks.len() != 1 {
            return Err(fail(
                None,
                "A3-v0 plan must contain exactly one function, region and block".into(),
            ));
        }
        let Some(entry) = self.functions.get(self.entry.0) else {
            return Err(fail(None, "entry function is out of bounds".into()));
        };
        let Some(region) = self.regions.get(entry.body.0) else {
            return Err(fail(None, "entry region is out of bounds".into()));
        };
        if region.blocks.len() != 1 {
            return Err(fail(
                None,
                "A3-v0 entry region must contain exactly one block".into(),
            ));
        }
        let block_id = region.blocks[0];
        let Some(block) = self.blocks.get(block_id.0) else {
            return Err(fail(None, "entry block is out of bounds".into()));
        };
        if block.operations != (0..self.operations.len()) {
            return Err(fail(
                None,
                "A3-v0 entry block must cover the complete operation sequence".into(),
            ));
        }
        if block.terminator != Terminator::Return(self.result) {
            return Err(fail(
                None,
                "entry block return does not match plan result".into(),
            ));
        }

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
                OpKind::Basis {
                    kind,
                    payload,
                    call,
                } => {
                    if *payload != basis_payload(*kind, call) {
                        return Err(fail(
                            Some(op_id),
                            "basis payload does not match basis identity/call".into(),
                        ));
                    }
                    for (expected, axis) in call.iteration_domain.axes.iter().enumerate() {
                        if axis.position != expected {
                            return Err(fail(
                                Some(op_id),
                                "iteration-domain axis positions must be dense and ordered".into(),
                            ));
                        }
                        if axis.kind == IterationAxisKind::Reduction
                            && axis.role != AxisRole::Reduction
                        {
                            return Err(fail(
                                Some(op_id),
                                "reduction iteration axis must have reduction role".into(),
                            ));
                        }
                    }
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
                OpKind::SemanticCall(call) => {
                    for (expected, axis) in call.iteration_domain.axes.iter().enumerate() {
                        if axis.position != expected {
                            return Err(fail(
                                Some(op_id),
                                "iteration-domain axis positions must be dense and ordered".into(),
                            ));
                        }
                    }
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

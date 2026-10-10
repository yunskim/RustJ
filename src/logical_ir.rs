//! A3-v0 single-block logical IR.
//!
//! This module deliberately separates operations from SSA values. Canonical A3
//! is constructed incrementally during J Graph execution-semantic lowering; it
//! does not execute kernels and does not encode physical scheduling decisions.

use crate::{
    Value,
    contracts::{Contract, Effect, Valence},
    execution_semantics::{
        AccessFact, CallTarget, Callable, ExecutionBasis, ExecutionBasisKind,
        ResolvedInstantiation, Symbol, SymbolId,
    },
    facts::{Facts, RankPlan, ValueRoleFacts},
    opportunity::{StructuralOpportunity, StructuralTopology},
    semantic::{FunctionHead, FunctionOperand, NameVersion},
};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IrSchemaVersion {
    pub major: u16,
    pub minor: u16,
}

pub const A3_SCHEMA_VERSION: IrSchemaVersion = IrSchemaVersion { major: 0, minor: 6 };

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
    pub(crate) fn values(&self) -> [Option<ValueId>; 2] {
        match *self {
            Self::PrefixAgreement { left, right }
            | Self::CellFrameAgreement { left, right, .. } => [Some(left), Some(right)],
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
    /// Logical output/frame iteration axis, NOT proof that iterations can run
    /// concurrently. J effects, errors, aliasing, rank fill and result-cell
    /// assembly require independent legality witnesses before any CPU/GPU map.
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
    kind: Option<ExecutionBasisKind>,
    result_facts: &Facts,
    rank_plan: Option<&RankPlan>,
    right_input_facts: &Facts,
) -> IterationDomain {
    if kind == Some(ExecutionBasisKind::Reduce) {
        let rank = right_input_facts
            .shape
            .as_ref()
            .map_or(right_input_facts.rank.unwrap_or(0), Vec::len);
        let mut axes = Vec::with_capacity(rank);
        for position in 0..rank {
            axes.push(IterationAxis {
                position,
                extent: right_input_facts
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

    if kind == Some(ExecutionBasisKind::CellApply) {
        if let Some(plan) = rank_plan {
            let frame = plan.result_frame.as_deref();
            return IterationDomain {
                axes: axes_from_shape(frame, frame.map(|shape| shape.len()), AxisRole::Frame),
            };
        }
    }

    IterationDomain {
        axes: axes_from_shape(
            result_facts.shape.as_deref(),
            result_facts.rank,
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

fn semantic_effect_summary(
    function: &crate::semantic::FunctionEntity,
    valence: Valence,
) -> EffectSummary {
    match &function.head {
        FunctionHead::PrimitiveVerb(id) => {
            EffectSummary::from_contract(crate::contracts::for_primitive(*id, valence))
        }
        FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert) => {
            let Some(FunctionOperand::Function(operand)) = function.operands.first() else {
                return EffectSummary::Unknown;
            };
            // u/ applies u dyadically between items.
            semantic_effect_summary(operand, Valence::Dyad)
        }
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank) => {
            let Some(FunctionOperand::Function(operand)) = function.operands.first() else {
                return EffectSummary::Unknown;
            };
            semantic_effect_summary(operand, valence)
        }
        FunctionHead::VocabularyPrimitive(_)
        | FunctionHead::NameRef(_)
        | FunctionHead::TakeName { .. }
        | FunctionHead::PrimitiveAdverb(_)
        | FunctionHead::PrimitiveConjunction(_)
        | FunctionHead::DefinitionConstructor(_)
        | FunctionHead::ExplicitDefinition(_)
        | FunctionHead::ModifierTrain
        | FunctionHead::Hook
        | FunctionHead::Fork => EffectSummary::Unknown,
    }
}

fn resolved_effect_summary(
    callable: &Callable,
    left: Option<ValueId>,
    contract: Contract,
) -> EffectSummary {
    if contract.effect == Effect::Pure {
        return EffectSummary::Pure;
    }

    semantic_effect_summary(
        &callable.semantic,
        if left.is_some() {
            Valence::Dyad
        } else {
            Valence::Monad
        },
    )
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
    /// Full outer-to-inner execution-basis composition retained from semantic
    /// lowering. OpKind::Basis uses the outer layer as its current routing key.
    pub execution_basis: ExecutionBasis,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowShapeSpec {
    /// The logical window shape is the shape of another source value.
    PatternShape { pattern: ValueId },
}

/// J-observable index/search result, independent of hash representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchOutputKind {
    FirstIndex,
    LastIndex,
    MembershipMask,
    IntervalIndex,
    SelfClassify,
}

/// The primitive's comparison meaning, not permission to treat floats as
/// exact hash keys. Any !. override/runtime cct remains a semantic input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchComparison {
    JEquality,
    JOrderedInterval,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchDescriptor {
    /// None only for monadic self-classification, which has no search-domain operand.
    pub indexed: Option<ValueId>,
    pub queried: ValueId,
    pub output: SearchOutputKind,
    pub comparison: SearchComparison,
    /// Inherited semantic rank specification; not an inferred storage layout.
    pub rank_boundary: Option<[i64; 3]>,
}

/// Build the *meaning* of LookupClassify from resolved primitive identity and
/// valence. This is not an optimized implementation or a hash-legality proof.
pub fn search_descriptor(call: &CallOp) -> Option<SearchDescriptor> {
    use crate::primitive::PrimitiveId;
    let (output, comparison) = match (call.callable.target, call.left) {
        (CallTarget::Primitive(PrimitiveId::IndexOf), Some(_)) => {
            (SearchOutputKind::FirstIndex, SearchComparison::JEquality)
        }
        (CallTarget::Primitive(PrimitiveId::Steps), Some(_)) => {
            (SearchOutputKind::LastIndex, SearchComparison::JEquality)
        }
        (CallTarget::Primitive(PrimitiveId::Member), Some(_)) => (
            SearchOutputKind::MembershipMask,
            SearchComparison::JEquality,
        ),
        (CallTarget::Primitive(PrimitiveId::Indices), Some(_)) => (
            SearchOutputKind::IntervalIndex,
            SearchComparison::JOrderedInterval,
        ),
        (CallTarget::Primitive(PrimitiveId::Equal), None) => {
            (SearchOutputKind::SelfClassify, SearchComparison::JEquality)
        }
        _ => return None,
    };
    // Dyadic e. asks whether *left* items belong to the *right* set,
    // unlike i./i:/I. which search left using right-side queries.
    // A3 must preserve this direction before any physical hash strategy.
    let (indexed, queried) = if output == SearchOutputKind::MembershipMask {
        (Some(call.right), call.left?)
    } else {
        (call.left, call.right)
    };
    Some(SearchDescriptor {
        indexed,
        queried,
        output,
        comparison,
        rank_boundary: call.instantiation.rank_boundary,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionBasisPayload {
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
    WindowView {
        source: ValueId,
        shape: WindowShapeSpec,
    },
    Reduce {
        axis: ReductionAxis,
    },
    ConcatAssemble,
    ReplicateCompactExpand,
    LookupClassify {
        search: SearchDescriptor,
    },
    /// Later basis families can retain their identity before their richer
    /// family-specific payload is implemented.
    Deferred,
}

fn basis_payload(kind: ExecutionBasisKind, call: &CallOp) -> ExecutionBasisPayload {
    use crate::primitive::PrimitiveId;

    match kind {
        ExecutionBasisKind::IndexSpace => ExecutionBasisPayload::IndexSpace {
            shape_spec: call.right,
        },
        ExecutionBasisKind::Elementwise => ExecutionBasisPayload::Elementwise,
        ExecutionBasisKind::CellApply => ExecutionBasisPayload::CellApply,
        ExecutionBasisKind::Reduce => ExecutionBasisPayload::Reduce {
            axis: ReductionAxis::LeadingCellAxis,
        },
        ExecutionBasisKind::StaticReindex => {
            let reindex = match call.callable.target {
                CallTarget::Primitive(PrimitiveId::Shape) => ReindexKind::Reshape,
                CallTarget::Primitive(PrimitiveId::Ravel) => ReindexKind::Ravel,
                CallTarget::Primitive(PrimitiveId::Reverse) => ReindexKind::Reverse,
                CallTarget::Primitive(PrimitiveId::Transpose) => ReindexKind::Transpose,
                CallTarget::Primitive(PrimitiveId::Take) => ReindexKind::Take,
                CallTarget::Primitive(PrimitiveId::Drop) => ReindexKind::Drop,
                _ => return ExecutionBasisPayload::Deferred,
            };
            ExecutionBasisPayload::StaticReindex { kind: reindex }
        }
        ExecutionBasisKind::Gather => {
            let Some(indices) = call.left else {
                return ExecutionBasisPayload::Deferred;
            };
            ExecutionBasisPayload::Gather {
                indices,
                source: call.right,
            }
        }
        ExecutionBasisKind::ConcatAssemble => ExecutionBasisPayload::ConcatAssemble,
        ExecutionBasisKind::ReplicateCompactExpand => ExecutionBasisPayload::ReplicateCompactExpand,
        ExecutionBasisKind::LookupClassify => search_descriptor(call)
            .map(|search| ExecutionBasisPayload::LookupClassify { search })
            .unwrap_or(ExecutionBasisPayload::Deferred),
        _ => ExecutionBasisPayload::Deferred,
    }
}

#[derive(Clone, Debug)]
pub enum OpKind {
    /// Caller-supplied logical array, with no namespace or storage identity.
    Input {
        index: usize,
    },
    Literal(Value),
    ReadNoun {
        symbol: SymbolId,
        version: NameVersion,
    },
    VerbReference(Callable),
    Basis {
        kind: ExecutionBasisKind,
        payload: ExecutionBasisPayload,
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
    /// Applied J-graph node that gave rise to this execution op. Several
    /// execution ops may share one origin when a combinator is expanded.
    pub j_origin: Option<crate::j_graph_ir::ValueId>,
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
    pub parser_provenance: Option<crate::frontend_context::ParserProvenance>,
    pub header: IrHeader,
    pub source: String,
    pub symbols: Vec<Symbol>,
    pub functions: Vec<Function>,
    pub regions: Vec<Region>,
    pub blocks: Vec<Block>,
    pub entry: FunctionId,
    pub operations: Vec<Operation>,
    pub values: Vec<ValueData>,
    pub j_graph_node_count: usize,
    pub j_graph_region_count: usize,
    /// Structural topology discovered directly from J semantic syntax before
    /// flattening to generic SSA-like operations.
    pub opportunities: Vec<StructuralOpportunity<ValueId>>,
    pub result: Option<ValueId>,
    pub write: Option<Write>,
}

pub struct LogicalOpView<'a> {
    pub operation: &'a Operation,
    pub result: Option<&'a ValueData>,
}

pub trait SemanticCapabilityView {
    fn result_facts(&self) -> Option<&Facts>;
    fn iteration_domain(&self) -> Option<&IterationDomain>;
    fn access_fact(&self) -> Option<AccessFact>;
    fn effect_summary(&self) -> Option<EffectSummary>;
    fn speculation_semantics(&self) -> Option<SpeculationSemantics>;
    fn possible_errors(&self) -> PossibleErrors;
    fn destination_relation(&self) -> Option<DestinationRelation>;
}

fn call_from_kind(kind: &OpKind) -> Option<&CallOp> {
    match kind {
        OpKind::Basis { call, .. } | OpKind::SemanticCall(call) => Some(call),
        _ => None,
    }
}

impl SemanticCapabilityView for LogicalOpView<'_> {
    fn result_facts(&self) -> Option<&Facts> {
        self.result.map(|result| &result.facts)
    }

    fn iteration_domain(&self) -> Option<&IterationDomain> {
        call_from_kind(&self.operation.kind).map(|call| &call.iteration_domain)
    }

    fn access_fact(&self) -> Option<AccessFact> {
        call_from_kind(&self.operation.kind).map(|call| call.access)
    }

    fn effect_summary(&self) -> Option<EffectSummary> {
        call_from_kind(&self.operation.kind).map(|call| call.effect)
    }

    fn speculation_semantics(&self) -> Option<SpeculationSemantics> {
        call_from_kind(&self.operation.kind).map(|call| call.speculation)
    }

    fn possible_errors(&self) -> PossibleErrors {
        match &self.operation.kind {
            OpKind::Basis { call, .. } | OpKind::SemanticCall(call) => call.possible_errors.clone(),
            OpKind::SemanticCheck(check) => PossibleErrors {
                known: vec![check.error],
                unknown: false,
            },
            _ => PossibleErrors::default(),
        }
    }

    fn destination_relation(&self) -> Option<DestinationRelation> {
        call_from_kind(&self.operation.kind).map(|call| call.destination)
    }
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

fn call_constraints(
    basis: &ExecutionBasis,
    rank_plan: Option<&RankPlan>,
    instantiation: &ResolvedInstantiation,
    left: Option<ValueId>,
    right: ValueId,
    left_facts: Option<&Facts>,
    right_facts: &Facts,
) -> ConstraintSet {
    let mut set = ConstraintSet::default();

    if let Some(left_value) = left {
        if basis.outer() == Some(ExecutionBasisKind::CellApply) {
            let witness = rank_plan.and_then(|plan| {
                plan.result_frame
                    .clone()
                    .map(|result_frame| FactWitness::CellFrameAgreement { result_frame })
            });
            let ranks = instantiation.rank_boundary.unwrap_or([0, 0, 0]);
            set.facts.push(ConstraintFact {
                constraint: Constraint::CellFrameAgreement {
                    left: left_value,
                    right,
                    left_rank: ranks[1],
                    right_rank: ranks[2],
                },
                witness,
            });
        } else if basis.outer() == Some(ExecutionBasisKind::Elementwise) {
            let witness = left_facts
                .and_then(|facts| facts.shape.clone())
                .zip(right_facts.shape.clone())
                .and_then(|(left_shape, right_shape)| {
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

    if basis.outer() == Some(ExecutionBasisKind::Gather) {
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

/// Incremental canonical A3 builder used by execution-semantic lowering.
///
/// It owns SSA value creation, ordered semantic checks and the single-block
/// A3-v0 container. It contains no schedule, buffer, layout or target choices.
pub(crate) struct PlanBuilder {
    plan: Plan,
    last_ordered: Option<OpId>,
}

impl PlanBuilder {
    pub(crate) fn new(
        source: String,
        j_graph_node_count: usize,
        j_graph_region_count: usize,
    ) -> Self {
        Self {
            plan: Plan {
                parser_provenance: None,
                header: IrHeader::current(),
                source,
                symbols: Vec::new(),
                functions: Vec::new(),
                regions: Vec::new(),
                blocks: Vec::new(),
                entry: FunctionId(0),
                operations: Vec::new(),
                values: Vec::new(),
                j_graph_node_count,
                j_graph_region_count,
                opportunities: Vec::new(),
                result: None,
                write: None,
            },
            last_ordered: None,
        }
    }

    pub(crate) fn facts(&self, value: ValueId) -> &Facts {
        &self.plan.values[value.0].facts
    }

    pub(crate) fn insert_role(&mut self, value: ValueId, role: crate::facts::ValueRole) {
        self.plan.values[value.0].roles.insert(role);
    }

    pub(crate) fn last_ordered(&self) -> Option<OpId> {
        self.last_ordered
    }

    #[allow(clippy::too_many_arguments)]
    fn push_value(
        &mut self,
        kind: OpKind,
        facts: Facts,
        roles: ValueRoleFacts,
        constraints: &ConstraintSet,
        j_origin: Option<crate::j_graph_ir::ValueId>,
        span: Range<usize>,
        ordered: bool,
    ) -> ValueId {
        let mut order_after = if ordered { self.last_ordered } else { None };
        for fact in constraints.unresolved() {
            let check_id = OpId(self.plan.operations.len());
            self.plan.operations.push(Operation {
                kind: OpKind::SemanticCheck(SemanticCheck {
                    constraint: fact.constraint.clone(),
                    error: error_for(&fact.constraint),
                    origin: span.clone(),
                }),
                results: Vec::new(),
                j_origin,
                span: span.clone(),
                order_after,
            });
            order_after = Some(check_id);
        }

        let op_id = OpId(self.plan.operations.len());
        let value_id = ValueId(self.plan.values.len());
        self.plan.values.push(ValueData {
            producer: op_id,
            facts,
            roles,
        });
        self.plan.operations.push(Operation {
            kind,
            results: vec![value_id],
            j_origin,
            span,
            order_after,
        });
        if ordered {
            self.last_ordered = Some(op_id);
        }
        value_id
    }

    pub(crate) fn push_literal(
        &mut self,
        value: Value,
        facts: Facts,
        j_origin: Option<crate::j_graph_ir::ValueId>,
        span: Range<usize>,
    ) -> ValueId {
        self.push_value(
            OpKind::Literal(value),
            facts,
            ValueRoleFacts::default(),
            &ConstraintSet::default(),
            j_origin,
            span,
            false,
        )
    }

    pub(crate) fn push_input(
        &mut self,
        index: usize,
        j_origin: Option<crate::j_graph_ir::ValueId>,
        span: Range<usize>,
    ) -> ValueId {
        self.push_value(
            OpKind::Input { index },
            Facts::default(),
            ValueRoleFacts::default(),
            &ConstraintSet::default(),
            j_origin,
            span,
            false,
        )
    }

    pub(crate) fn push_read_noun(
        &mut self,
        symbol: SymbolId,
        version: NameVersion,
        facts: Facts,
        j_origin: Option<crate::j_graph_ir::ValueId>,
        span: Range<usize>,
    ) -> ValueId {
        self.push_value(
            OpKind::ReadNoun { symbol, version },
            facts,
            ValueRoleFacts::default(),
            &ConstraintSet::default(),
            j_origin,
            span,
            true,
        )
    }

    pub(crate) fn push_verb_reference(
        &mut self,
        callable: Callable,
        j_origin: Option<crate::j_graph_ir::ValueId>,
        span: Range<usize>,
    ) -> ValueId {
        self.push_value(
            OpKind::VerbReference(callable),
            Facts::default(),
            ValueRoleFacts::default(),
            &ConstraintSet::default(),
            j_origin,
            span,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_call(
        &mut self,
        callable: Callable,
        execution_basis: ExecutionBasis,
        left: Option<ValueId>,
        right: ValueId,
        contract: Contract,
        result_facts: Facts,
        rank_plan: Option<RankPlan>,
        access: AccessFact,
        instantiation: ResolvedInstantiation,
        roles: ValueRoleFacts,
        j_origin: Option<crate::j_graph_ir::ValueId>,
        span: Range<usize>,
    ) -> ValueId {
        let left_facts = left.map(|value| self.facts(value).clone());
        let right_facts = self.facts(right).clone();
        let outer_basis = execution_basis.outer();
        let constraints = call_constraints(
            &execution_basis,
            rank_plan.as_ref(),
            &instantiation,
            left,
            right,
            left_facts.as_ref(),
            &right_facts,
        );
        let effect = resolved_effect_summary(&callable, left, contract);
        let call = CallOp {
            callable,
            execution_basis,
            left,
            right,
            contract,
            iteration_domain: iteration_domain(
                outer_basis,
                &result_facts,
                rank_plan.as_ref(),
                &right_facts,
            ),
            effect,
            speculation: SpeculationSemantics::from_contract(contract),
            possible_errors: PossibleErrors::from_contract(contract),
            destination: DestinationRelation::Unknown,
            instantiation,
            rank_plan,
            access,
            constraints: constraints.clone(),
        };

        let kind = match outer_basis {
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
        self.push_value(
            kind,
            result_facts,
            roles,
            &constraints,
            j_origin,
            span,
            true,
        )
    }

    pub(crate) fn finish(
        mut self,
        symbols: Vec<Symbol>,
        opportunities: Vec<StructuralOpportunity<ValueId>>,
        result: Option<ValueId>,
        write: Option<Write>,
    ) -> Plan {
        self.plan.symbols = symbols;
        self.plan.opportunities = opportunities;
        self.plan.result = result;
        self.plan.write = write;

        let block = BlockId(0);
        let region = RegionId(0);
        self.plan.blocks.push(Block {
            operations: 0..self.plan.operations.len(),
            terminator: Terminator::Return(self.plan.result),
        });
        self.plan.regions.push(Region {
            blocks: vec![block],
        });
        self.plan.functions.push(Function { body: region });
        self.plan
    }
}

impl Plan {
    pub fn operation_view(&self, id: OpId) -> Option<LogicalOpView<'_>> {
        let operation = self.operations.get(id.0)?;
        let result = operation
            .results
            .first()
            .and_then(|value| self.values.get(value.0));
        Some(LogicalOpView { operation, result })
    }

    pub fn verify(&self) -> std::result::Result<(), VerifyError> {
        let fail = |operation: Option<OpId>, message: String| VerifyError { operation, message };
        if let Some(provenance) = &self.parser_provenance {
            if provenance.context.source.as_ref() != self.source {
                return Err(fail(None, "A3/parser source mismatch".into()));
            }
            provenance
                .context
                .verify()
                .map_err(|message| fail(None, message))?;
            if !provenance.context.complete
                || provenance.graph_nodes.len() != self.j_graph_node_count
                || provenance.graph_nodes.iter().any(|nodes| {
                    nodes.is_empty()
                        || nodes
                            .iter()
                            .any(|node| node.0 >= provenance.context.nodes.len())
                })
            {
                return Err(fail(None, "invalid A3 parser provenance".into()));
            }
        }
        let source_len = self.source.len();

        if self.header.schema != A3_SCHEMA_VERSION {
            return Err(fail(None, "unsupported A3 IR schema version".into()));
        }
        if self.header.provenance.primitive_registry_version != crate::primitive::REGISTRY_VERSION {
            return Err(fail(
                None,
                "A3 IR primitive registry provenance does not match compiler".into(),
            ));
        }

        for opportunity in &self.opportunities {
            if let Some(origin) = opportunity.j_region_origin {
                if origin.0 >= self.j_graph_region_count {
                    return Err(fail(
                        None,
                        "structural opportunity J-region origin is out of bounds".into(),
                    ));
                }
            }
            if opportunity.span.start > opportunity.span.end
                || opportunity.span.end > source_len
                || !self.source.is_char_boundary(opportunity.span.start)
                || !self.source.is_char_boundary(opportunity.span.end)
            {
                return Err(fail(None, "invalid structural opportunity span".into()));
            }
            for value in opportunity.values() {
                if value.0 >= self.values.len() {
                    return Err(fail(
                        None,
                        "structural opportunity references an out-of-bounds A3 value".into(),
                    ));
                }
            }
            match &opportunity.topology {
                StructuralTopology::Pipeline { stage_results, .. } if stage_results.len() < 2 => {
                    return Err(fail(
                        None,
                        "pipeline opportunity must contain at least two stages".into(),
                    ));
                }
                StructuralTopology::BranchJoin { branch_results, .. }
                    if branch_results.len() < 2 =>
                {
                    return Err(fail(
                        None,
                        "branch/join opportunity must contain at least two branches".into(),
                    ));
                }
                _ => {}
            }
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

        let mut input_count = 0;
        for (index, operation) in self.operations.iter().enumerate() {
            let op_id = OpId(index);
            if operation.span.start > operation.span.end
                || operation.span.end > source_len
                || !self.source.is_char_boundary(operation.span.start)
                || !self.source.is_char_boundary(operation.span.end)
            {
                return Err(fail(Some(op_id), "invalid source span".into()));
            }
            if let Some(origin) = operation.j_origin {
                if origin.0 >= self.j_graph_node_count {
                    return Err(fail(
                        Some(op_id),
                        "J graph operation origin is out of bounds".into(),
                    ));
                }
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
            let check_symbol = |symbol: SymbolId| {
                if symbol.0 >= self.symbols.len() {
                    Err(fail(Some(op_id), "symbol id is out of bounds".into()))
                } else {
                    Ok(())
                }
            };
            let check_callable = |callable: &Callable| {
                if (callable.target == CallTarget::Definition)
                    != matches!(
                        callable.semantic.head,
                        crate::semantic::FunctionHead::ExplicitDefinition(_)
                    )
                {
                    return Err(fail(
                        Some(op_id),
                        "definition target disagrees with semantic entity".into(),
                    ));
                }
                if callable.semantic.result_pos != crate::semantic::FunctionPartOfSpeech::Verb {
                    return Err(fail(
                        Some(op_id),
                        "callable semantic entity is not a verb".into(),
                    ));
                }
                if let CallTarget::Dynamic(symbol) = callable.target {
                    check_symbol(symbol)?;
                }
                Ok(())
            };
            let check_call_result = |call: &CallOp| {
                if call.callable.target == CallTarget::Definition {
                    return Err(fail(
                        Some(op_id),
                        "definition body requires structural lowering".into(),
                    ));
                }
                let [result] = operation.results.as_slice() else {
                    return Err(fail(
                        Some(op_id),
                        "call operation must produce exactly one SSA value".into(),
                    ));
                };
                let Some(data) = self.values.get(result.0) else {
                    return Err(fail(
                        Some(op_id),
                        "call result value is out of bounds".into(),
                    ));
                };
                if call.instantiation.result_dtype != data.facts.dtype
                    || call.instantiation.result_rank != data.facts.rank
                {
                    return Err(fail(
                        Some(op_id),
                        "call instantiation result facts do not match SSA result facts".into(),
                    ));
                }
                Ok(())
            };

            match &operation.kind {
                OpKind::Input { index } => {
                    if *index != input_count || operation.results.len() != 1 {
                        return Err(fail(
                            Some(op_id),
                            "array inputs require dense indices and one result".into(),
                        ));
                    }
                    input_count += 1;
                }
                OpKind::Literal(_) => {}
                OpKind::ReadNoun { symbol, .. } => check_symbol(*symbol)?,
                OpKind::VerbReference(callable) => check_callable(callable)?,
                OpKind::Basis {
                    kind,
                    payload,
                    call,
                } => {
                    check_callable(&call.callable)?;
                    check_call_result(call)?;
                    if call.execution_basis.outer() != Some(*kind) {
                        return Err(fail(
                            Some(op_id),
                            "outer execution basis does not match basis operation identity".into(),
                        ));
                    }
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
                    check_callable(&call.callable)?;
                    check_call_result(call)?;
                    if !call.execution_basis.is_empty() {
                        return Err(fail(
                            Some(op_id),
                            "semantic fallback unexpectedly retains an execution basis".into(),
                        ));
                    }
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
                return Err(fail(
                    None,
                    format!("value {index} producer is out of bounds"),
                ));
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

//! Execution-oriented semantic lowering from J Graph IR.
//!
//! J grammar/topology discovery belongs to `j_graph_ir`; this module expands
//! the selected graph into explicit execution dataflow, facts, checks and basis
//! identities. IDs are local to one plan, not runtime addresses.
use crate::{
    Error, Result, Value,
    contracts::{self, Contract, Valence},
    facts::{TypeFact, ValueRole, ValueRoleFacts},
    j_graph_ir,
    opportunity::{OpportunitySource, StructuralOpportunity, StructuralTopology},
    semantic::{
        BoundProgram, Expr, ExprKind, FunctionEntity, FunctionHead, FunctionOperand,
        FunctionPartOfSpeech, NameVersion, Verb,
    },
};
use std::{collections::HashMap, ops::Range, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueId(pub usize);
/// Scope identity is separate from a symbol's part of speech.
/// Only CurrentGlobal is emitted until lexical scopes/locales are supported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    CurrentGlobal,
    LocalFrame(u32),
}
#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub scope: Scope,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallTarget {
    Primitive(crate::primitive::PrimitiveId),
    /// Do not freeze this target to its current definition without a proof/guard.
    Dynamic(SymbolId),
}
#[derive(Clone, Debug)]
pub struct Callable {
    pub target: CallTarget,
    /// Shared semantic function graph; lowering may inspect this without
    /// reparsing source or recursively copying a large derived function.
    pub semantic: Arc<FunctionEntity>,
}
#[derive(Clone, Debug)]
pub enum Operation {
    Literal(Value),
    /// Snapshot requirement: value must belong to this observed name version.
    ReadNoun {
        symbol: SymbolId,
        version: NameVersion,
    },
    VerbReference(Callable),
    Call {
        callable: Callable,
        left: Option<ValueId>,
        right: ValueId,
        contract: Contract,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessRelation {
    /// One result element depends on the corresponding logical input element(s).
    ElementwiseMap,
    /// Monadic reduction over the leading logical cell axis in the current v0 model.
    ReduceLeadingAxis,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AccessFact {
    #[default]
    Opaque,
    Known(AccessRelation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExecutionBasisKind {
    IndexSpace,
    Elementwise,
    CellApply,
    StaticReindex,
    Gather,
    Scatter,
    ScatterCombine,
    WindowView,
    SegmentView,
    Permute,
    Reduce,
    Scan,
    Contract,
    ConcatAssemble,
    ReplicateCompactExpand,
    Grade,
    LookupClassify,
    GroupBy,
    NestedTraverse,
    LinearSolve,
    StateMachine,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExecutionBasis {
    /// Outer-to-inner execution-basis structure. A ranked reduction, for
    /// example, remains [CellApply, Reduce] even when A3 keeps it as one
    /// structured call for now.
    pub layers: Vec<ExecutionBasisKind>,
}

impl ExecutionBasis {
    pub fn outer(&self) -> Option<ExecutionBasisKind> {
        self.layers.first().copied()
    }

    pub fn contains(&self, kind: ExecutionBasisKind) -> bool {
        self.layers.contains(&kind)
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

/// Actual call instance facts.  This is analysis metadata, not semantic identity
/// and not a physical implementation choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedInstantiation {
    pub target: CallTarget,
    pub valence: Valence,
    pub left_dtype: Option<TypeFact>,
    pub left_rank: Option<usize>,
    pub right_dtype: TypeFact,
    pub right_rank: Option<usize>,
    pub result_dtype: TypeFact,
    pub result_rank: Option<usize>,
    pub rank_boundary: Option<[i64; 3]>,
}

fn primitive_execution_basis(
    id: crate::primitive::PrimitiveId,
    valence: Valence,
) -> Option<ExecutionBasisKind> {
    use crate::primitive::PrimitiveId::*;

    let dyad = valence == Valence::Dyad;
    match (id, dyad) {
        (IndexOf | Steps, false) => Some(ExecutionBasisKind::IndexSpace),
        (Equal, false) => Some(ExecutionBasisKind::LookupClassify),
        (Indices, false) => Some(ExecutionBasisKind::ReplicateCompactExpand),
        (Shape, true)
        | (Ravel, false)
        | (Reverse, _)
        | (Transpose, _)
        | (Take, _)
        | (Drop, _) => Some(ExecutionBasisKind::StaticReindex),
        (Ravel, true) => Some(ExecutionBasisKind::ConcatAssemble),
        (From, true) => Some(ExecutionBasisKind::Gather),
        (IndexOf | Steps | Indices | Member, true) => Some(ExecutionBasisKind::LookupClassify),
        (Magnitude, true) => Some(ExecutionBasisKind::Elementwise),
        _ if contracts::for_primitive(id, valence).class
            == crate::contracts::OperationClass::Map =>
        {
            Some(ExecutionBasisKind::Elementwise)
        }
        _ => None,
    }
}

fn append_execution_basis(
    function: &Arc<FunctionEntity>,
    valence: Valence,
    layers: &mut Vec<ExecutionBasisKind>,
) {
    match &function.head {
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank) => {
            layers.push(ExecutionBasisKind::CellApply);
            if let Some(FunctionOperand::Function(operand)) = function.operands.first() {
                append_execution_basis(operand, valence, layers);
            }
        }
        FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert) => {
            layers.push(ExecutionBasisKind::Reduce);
            // The reducer leaf identity is carried by the semantic FunctionEntity.
            // Recurse only when the reducer itself is structurally derived so that
            // Insert(Rank(u)) remains distinct from Rank(Insert(u)).
            if let Some(FunctionOperand::Function(operand)) = function.operands.first() {
                if matches!(
                    operand.head,
                    FunctionHead::PrimitiveAdverb(_) | FunctionHead::PrimitiveConjunction(_)
                ) {
                    append_execution_basis(operand, Valence::Dyad, layers);
                }
            }
        }
        FunctionHead::PrimitiveVerb(id) => {
            if let Some(kind) = primitive_execution_basis(*id, valence) {
                layers.push(kind);
            }
        }
        FunctionHead::NameRef(_) => {}
        FunctionHead::PrimitiveAdverb(_)
        | FunctionHead::PrimitiveConjunction(_)
        | FunctionHead::Hook
        | FunctionHead::Fork => {}
    }
}

fn outer_rank_boundary(function: &FunctionEntity) -> Option<[i64; 3]> {
    if !matches!(
        function.head,
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank)
    ) {
        return None;
    }
    let [
        FunctionOperand::Function(_),
        FunctionOperand::Noun { value, .. },
    ] = function.operands.as_slice()
    else {
        return None;
    };
    if value.is_empty() || value.len() > 3 {
        return None;
    }
    let at = |i| value.int_at(i).ok();
    match value.len() {
        1 => {
            let r = at(0)?;
            Some([r, r, r])
        }
        2 => Some([at(1)?, at(0)?, at(1)?]),
        3 => Some([at(0)?, at(1)?, at(2)?]),
        _ => None,
    }
}

fn execution_basis(operation: &Operation) -> ExecutionBasis {
    let Operation::Call { callable, left, .. } = operation else {
        return ExecutionBasis::default();
    };

    let valence = if left.is_some() {
        Valence::Dyad
    } else {
        Valence::Monad
    };
    let mut layers = Vec::new();
    append_execution_basis(&callable.semantic, valence, &mut layers);
    ExecutionBasis { layers }
}

fn input_roles(operation: &Operation) -> Vec<(ValueId, ValueRole)> {
    use crate::primitive::PrimitiveId::*;

    let Operation::Call {
        callable,
        left,
        right,
        ..
    } = operation
    else {
        return Vec::new();
    };
    let CallTarget::Primitive(id) = callable.target else {
        return Vec::new();
    };
    match (id, *left) {
        (IndexOf, None) => vec![(*right, ValueRole::ShapeVector)],
        (Shape, Some(left)) => vec![(left, ValueRole::ShapeVector)],
        (From, Some(left)) => vec![(left, ValueRole::IndexVector)],
        (Take | Drop, Some(left)) => vec![(left, ValueRole::CountVector)],
        (Transpose, Some(left)) => vec![(left, ValueRole::AxisPermutation)],
        _ => Vec::new(),
    }
}

fn result_roles(operation: &Operation) -> ValueRoleFacts {
    use crate::primitive::PrimitiveId::*;

    let mut roles = ValueRoleFacts::default();
    let Operation::Call {
        callable, left, ..
    } = operation
    else {
        return roles;
    };
    let CallTarget::Primitive(id) = callable.target else {
        return roles;
    };
    match (id, left.is_some()) {
        (Shape, false) => roles.insert(ValueRole::ShapeVector),
        (Indices, false) | (IndexOf | Steps, true) => roles.insert(ValueRole::IndexVector),
        _ => {}
    }
    roles
}

#[derive(Clone, Debug)]
pub struct Node {
    pub operation: Operation,
    /// Originating applied J-graph node. One J combinator application may
    /// expand to multiple execution-oriented logical operations.
    pub j_origin: Option<j_graph_ir::ValueId>,
    pub facts: crate::facts::Facts,
    pub rank_plan: Option<crate::facts::RankPlan>,
    /// Outer-to-inner execution-basis structure. This may retain nested
    /// CellApply/Reduce/etc. identity even when the transition IR keeps one
    /// structured call rather than inventing a multi-node expansion.
    pub basis: ExecutionBasis,
    pub instantiation: Option<ResolvedInstantiation>,
    /// Roles describe how this value is used/produced, not a new J noun type.
    pub roles: ValueRoleFacts,
    /// Missing access knowledge is explicit and is an optimization barrier,
    /// never by itself a J semantic error.
    pub access: AccessFact,
    pub span: Range<usize>,
    /// Conservative order edge for potential errors, reads and effects.
    pub order_after: Option<ValueId>,
}
#[derive(Clone, Debug)]
pub struct Write {
    pub symbol: SymbolId,
    pub value: ValueId,
    pub previous: Option<NameVersion>,
    pub proposed: NameVersion,
    pub span: Range<usize>,
    /// Commit only after evaluation and its ordered operations succeed.
    pub after: Option<ValueId>,
}
#[derive(Clone, Debug)]
pub struct LogicalPlan {
    pub source: String,
    pub symbols: Vec<Symbol>,
    pub nodes: Vec<Node>,
    pub j_graph_node_count: usize,
    pub j_graph_region_count: usize,
    /// J syntax/derived semantics exposes topology before generic DAG analysis.
    /// These are target-independent optimization opportunities, not legality proofs.
    pub opportunities: Vec<StructuralOpportunity<ValueId>>,
    pub result: Option<ValueId>,
    pub write: Option<Write>,
}

#[derive(Clone, Debug)]
pub struct CompilationAnalysis {
    /// JAXA-style graph algebra: J grammar/combinators + static graph hints.
    pub j_graph: crate::j_graph_ir::Plan,
    /// Target-independent algebraic alternatives discovered from J Graph IR.
    /// Candidates retain explicit equivalence witnesses and do not mutate j_graph.
    pub graph_rewrites: Vec<crate::j_graph_rewrite::GraphRewriteCandidate>,
    /// Source-vs-replacement resource views in the same target-independent
    /// logical-atom/symbolic-state domain. This is evaluation, not selection.
    pub graph_rewrite_resources:
        Vec<crate::j_graph_resource::RewriteResourceEvaluation>,
    /// Execution-oriented logical dataflow derived from j_graph.
    pub execution: LogicalPlan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyError {
    pub node: Option<ValueId>,
    pub message: String,
}
impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(node) = self.node {
            write!(f, "logical IR verification failed at value {}: {}", node.0, self.message)
        } else {
            write!(f, "logical IR verification failed: {}", self.message)
        }
    }
}
impl std::error::Error for VerifyError {}

impl LogicalPlan {
    pub fn verify(&self) -> std::result::Result<(), VerifyError> {
        let fail = |node: Option<ValueId>, message: String| VerifyError { node, message };
        let source_len = self.source.len();

        for (index, node) in self.nodes.iter().enumerate() {
            let id = ValueId(index);
            if node.span.start > node.span.end
                || node.span.end > source_len
                || !self.source.is_char_boundary(node.span.start)
                || !self.source.is_char_boundary(node.span.end)
            {
                return Err(fail(Some(id), "invalid source span".into()));
            }
            if let Some(origin) = node.j_origin {
                if origin.0 >= self.j_graph_node_count {
                    return Err(fail(
                        Some(id),
                        "J graph origin is out of bounds".into(),
                    ));
                }
            }
            if let Some(rank) = node.facts.rank {
                if let Some(shape) = &node.facts.shape {
                    if rank != shape.len() {
                        return Err(fail(Some(id), "fact rank does not match shape".into()));
                    }
                }
            }
            if let Some(before) = node.order_after {
                if before.0 >= index {
                    return Err(fail(Some(id), "order edge must reference an earlier value".into()));
                }
            }
            if node.basis != execution_basis(&node.operation) {
                return Err(fail(
                    Some(id),
                    "basis metadata does not match the logical operation".into(),
                ));
            }

            let check_symbol = |symbol: SymbolId| {
                (symbol.0 < self.symbols.len())
                    .then_some(())
                    .ok_or_else(|| fail(Some(id), "symbol id out of bounds".into()))
            };
            let check_value = |value: ValueId, label: &str| {
                (value.0 < index)
                    .then_some(())
                    .ok_or_else(|| fail(Some(id), format!("{label} must reference an earlier value")))
            };
            let check_callable = |callable: &Callable| {
                if callable.semantic.result_pos != FunctionPartOfSpeech::Verb {
                    return Err(fail(Some(id), "callable semantic entity is not a verb".into()));
                }
                match callable.target {
                    CallTarget::Primitive(_) => Ok(()),
                    CallTarget::Dynamic(symbol) => check_symbol(symbol),
                }
            };

            match &node.operation {
                Operation::Literal(_) => {}
                Operation::ReadNoun { symbol, .. } => check_symbol(*symbol)?,
                Operation::VerbReference(callable) => check_callable(callable)?,
                Operation::Call {
                    callable,
                    left,
                    right,
                    ..
                } => {
                    check_callable(callable)?;
                    if let Some(left) = left {
                        check_value(*left, "left input")?;
                    }
                    check_value(*right, "right input")?;

                    let Some(instantiation) = &node.instantiation else {
                        return Err(fail(Some(id), "call is missing resolved instantiation".into()));
                    };
                    let expected_valence = if left.is_some() {
                        Valence::Dyad
                    } else {
                        Valence::Monad
                    };
                    if instantiation.valence != expected_valence {
                        return Err(fail(
                            Some(id),
                            "instantiation valence does not match call".into(),
                        ));
                    }
                    if instantiation.target != callable.target {
                        return Err(fail(
                            Some(id),
                            "instantiation target does not match call".into(),
                        ));
                    }
                    if instantiation.result_dtype != node.facts.dtype
                        || instantiation.result_rank != node.facts.rank
                    {
                        return Err(fail(
                            Some(id),
                            "instantiation result facts do not match node facts".into(),
                        ));
                    }
                }
            }
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
                if value.0 >= self.nodes.len() {
                    return Err(fail(
                        None,
                        "structural opportunity references an out-of-bounds value".into(),
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
                StructuralTopology::BranchJoin {
                    branch_results, ..
                } if branch_results.len() < 2 => {
                    return Err(fail(
                        None,
                        "branch/join opportunity must contain at least two branches".into(),
                    ));
                }
                _ => {}
            }
        }

        if let Some(result) = self.result {
            if result.0 >= self.nodes.len() {
                return Err(fail(None, "result value id out of bounds".into()));
            }
        }
        if let Some(write) = &self.write {
            if write.symbol.0 >= self.symbols.len() {
                return Err(fail(None, "write symbol id out of bounds".into()));
            }
            if write.value.0 >= self.nodes.len() {
                return Err(fail(None, "write value id out of bounds".into()));
            }
            if let Some(after) = write.after {
                if after.0 >= self.nodes.len() {
                    return Err(fail(None, "write order dependency out of bounds".into()));
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn lower(
    bound: BoundProgram,
    noun_facts: &dyn Fn(&str) -> crate::facts::Facts,
) -> Result<LogicalPlan> {
    let graph_fact = |name: &str| {
        let facts = noun_facts(name);
        crate::j_graph_ir::GraphFacts::from_logical_parts(
            facts.dtype,
            facts.shape,
            facts.rank,
        )
    };
    lower_graph(
        crate::j_graph_ir::Plan::from_bound_with_graph_facts(bound, &graph_fact)?,
        noun_facts,
    )
}

pub(crate) fn lower_graph(
    graph: crate::j_graph_ir::Plan,
    noun_facts: &dyn Fn(&str) -> crate::facts::Facts,
) -> Result<LogicalPlan> {
    let source = graph.source.clone();
    let graph_node_count = graph.nodes.len();
    let graph_result = graph.result;
    let graph_write = graph.write.clone();
    let graph_region_count = graph.regions.len();
    let graph_regions = graph.regions.clone();

    let mut builder = Builder {
        noun_facts,
        symbols: Vec::new(),
        names: HashMap::new(),
        nodes: Vec::new(),
        opportunities: Vec::new(),
        last_ordered: None,
        reads: HashMap::new(),
        current_j_origin: None,
    };
    let mut value_map = Vec::with_capacity(graph.nodes.len());

    for (index, node) in graph.nodes.into_iter().enumerate() {
        let origin = crate::j_graph_ir::ValueId(index);
        builder.current_j_origin = Some(origin);
        let graph_facts = node.facts.clone();
        let span = node.span;
        let value = match node.kind {
            crate::j_graph_ir::NodeKind::Literal(value) => {
                builder.push(Operation::Literal(value), span, false)
            }
            crate::j_graph_ir::NodeKind::ReadNoun { name, version } => {
                let symbol = builder.symbol(&name);
                builder.push(Operation::ReadNoun { symbol, version }, span, true)
            }
            crate::j_graph_ir::NodeKind::VerbValue { function } => {
                let callable = builder.callable_entity(function)?;
                builder.push(Operation::VerbReference(callable), span, false)
            }
            crate::j_graph_ir::NodeKind::Apply {
                function,
                left,
                right,
                ..
            } => {
                let left = left.map(|value| value_map[value.0]);
                let right = value_map[right.0];
                builder.call_entity(function, left, right, span)?
            }
        };
        let execution_facts = &builder.nodes[value.0].facts;
        if !graph_facts.agrees_with(
            execution_facts.dtype,
            execution_facts.shape.as_deref(),
            execution_facts.rank,
        ) {
            return Err(Error::Unsupported(format!(
                "J Graph/Execution fact drift at graph value {}",
                index
            )));
        }
        value_map.push(value);
    }
    builder.current_j_origin = None;

    // J Graph IR is the source of truth for syntax-derived topology.  Project
    // its explicit regions onto execution ValueIds instead of rediscovering
    // pipelines/branches from the flattened execution DAG.
    for (region_index, region) in graph_regions.into_iter().enumerate() {
        let region_origin = Some(crate::j_graph_ir::RegionId(region_index));
        let map = |value: crate::j_graph_ir::ValueId| value_map[value.0];
        match region.kind {
            crate::j_graph_ir::RegionKind::Pipeline { stage_results } => {
                builder.opportunities.push(StructuralOpportunity {
                    source: OpportunitySource::Atop,
                    j_region_origin: region_origin,
                    span: region.span,
                    topology: StructuralTopology::Pipeline {
                        inputs: region.inputs.into_iter().map(map).collect(),
                        stage_results: stage_results.into_iter().map(map).collect(),
                    },
                });
            }
            crate::j_graph_ir::RegionKind::Hook {
                branch_results,
                join_result,
                live_across,
            } => {
                let shared_inputs = if region.inputs.len() == 1 {
                    region.inputs.into_iter().map(map).collect()
                } else {
                    Vec::new()
                };
                builder.opportunities.push(StructuralOpportunity {
                    source: OpportunitySource::Hook,
                    j_region_origin: region_origin,
                    span: region.span,
                    topology: StructuralTopology::BranchJoin {
                        shared_inputs,
                        branch_results: branch_results.into_iter().map(map).collect(),
                        join_result: map(join_result),
                        live_across: live_across.into_iter().map(map).collect(),
                    },
                });
            }
            crate::j_graph_ir::RegionKind::Fork {
                branch_results,
                join_result,
                live_across,
            } => {
                builder.opportunities.push(StructuralOpportunity {
                    source: OpportunitySource::Fork,
                    j_region_origin: region_origin,
                    span: region.span,
                    topology: StructuralTopology::BranchJoin {
                        shared_inputs: region.inputs.into_iter().map(map).collect(),
                        branch_results: branch_results.into_iter().map(map).collect(),
                        join_result: map(join_result),
                        live_across: live_across.into_iter().map(map).collect(),
                    },
                });
            }
        }
    }

    let result = graph_result.map(|value| value_map[value.0]);
    let write = if let Some(write) = graph_write {
        Some(Write {
            symbol: builder.symbol(&write.name),
            value: value_map[write.value.0],
            previous: write.previous,
            proposed: write.proposed,
            span: write.span,
            after: builder.last_ordered,
        })
    } else {
        None
    };

    Ok(LogicalPlan {
        source,
        symbols: builder.symbols,
        nodes: builder.nodes,
        j_graph_node_count: graph_node_count,
        j_graph_region_count: graph_region_count,
        opportunities: builder.opportunities,
        result,
        write,
    })
}

struct Builder<'a> {
    noun_facts: &'a dyn Fn(&str) -> crate::facts::Facts,
    symbols: Vec<Symbol>,
    names: HashMap<String, SymbolId>,
    nodes: Vec<Node>,
    opportunities: Vec<StructuralOpportunity<ValueId>>,
    last_ordered: Option<ValueId>,
    reads: HashMap<(String, usize, usize), NameVersion>,
    current_j_origin: Option<j_graph_ir::ValueId>,
}
impl Builder<'_> {
    fn symbol(&mut self, name: &str) -> SymbolId {
        if let Some(id) = self.names.get(name) {
            return *id;
        }
        let id = SymbolId(self.symbols.len());
        self.symbols.push(Symbol {
            name: name.into(),
            scope: Scope::CurrentGlobal,
        });
        self.names.insert(name.into(), id);
        id
    }
    fn callable(&mut self, verb: Verb) -> Result<Callable> {
        self.callable_entity(verb.entity)
    }

    fn callable_entity(&mut self, semantic: Arc<FunctionEntity>) -> Result<Callable> {
        let mut current = semantic.clone();
        loop {
            match &current.head {
                FunctionHead::PrimitiveVerb(id) => {
                    return Ok(Callable {
                        target: CallTarget::Primitive(*id),
                        semantic,
                    });
                }
                FunctionHead::NameRef(name) => {
                    return Ok(Callable {
                        target: CallTarget::Dynamic(self.symbol(name)),
                        semantic,
                    });
                }
                FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert)
                    if current.result_pos == FunctionPartOfSpeech::Verb => {
                    let [FunctionOperand::Function(base)] = current.operands.as_slice() else {
                        return Err(Error::Unsupported("malformed insert semantic entity".into()));
                    };
                    current = base.clone();
                }
                FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank)
                    if current.result_pos == FunctionPartOfSpeech::Verb => {
                    let [
                        FunctionOperand::Function(base),
                        FunctionOperand::Noun { value, .. },
                    ] = current.operands.as_slice()
                    else {
                        return Err(Error::Unsupported("malformed rank semantic entity".into()));
                    };
                    if value.is_empty() || value.len() > 3 {
                        return Err(Error::Length);
                    }
                    for i in 0..value.len() {
                        value.int_at(i)?;
                    }
                    current = base.clone();
                }
                FunctionHead::PrimitiveAdverb(_)
                | FunctionHead::PrimitiveConjunction(_)
                | FunctionHead::Hook
                | FunctionHead::Fork => {
                    return Err(Error::Unsupported(
                        "semantic function requires structural lowering".into(),
                    ));
                }
            }
        }
    }
    fn push(&mut self, operation: Operation, span: Range<usize>, ordered: bool) -> ValueId {
        let id = ValueId(self.nodes.len());
        let access = match &operation {
            Operation::Call { callable, left, .. }
                if left.is_none()
                    && matches!(
                        &callable.semantic.head,
                        FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert)
                    ) =>
            {
                AccessFact::Known(AccessRelation::ReduceLeadingAxis)
            }
            Operation::Call {
                callable,
                contract,
                ..
            } if matches!(&callable.semantic.head, FunctionHead::PrimitiveVerb(_))
                && contract.class == crate::contracts::OperationClass::Map =>
            {
                AccessFact::Known(AccessRelation::ElementwiseMap)
            }
            _ => AccessFact::Opaque,
        };
        let (facts, rank_plan) = match &operation {
            Operation::Literal(value) => (crate::facts::Facts::of(value), None),
            Operation::ReadNoun { symbol, .. } => {
                ((self.noun_facts)(&self.symbols[symbol.0].name), None)
            }
            Operation::Call {
                callable,
                left,
                right,
                ..
            } => crate::facts::infer_semantic_call(
                &callable.semantic,
                left.map(|id| &self.nodes[id.0].facts),
                &self.nodes[right.0].facts,
            ),
            _ => (crate::facts::Facts::default(), None),
        };
        let basis = execution_basis(&operation);
        let instantiation = self.resolved_instantiation(&operation, &facts);
        for (value, role) in input_roles(&operation) {
            self.nodes[value.0].roles.insert(role);
        }
        let roles = result_roles(&operation);
        self.nodes.push(Node {
            rank_plan,
            j_origin: self.current_j_origin,
            facts,
            basis,
            instantiation,
            roles,
            access,
            operation,
            span,
            order_after: if ordered { self.last_ordered } else { None },
        });
        if ordered {
            self.last_ordered = Some(id);
        }
        id
    }
    fn resolved_instantiation(
        &self,
        operation: &Operation,
        result: &crate::facts::Facts,
    ) -> Option<ResolvedInstantiation> {
        let Operation::Call {
            callable,
            left,
            right,
            ..
        } = operation
        else {
            return None;
        };
        let left_facts = left.map(|id| &self.nodes[id.0].facts);
        let right_facts = &self.nodes[right.0].facts;
        Some(ResolvedInstantiation {
            target: callable.target,
            valence: if left.is_some() {
                Valence::Dyad
            } else {
                Valence::Monad
            },
            left_dtype: left_facts.map(|facts| facts.dtype),
            left_rank: left_facts.and_then(|facts| facts.rank),
            right_dtype: right_facts.dtype,
            right_rank: right_facts.rank,
            result_dtype: result.dtype,
            result_rank: result.rank,
            rank_boundary: outer_rank_boundary(&callable.semantic),
        })
    }

    fn expression(&mut self, expr: Expr) -> Result<ValueId> {
        let span = expr.span;
        match expr.kind {
            ExprKind::Group(inner) => self.expression(*inner),
            ExprKind::Literal(value) => Ok(self.push(Operation::Literal(value), span, false)),
            ExprKind::ReadName(name) => {
                let version = *self
                    .reads
                    .get(&(name.clone(), span.start, span.end))
                    .ok_or_else(|| Error::Value(name.clone()))?;
                let symbol = self.symbol(&name);
                Ok(self.push(Operation::ReadNoun { symbol, version }, span, true))
            }
            ExprKind::VerbValue(verb) => {
                let callable = self.callable(verb)?;
                Ok(self.push(Operation::VerbReference(callable), span, false))
            }
            ExprKind::Monad { verb, argument } => {
                let right = self.expression(*argument)?;
                self.call(verb, None, right, span)
            }
            ExprKind::Dyad { verb, left, right } => {
                let right = self.expression(*right)?;
                let left = self.expression(*left)?;
                self.call(verb, Some(left), right, span)
            }
        }
    }
    fn call(
        &mut self,
        verb: Verb,
        left: Option<ValueId>,
        right: ValueId,
        span: Range<usize>,
    ) -> Result<ValueId> {
        self.call_entity(verb.entity, left, right, span)
    }

    fn call_entity(
        &mut self,
        semantic: Arc<FunctionEntity>,
        left: Option<ValueId>,
        right: ValueId,
        span: Range<usize>,
    ) -> Result<ValueId> {
        match &semantic.head {
            FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Atop)
            | FunctionHead::Hook
            | FunctionHead::Fork => Err(Error::Unsupported(
                "composite J function reached execution lowering without J Graph expansion".into(),
            )),
            _ => {
                let callable = self.callable_entity(semantic)?;
                let valence = if left.is_some() {
                    Valence::Dyad
                } else {
                    Valence::Monad
                };
                // Base primitive contracts do not prove properties of derived verbs.
                // Inspect semantic identity directly so modifier nesting is not
                // accidentally flattened into reduce/rank flags.
                let contract = match &callable.semantic.head {
                    FunctionHead::PrimitiveVerb(id) => contracts::for_primitive(*id, valence),
                    _ => contracts::lookup("", valence),
                };
                Ok(self.push(
                    Operation::Call {
                        callable,
                        left,
                        right,
                        contract,
                    },
                    span,
                    true,
                ))
            }
        }
    }
}

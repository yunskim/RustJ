//! Execution-free lowering. IDs are local to one plan, not runtime addresses.
//! Plans are inspection snapshots; there is deliberately no execute-plan API.
use crate::{
    Error, Result, Value,
    contracts::{self, Contract, Valence},
    facts::{TypeFact, ValueRole, ValueRoleFacts},
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
    pub reduce: bool,
    pub rank: Option<[i64; 3]>,
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
pub enum BasisKind {
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

fn direct_basis(operation: &Operation) -> Option<BasisKind> {
    use crate::primitive::PrimitiveId::*;

    let Operation::Call {
        callable,
        left,
        contract,
        ..
    } = operation
    else {
        return None;
    };

    if callable.rank.is_some() {
        return Some(BasisKind::CellApply);
    }
    if callable.reduce {
        return Some(BasisKind::Reduce);
    }

    let CallTarget::Primitive(id) = callable.target else {
        return None;
    };
    let dyad = left.is_some();
    match (id, dyad) {
        (IndexOf | Steps, false) => Some(BasisKind::IndexSpace),
        (Equal, false) => Some(BasisKind::LookupClassify),
        (Indices, false) => Some(BasisKind::ReplicateCompactExpand),
        (Shape, true)
        | (Ravel, false)
        | (Reverse, _)
        | (Transpose, _)
        | (Take, _)
        | (Drop, _) => Some(BasisKind::StaticReindex),
        (Ravel, true) => Some(BasisKind::ConcatAssemble),
        (From, true) => Some(BasisKind::Gather),
        (IndexOf | Steps | Indices | Member, true) => Some(BasisKind::LookupClassify),
        (Magnitude, true) => Some(BasisKind::Elementwise),
        _ if contract.class == crate::contracts::OperationClass::Map => {
            Some(BasisKind::Elementwise)
        }
        _ => None,
    }
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
    pub facts: crate::facts::Facts,
    pub rank_plan: Option<crate::facts::RankPlan>,
    /// Direct basis identity when the current transition IR can classify the
    /// operation without inventing a multi-node expansion.
    pub basis: Option<BasisKind>,
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
    /// J syntax/derived semantics exposes topology before generic DAG analysis.
    /// These are target-independent optimization opportunities, not legality proofs.
    pub opportunities: Vec<StructuralOpportunity<ValueId>>,
    pub result: Option<ValueId>,
    pub write: Option<Write>,
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
            if node.basis != direct_basis(&node.operation) {
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
    let mut builder = Builder {
        noun_facts,
        symbols: Vec::new(),
        names: HashMap::new(),
        nodes: Vec::new(),
        opportunities: Vec::new(),
        last_ordered: None,
        reads: bound
            .reads
            .into_iter()
            .map(|r| ((r.name, r.span.start, r.span.end), r.version))
            .collect(),
    };
    let result = bound
        .program
        .expression
        .map(|expr| builder.expression(expr))
        .transpose()?;
    let write = if let Some(write) = bound.write {
        Some(Write {
            symbol: builder.symbol(&write.name),
            value: result.ok_or_else(|| Error::Syntax("assignment without value".into()))?,
            previous: write.previous,
            proposed: write.proposed,
            span: write.span,
            after: builder.last_ordered,
        })
    } else {
        None
    };
    Ok(LogicalPlan {
        source: bound.program.source,
        symbols: builder.symbols,
        nodes: builder.nodes,
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
}
fn flatten_atop_execution(
    semantic: &Arc<FunctionEntity>,
    out: &mut Vec<Arc<FunctionEntity>>,
) -> Result<()> {
    if matches!(
        semantic.head,
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Atop)
    ) {
        let [
            FunctionOperand::Function(outer),
            FunctionOperand::Function(inner),
        ] = semantic.operands.as_slice()
        else {
            return Err(Error::Unsupported("malformed atop semantic entity".into()));
        };
        // f @: g executes g first, then f.  Recursively flatten both sides so
        // an entire atop chain is recorded as one pipeline opportunity.
        flatten_atop_execution(inner, out)?;
        flatten_atop_execution(outer, out)?;
    } else {
        out.push(semantic.clone());
    }
    Ok(())
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
        let mut reduce = false;
        let mut rank = None;
        loop {
            match &current.head {
                FunctionHead::PrimitiveVerb(id) => {
                    return Ok(Callable {
                        target: CallTarget::Primitive(*id),
                        semantic,
                        reduce,
                        rank,
                    });
                }
                FunctionHead::NameRef(name) => {
                    return Ok(Callable {
                        target: CallTarget::Dynamic(self.symbol(name)),
                        semantic,
                        reduce,
                        rank,
                    });
                }
                FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert)
                    if current.result_pos == FunctionPartOfSpeech::Verb => {
                    let [FunctionOperand::Function(base)] = current.operands.as_slice() else {
                        return Err(Error::Unsupported("malformed insert semantic entity".into()));
                    };
                    reduce = true;
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
                    let at = |i| value.int_at(i);
                    rank = Some(match value.len() {
                        1 => [at(0)?, at(0)?, at(0)?],
                        2 => [at(1)?, at(0)?, at(1)?],
                        _ => [at(0)?, at(1)?, at(2)?],
                    });
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
            } => match callable.target {
                CallTarget::Primitive(id) => crate::facts::infer_call(
                    id,
                    callable.reduce,
                    callable.rank,
                    left.map(|id| &self.nodes[id.0].facts),
                    &self.nodes[right.0].facts,
                ),
                _ => (crate::facts::Facts::default(), None),
            },
            _ => (crate::facts::Facts::default(), None),
        };
        let basis = direct_basis(&operation);
        let instantiation = self.resolved_instantiation(&operation, &facts);
        for (value, role) in input_roles(&operation) {
            self.nodes[value.0].roles.insert(role);
        }
        let roles = result_roles(&operation);
        self.nodes.push(Node {
            rank_plan,
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
            rank_boundary: callable.rank,
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
            FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Atop) => {
                let mut stages = Vec::new();
                flatten_atop_execution(&semantic, &mut stages)?;
                if stages.len() < 2 {
                    return Err(Error::Unsupported("malformed atop pipeline".into()));
                }

                let mut inputs = Vec::with_capacity(2);
                if let Some(left) = left {
                    inputs.push(left);
                }
                inputs.push(right);

                let mut current = right;
                let mut stage_results = Vec::with_capacity(stages.len());
                for (index, stage) in stages.into_iter().enumerate() {
                    current = self.call_entity(
                        stage.clone(),
                        if index == 0 { left } else { None },
                        current,
                        stage.span.clone(),
                    )?;
                    stage_results.push(current);
                }
                self.opportunities.push(StructuralOpportunity {
                    source: OpportunitySource::Atop,
                    span: semantic.span.clone(),
                    topology: StructuralTopology::Pipeline {
                        inputs,
                        stage_results,
                    },
                });
                Ok(current)
            }
            FunctionHead::Fork => {
                let [
                    FunctionOperand::Function(f),
                    FunctionOperand::Function(g),
                    FunctionOperand::Function(h),
                ] = semantic.operands.as_slice()
                else {
                    return Err(Error::Unsupported("malformed fork semantic entity".into()));
                };
                // jsource-compatible observable order for a general fork is h, f, g.
                let h_result = self.call_entity(
                    h.clone(),
                    left,
                    right,
                    h.span.clone(),
                )?;
                let f_result = self.call_entity(
                    f.clone(),
                    left,
                    right,
                    f.span.clone(),
                )?;
                let join_result = self.call_entity(
                    g.clone(),
                    Some(f_result),
                    h_result,
                    span.clone(),
                )?;
                let mut shared_inputs = Vec::with_capacity(2);
                if let Some(left) = left {
                    shared_inputs.push(left);
                }
                shared_inputs.push(right);
                self.opportunities.push(StructuralOpportunity {
                    source: OpportunitySource::Fork,
                    span: semantic.span.clone(),
                    topology: StructuralTopology::BranchJoin {
                        shared_inputs: shared_inputs.clone(),
                        // Preserve J's observable branch evaluation order: h, then f.
                        branch_results: vec![h_result, f_result],
                        join_result,
                        live_across: shared_inputs,
                    },
                });
                Ok(join_result)
            }
            FunctionHead::Hook => {
                let [
                    FunctionOperand::Function(f),
                    FunctionOperand::Function(g),
                ] = semantic.operands.as_slice()
                else {
                    return Err(Error::Unsupported("malformed hook semantic entity".into()));
                };
                // (f g) y = y f (g y); x (f g) y = x f (g y).
                // The right verb therefore executes before f.
                let g_result = self.call_entity(
                    g.clone(),
                    None,
                    right,
                    g.span.clone(),
                )?;
                let f_left = left.unwrap_or(right);
                let join_result = self.call_entity(
                    f.clone(),
                    Some(f_left),
                    g_result,
                    span.clone(),
                )?;
                self.opportunities.push(StructuralOpportunity {
                    source: OpportunitySource::Hook,
                    span: semantic.span.clone(),
                    topology: StructuralTopology::BranchJoin {
                        shared_inputs: vec![f_left, right],
                        branch_results: vec![f_left, g_result],
                        join_result,
                        // In a monadic hook the original y must remain available
                        // while g(y) is computed.  In the dyad, left plays that role.
                        live_across: vec![f_left],
                    },
                });
                Ok(join_result)
            }
            _ => {
                let callable = self.callable_entity(semantic)?;
                let valence = if left.is_some() {
                    Valence::Dyad
                } else {
                    Valence::Monad
                };
                // Base primitive contracts do not prove properties of derived verbs.
                let contract = match callable.target {
                    CallTarget::Primitive(id) if !callable.reduce && callable.rank.is_none() => {
                        contracts::for_primitive(id, valence)
                    }
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

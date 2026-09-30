//! Execution-free lowering. IDs are local to one plan, not runtime addresses.
//! Plans are inspection snapshots; there is deliberately no execute-plan API.
use crate::{
    Error, Result, Value,
    contracts::{self, Contract, RankContract, RankSpec, Valence},
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
    /// Explicit rank-conjunction boundaries, outermost first.
    pub explicit_ranks: Vec<RankContract>,
    /// Innate rank of the resolved callable. Dynamic names remain unknown.
    pub innate_rank: Option<RankContract>,
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

#[derive(Clone, Debug)]
pub struct Node {
    pub operation: Operation,
    pub facts: crate::facts::Facts,
    pub cell_application: Option<crate::facts::CellApplicationPlan>,
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
                }
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
        result,
        write,
    })
}

fn rank_spec_at(value: &Value, index: usize) -> Result<RankSpec> {
    match value.data() {
        crate::Data::Bool(values) => Ok(RankSpec::Absolute(values[index] as usize)),
        crate::Data::Int(values) => Ok(RankSpec::from_integer(values[index])),
        crate::Data::Float(values) => {
            let value = values[index];
            if value == f64::INFINITY {
                Ok(RankSpec::Infinite)
            } else if value == f64::NEG_INFINITY {
                // jsource clamps the noun rank to -RMAX before resolving it
                // relative to an argument; for any realizable array rank this
                // is therefore rank 0.
                Ok(RankSpec::Relative(i64::MIN))
            } else if value.is_finite()
                && value.fract() == 0.0
                && value >= i64::MIN as f64
                && value < -(i64::MIN as f64)
            {
                Ok(RankSpec::from_integer(value as i64))
            } else {
                Err(Error::Domain)
            }
        }
        crate::Data::Char(_) | crate::Data::Boxed(_) | crate::Data::Sparse(_) => Err(Error::Domain),
    }
}

fn rank_contract_from_noun(value: &Value) -> Result<RankContract> {
    if value.shape().len() > 1 {
        return Err(Error::Rank);
    }
    if value.is_empty() || value.len() > 3 {
        return Err(Error::Length);
    }
    let at = |index| rank_spec_at(value, index);
    Ok(match value.len() {
        1 => {
            let rank = at(0)?;
            RankContract::all(rank)
        }
        2 => RankContract::new(at(1)?, at(0)?, at(1)?),
        3 => RankContract::new(at(0)?, at(1)?, at(2)?),
        _ => unreachable!("rank noun length checked above"),
    })
}

struct Builder<'a> {
    noun_facts: &'a dyn Fn(&str) -> crate::facts::Facts,
    symbols: Vec<Symbol>,
    names: HashMap<String, SymbolId>,
    nodes: Vec<Node>,
    last_ordered: Option<ValueId>,
    reads: HashMap<(String, usize, usize), NameVersion>,
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
        let mut explicit_ranks = Vec::new();
        loop {
            match &current.head {
                FunctionHead::PrimitiveVerb(id) => {
                    let innate_rank = if reduce {
                        // Insert is a derived monadic operation over its whole
                        // argument cell. The operand primitive's scalar rank is
                        // inside the reduction, not the rank of +/ itself.
                        Some(RankContract::all(RankSpec::Infinite))
                    } else {
                        Some(contracts::innate_rank(*id))
                    };
                    return Ok(Callable {
                        target: CallTarget::Primitive(*id),
                        semantic,
                        reduce,
                        explicit_ranks,
                        innate_rank,
                    });
                }
                FunctionHead::NameRef(name) => {
                    return Ok(Callable {
                        target: CallTarget::Dynamic(self.symbol(name)),
                        semantic,
                        reduce,
                        explicit_ranks,
                        innate_rank: None,
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
                    if reduce {
                        return Err(Error::Unsupported(
                            "ranked reduction operand requires structural reduction lowering".into(),
                        ));
                    }
                    let [
                        FunctionOperand::Function(base),
                        FunctionOperand::Noun { value, .. },
                    ] = current.operands.as_slice()
                    else {
                        return Err(Error::Unsupported(
                            "rank with non-noun right operand requires rank-contract resolution"
                                .into(),
                        ));
                    };
                    explicit_ranks.push(rank_contract_from_noun(value)?);
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
        let (facts, cell_application) = match &operation {
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
                    &callable.explicit_ranks,
                    callable.innate_rank,
                    left.map(|id| &self.nodes[id.0].facts),
                    &self.nodes[right.0].facts,
                ),
                _ => (
                    crate::facts::Facts::default(),
                    crate::facts::plan_cell_application(
                        &callable.explicit_ranks,
                        callable.innate_rank,
                        left.map(|id| &self.nodes[id.0].facts),
                        &self.nodes[right.0].facts,
                    ),
                ),
            },
            _ => (crate::facts::Facts::default(), None),
        };
        self.nodes.push(Node {
            cell_application,
            facts,
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
                self.call_entity(
                    g.clone(),
                    Some(f_result),
                    h_result,
                    span,
                )
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
                self.call_entity(
                    f.clone(),
                    Some(f_left),
                    g_result,
                    span,
                )
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
                    CallTarget::Primitive(id) if !callable.reduce && callable.explicit_ranks.is_empty() => {
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

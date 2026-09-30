//! Execution-free lowering. IDs are local to one plan, not runtime addresses.
//! Plans are inspection snapshots; there is deliberately no execute-plan API.
use crate::{
    Error, Result, Value,
    contracts::{self, Contract, Valence},
    semantic::{BoundProgram, Expr, ExprKind, NameVersion, Verb, VerbModifier},
};
use std::{collections::HashMap, ops::Range};

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallTarget {
    Primitive(crate::primitive::PrimitiveId),
    /// Do not freeze this target to its current definition without a proof/guard.
    Dynamic(SymbolId),
}
#[derive(Clone, Debug)]
pub struct Callable {
    pub target: CallTarget,
    /// Ordered semantic modifier provenance from the J Semantic IR.
    pub modifiers: Vec<VerbModifier>,
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
#[derive(Clone, Debug)]
pub struct Node {
    pub operation: Operation,
    pub facts: crate::facts::Facts,
    pub rank_plan: Option<crate::facts::RankPlan>,
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
    fn callable(&mut self, verb: Verb) -> Callable {
        let target = match verb.target {
            crate::semantic::VerbTarget::Named(name) => CallTarget::Dynamic(self.symbol(&name)),
            crate::semantic::VerbTarget::Primitive(id) => CallTarget::Primitive(id),
        };
        Callable {
            target,
            modifiers: verb.modifiers,
            reduce: verb.reduce,
            rank: verb.rank,
        }
    }
    fn push(&mut self, operation: Operation, span: Range<usize>, ordered: bool) -> ValueId {
        let id = ValueId(self.nodes.len());
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
        self.nodes.push(Node {
            rank_plan,
            facts,
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
                let callable = self.callable(verb);
                Ok(self.push(Operation::VerbReference(callable), span, false))
            }
            ExprKind::Monad { verb, argument } => {
                let right = self.expression(*argument)?;
                Ok(self.call(verb, None, right, span))
            }
            ExprKind::Dyad { verb, left, right } => {
                let right = self.expression(*right)?;
                let left = self.expression(*left)?;
                Ok(self.call(verb, Some(left), right, span))
            }
        }
    }
    fn call(
        &mut self,
        verb: Verb,
        left: Option<ValueId>,
        right: ValueId,
        span: Range<usize>,
    ) -> ValueId {
        let callable = self.callable(verb);
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
        self.push(
            Operation::Call {
                callable,
                left,
                right,
                contract,
            },
            span,
            true,
        )
    }
}

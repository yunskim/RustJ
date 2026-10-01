//! Legacy M1 transition IR.
//!
//! This module contains the temporary execution-plan container that predates
//! the canonical A3 `logical_ir::Plan`. It exists only to keep compatibility
//! APIs working while direct J Graph → A3 lowering is completed. New compiler
//! code must not add features here.

use crate::{
    Value,
    contracts::Contract,
    execution_semantics::{
        AccessFact, Callable, ExecutionBasis, ResolvedInstantiation, Symbol, SymbolId,
    },
    facts::{RankPlan, ValueRoleFacts},
    opportunity::StructuralOpportunity,
    semantic::NameVersion,
};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueId(pub usize);

#[derive(Clone, Debug)]
pub enum Operation {
    Literal(Value),
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
    pub j_origin: Option<crate::j_graph_ir::ValueId>,
    pub facts: crate::facts::Facts,
    pub rank_plan: Option<RankPlan>,
    pub basis: ExecutionBasis,
    pub instantiation: Option<ResolvedInstantiation>,
    pub roles: ValueRoleFacts,
    pub access: AccessFact,
    pub span: Range<usize>,
    pub order_after: Option<ValueId>,
}

#[derive(Clone, Debug)]
pub struct Write {
    pub symbol: SymbolId,
    pub value: ValueId,
    pub previous: Option<NameVersion>,
    pub proposed: NameVersion,
    pub span: Range<usize>,
    pub after: Option<ValueId>,
}

#[derive(Clone, Debug)]
pub struct LogicalPlan {
    pub source: String,
    pub symbols: Vec<Symbol>,
    pub nodes: Vec<Node>,
    pub j_graph_node_count: usize,
    pub j_graph_region_count: usize,
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
            write!(
                f,
                "transition IR verification failed at value {}: {}",
                node.0, self.message
            )
        } else {
            write!(f, "transition IR verification failed: {}", self.message)
        }
    }
}

impl std::error::Error for VerifyError {}

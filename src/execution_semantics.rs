//! Shared execution-semantic contracts between J Graph lowering and A3 Logical IR.
//!
//! These types describe target-independent executable meaning. They are not a
//! transition plan, schedule, buffer assignment, device placement, or backend
//! realization. Keeping them outside `analysis.rs` prevents the temporary
//! `analysis::LogicalPlan` container from owning canonical compiler vocabulary.

use crate::{contracts::Valence, facts::TypeFact, semantic::FunctionEntity};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

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
    /// Immutable semantic definition value. Body execution still requires
    /// structural lowering; this is never a primitive or a tensor constant.
    Definition,
}

#[derive(Clone, Debug)]
pub struct Callable {
    pub target: CallTarget,
    /// Shared semantic function graph; lowering may inspect this without
    /// reparsing source or recursively copying a large derived function.
    pub semantic: Arc<FunctionEntity>,
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

/// Actual call instance facts. This is analysis metadata, not semantic identity
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

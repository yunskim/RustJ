//! Aggregate compiler analysis result.
//!
//! This module owns the cross-stage analysis bundle. Canonical executable
//! analysis is represented only by A3 `logical_ir::Plan`.

#[derive(Clone, Debug)]
pub struct CompilationAnalysis {
    /// J grammar/combinator-aware computation graph.
    pub j_graph: crate::j_graph_ir::Plan,
    /// Target-independent algebraic alternatives discovered from J Graph IR.
    pub graph_rewrites: Vec<crate::j_graph_rewrite::GraphRewriteCandidate>,
    /// Source-vs-replacement resource views in the same target-independent
    /// logical-atom/symbolic-state domain.
    pub graph_rewrite_resources: Vec<crate::j_graph_resource::RewriteResourceEvaluation>,
    /// Canonical A3 Logical Execution IR for new compiler consumers.
    pub logical: crate::logical_ir::Plan,
}

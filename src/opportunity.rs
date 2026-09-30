//! Structural optimization opportunities exposed directly by J semantic syntax.
//!
//! These records are target-independent.  They preserve topology and provenance
//! that would otherwise have to be rediscovered from a flattened dataflow DAG.
//! They are opportunities, not proofs that fusion/parallel execution is legal.

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OpportunitySource {
    Atop,
    Hook,
    Fork,
    /// Reserved for the backward-intent/VJP pass.  Adjoint expansion is not
    /// implemented yet, but its fan-out topology belongs in this same layer.
    AdjointVjp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StructuralTopology<V> {
    /// J @: exposes an ordered producer-consumer pipeline.
    Pipeline {
        inputs: Vec<V>,
        /// Results after each stage, in execution order.  Every result except
        /// the last is a candidate for materialization elision.
        stage_results: Vec<V>,
    },
    /// Hook/fork expose fan-out/fan-in and values that must remain available
    /// across branch computation.  Branch ordering here preserves J observable
    /// evaluation order; parallel execution still requires a semantic proof.
    BranchJoin {
        shared_inputs: Vec<V>,
        branch_results: Vec<V>,
        join_result: V,
        live_across: Vec<V>,
    },
    /// Backward-intent expansion can expose data-adjoint and parameter-adjoint
    /// branches that have no data dependency on one another.
    ParallelFanOut {
        inputs: Vec<V>,
        branch_results: Vec<V>,
        continuing: Option<V>,
        emitted: Vec<V>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralOpportunity<V> {
    pub source: OpportunitySource,
    pub span: Range<usize>,
    pub topology: StructuralTopology<V>,
}

impl<V> StructuralOpportunity<V> {
    pub fn map_values<U>(self, mut map: impl FnMut(V) -> U) -> StructuralOpportunity<U> {
        let topology = match self.topology {
            StructuralTopology::Pipeline {
                inputs,
                stage_results,
            } => StructuralTopology::Pipeline {
                inputs: inputs.into_iter().map(&mut map).collect(),
                stage_results: stage_results.into_iter().map(&mut map).collect(),
            },
            StructuralTopology::BranchJoin {
                shared_inputs,
                branch_results,
                join_result,
                live_across,
            } => StructuralTopology::BranchJoin {
                shared_inputs: shared_inputs.into_iter().map(&mut map).collect(),
                branch_results: branch_results.into_iter().map(&mut map).collect(),
                join_result: map(join_result),
                live_across: live_across.into_iter().map(&mut map).collect(),
            },
            StructuralTopology::ParallelFanOut {
                inputs,
                branch_results,
                continuing,
                emitted,
            } => StructuralTopology::ParallelFanOut {
                inputs: inputs.into_iter().map(&mut map).collect(),
                branch_results: branch_results.into_iter().map(&mut map).collect(),
                continuing: continuing.map(&mut map),
                emitted: emitted.into_iter().map(&mut map).collect(),
            },
        };
        StructuralOpportunity {
            source: self.source,
            span: self.span,
            topology,
        }
    }

    pub fn values(&self) -> Vec<&V> {
        match &self.topology {
            StructuralTopology::Pipeline {
                inputs,
                stage_results,
            } => inputs.iter().chain(stage_results).collect(),
            StructuralTopology::BranchJoin {
                shared_inputs,
                branch_results,
                join_result,
                live_across,
            } => shared_inputs
                .iter()
                .chain(branch_results)
                .chain(std::iter::once(join_result))
                .chain(live_across)
                .collect(),
            StructuralTopology::ParallelFanOut {
                inputs,
                branch_results,
                continuing,
                emitted,
            } => inputs
                .iter()
                .chain(branch_results)
                .chain(continuing.iter())
                .chain(emitted)
                .collect(),
        }
    }
}

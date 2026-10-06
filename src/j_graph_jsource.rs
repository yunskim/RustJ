//! jsource-derived optimization opportunities at the J Graph boundary.
//!
//! An opportunity is an audited *question* for Execution Semantic Lowering or a
//! downstream planner, not an equivalence proof, transformed graph, selected
//! kernel, or performance assertion. In particular, source syntactic identity
//! never changes the J execution/error order in this pass.
//!
//! This sidecar is deliberately separate from witnessed algebraic rewrites and
//! fusion envelopes: these optimizations often change the algorithm or storage
//! strategy, rather than the logical expression.

use std::ops::Range;

use crate::{
    j_graph_ir::{GraphBasis, GraphFacts, GraphForm, NodeKind, Plan, ValueId},
    primitive::PrimitiveId,
    semantic::{FunctionHead, Valence},
};

pub const JSOURCE_SOURCE_PIN: &str = "13994ffa1ed5f06f79fad6e9822a7ed2d29b1528";
pub const JSOURCE_CATALOG_VERSION: u32 = 1;

/// The kind of question this optimization family asks. These are not IR nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum JsourceFamily {
    ReductionFastPath,
    WindowAlgorithm,
    SearchAlgorithm,
    IntervalLookup,
    GatherCopyOrView,
    ReindexCopyOrView,
    MapReduceStreaming,
    ResultAssemblyDemand,
    GroupAggregate,
    MatrixContraction,
    GradeRanking,
    TolerantHash,
    SparseAlgorithm,
    BufferOwnership,
    NameLookupCache,
}

/// Which existing RustJ stage is allowed to own the decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionOwner {
    ExecutionSemantics,
    GraphFusion,
    ExecutionAlgorithm,
    PhysicalPlanner,
    RuntimeBinding,
}

/// Discovery support is distinct from executable support.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveryCoverage {
    /// This module can recognize a source-family pattern, but cannot select it.
    AnalysisOnly,
    /// Existing separate analysis owns the recognition; do not duplicate it.
    ExistingAnalyzer,
    /// J frontend or derived-verb contracts are not yet strong enough.
    AwaitingFrontendOrFacts,
    /// The optimization concerns a lower stage, not a graph rewrite.
    DownstreamOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProofRequirement {
    RankCellAndAssembly,
    NumericToleranceAndFit,
    EmptySparseAndPrototype,
    EffectAndErrorOrdering,
    InputTypeAndShape,
    ConsumerDemandAndFanout,
    AliasLifetimeAndOwnership,
    AlgorithmTargetAndCost,
    BindingAndLocaleVersion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsourceFamilyRule {
    pub family: JsourceFamily,
    /// Stable descriptive identity for diagnostics or future versioned caches.
    pub stable_id: &'static str,
    pub source_file: &'static str,
    pub source_symbol: &'static str,
    pub owner: DecisionOwner,
    pub discovery: DiscoveryCoverage,
    pub proof_requirements: &'static [ProofRequirement],
}

use ProofRequirement::*;

const NUMERIC: &[ProofRequirement] = &[
    RankCellAndAssembly,
    NumericToleranceAndFit,
    EmptySparseAndPrototype,
    EffectAndErrorOrdering,
    InputTypeAndShape,
    AlgorithmTargetAndCost,
];
const ACCESS: &[ProofRequirement] = &[
    RankCellAndAssembly,
    EmptySparseAndPrototype,
    EffectAndErrorOrdering,
    InputTypeAndShape,
    AliasLifetimeAndOwnership,
    AlgorithmTargetAndCost,
];
const GROUP: &[ProofRequirement] = &[
    RankCellAndAssembly,
    NumericToleranceAndFit,
    EmptySparseAndPrototype,
    EffectAndErrorOrdering,
    InputTypeAndShape,
    ConsumerDemandAndFanout,
    AlgorithmTargetAndCost,
];

/// Reviewed upstream entrypoints, pinned independently of current jsource HEAD.
/// Rule definitions do not authorize executing an optimization.
pub const JSOURCE_FAMILY_RULES: &[JsourceFamilyRule] = &[
    JsourceFamilyRule {
        family: JsourceFamily::ReductionFastPath, stable_id: "jsource.reduce-fast-path",
        source_file: "jsrc/ar.c", source_symbol: "jtreduce", owner: DecisionOwner::ExecutionSemantics,
        discovery: DiscoveryCoverage::AnalysisOnly, proof_requirements: NUMERIC,
    },
    JsourceFamilyRule {
        family: JsourceFamily::WindowAlgorithm, stable_id: "jsource.window-algorithm",
        source_file: "jsrc/ap.c", source_symbol: "jtbslash/jtmovfslash",
        owner: DecisionOwner::ExecutionAlgorithm, discovery: DiscoveryCoverage::AnalysisOnly,
        proof_requirements: NUMERIC,
    },
    JsourceFamilyRule {
        family: JsourceFamily::SearchAlgorithm, stable_id: "jsource.search-algorithm",
        source_file: "jsrc/vi.c", source_symbol: "indexofsub/jtiobs",
        owner: DecisionOwner::ExecutionAlgorithm, discovery: DiscoveryCoverage::AnalysisOnly,
        proof_requirements: GROUP,
    },
    JsourceFamilyRule {
        family: JsourceFamily::IntervalLookup, stable_id: "jsource.interval-lookup",
        source_file: "jsrc/viix.c", source_symbol: "interval-index search",
        owner: DecisionOwner::ExecutionAlgorithm, discovery: DiscoveryCoverage::AnalysisOnly,
        proof_requirements: ACCESS,
    },
    JsourceFamilyRule {
        family: JsourceFamily::GatherCopyOrView, stable_id: "jsource.gather-copy-or-view",
        source_file: "jsrc/vfrom.c", source_symbol: "jtget1cell",
        owner: DecisionOwner::PhysicalPlanner, discovery: DiscoveryCoverage::AnalysisOnly,
        proof_requirements: ACCESS,
    },
    JsourceFamilyRule {
        family: JsourceFamily::ReindexCopyOrView, stable_id: "jsource.reindex-copy-or-view",
        source_file: "jsrc/vf.c", source_symbol: "reshape virtual/inplace",
        owner: DecisionOwner::PhysicalPlanner, discovery: DiscoveryCoverage::AnalysisOnly,
        proof_requirements: ACCESS,
    },
    JsourceFamilyRule {
        family: JsourceFamily::MapReduceStreaming, stable_id: "jsource.map-reduce-streaming",
        source_file: "jsrc/va2.c", source_symbol: "jtfslashatg",
        owner: DecisionOwner::GraphFusion, discovery: DiscoveryCoverage::ExistingAnalyzer,
        proof_requirements: NUMERIC,
    },
    JsourceFamilyRule {
        family: JsourceFamily::ResultAssemblyDemand, stable_id: "jsource.result-assembly-demand",
        source_file: "jsrc/result.h", source_symbol: "ZZFLAGWILLBEOPENED/COUNTITEMS",
        owner: DecisionOwner::ExecutionSemantics, discovery: DiscoveryCoverage::AwaitingFrontendOrFacts,
        proof_requirements: GROUP,
    },
    JsourceFamilyRule {
        family: JsourceFamily::GroupAggregate, stable_id: "jsource.group-aggregate",
        source_file: "jsrc/ao.c", source_symbol: "jtkeyct/jtsldot",
        owner: DecisionOwner::ExecutionAlgorithm, discovery: DiscoveryCoverage::AwaitingFrontendOrFacts,
        proof_requirements: GROUP,
    },
    JsourceFamilyRule {
        family: JsourceFamily::MatrixContraction, stable_id: "jsource.matrix-contraction",
        source_file: "jsrc/cip.c", source_symbol: "jtpdt/jtdot",
        owner: DecisionOwner::ExecutionAlgorithm, discovery: DiscoveryCoverage::AwaitingFrontendOrFacts,
        proof_requirements: NUMERIC,
    },
    JsourceFamilyRule {
        family: JsourceFamily::GradeRanking, stable_id: "jsource.grade-ranking",
        source_file: "jsrc/vg.c", source_symbol: "jtgrade1/range dispatch",
        owner: DecisionOwner::ExecutionAlgorithm, discovery: DiscoveryCoverage::AwaitingFrontendOrFacts,
        proof_requirements: GROUP,
    },
    JsourceFamilyRule {
        family: JsourceFamily::TolerantHash, stable_id: "jsource.tolerant-hash",
        source_file: "jsrc/viavx2.c", source_symbol: "tolerant neighbor-bucket hash",
        owner: DecisionOwner::ExecutionAlgorithm, discovery: DiscoveryCoverage::DownstreamOnly,
        proof_requirements: GROUP,
    },
    JsourceFamilyRule {
        family: JsourceFamily::SparseAlgorithm, stable_id: "jsource.sparse-algorithm",
        source_file: "jsrc/cpdtsp.c", source_symbol: "jtpdtsp",
        owner: DecisionOwner::ExecutionAlgorithm, discovery: DiscoveryCoverage::DownstreamOnly,
        proof_requirements: NUMERIC,
    },
    JsourceFamilyRule {
        family: JsourceFamily::BufferOwnership, stable_id: "jsource.buffer-ownership",
        source_file: "jsrc/vcat.c", source_symbol: "boxed ownership transfer",
        owner: DecisionOwner::PhysicalPlanner, discovery: DiscoveryCoverage::DownstreamOnly,
        proof_requirements: ACCESS,
    },
    JsourceFamilyRule {
        family: JsourceFamily::NameLookupCache, stable_id: "jsource.name-lookup-cache",
        source_file: "jsrc/sc.c", source_symbol: "jtunquote short/long caches",
        owner: DecisionOwner::RuntimeBinding, discovery: DiscoveryCoverage::DownstreamOnly,
        proof_requirements: &[BindingAndLocaleVersion, EffectAndErrorOrdering],
    },
];

pub fn family_rule(family: JsourceFamily) -> &'static JsourceFamilyRule {
    JSOURCE_FAMILY_RULES.iter().find(|r| r.family == family)
        .expect("every discoverable family must have a source-provenance rule")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpportunityLegality {
    /// A syntax/graph pattern is not a proof of equivalence or profitability.
    AwaitingSemanticProofs,
}

/// Target-independent source observation, not a new graph expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsourceOpportunity {
    pub family: JsourceFamily,
    pub source_value: ValueId,
    pub source_span: Range<usize>,
    pub source_basis: GraphBasis,
    pub source_facts: GraphFacts,
    pub legality: OpportunityLegality,
    pub selected: bool,
}

impl JsourceOpportunity {
    pub fn rule(&self) -> &'static JsourceFamilyRule {
        family_rule(self.family)
    }

    /// Reject forged or stale observations, including illegal selected states.
    pub fn verify(&self, plan: &Plan) -> Result<(), &'static str> {
        if !discover(plan).iter().any(|expected| expected == self) {
            return Err("jsource opportunity differs from source graph or proof state");
        }
        Ok(())
    }
}

/// Enumerate only the patterns the current frontend genuinely represents.
/// Notably, unavailable Key, Grade and Dot constructors are not fabricated.
pub fn discover(plan: &Plan) -> Vec<JsourceOpportunity> {
    let mut result = Vec::new();
    for (idx, node) in plan.nodes.iter().enumerate() {
        let NodeKind::Apply { function, form, valence, left, basis, .. } = &node.kind else {
            continue;
        };
        let family = match form {
            GraphForm::Reduce { .. } => Some(JsourceFamily::ReductionFastPath),
            GraphForm::PrefixInfix { .. } => Some(JsourceFamily::WindowAlgorithm),
            GraphForm::Atomic => match (&function.head, valence, left) {
                (FunctionHead::PrimitiveVerb(PrimitiveId::IndexOf | PrimitiveId::Member | PrimitiveId::Find),
                    Valence::Dyad, Some(_)) => Some(JsourceFamily::SearchAlgorithm),
                (FunctionHead::PrimitiveVerb(PrimitiveId::Indices),
                    Valence::Dyad, Some(_)) => Some(JsourceFamily::IntervalLookup),
                (FunctionHead::PrimitiveVerb(PrimitiveId::From),
                    Valence::Dyad, Some(_)) => Some(JsourceFamily::GatherCopyOrView),
                (FunctionHead::PrimitiveVerb(PrimitiveId::Shape | PrimitiveId::Reverse |
                    PrimitiveId::Transpose | PrimitiveId::Take | PrimitiveId::Drop),
                    Valence::Dyad, Some(_)) => Some(JsourceFamily::ReindexCopyOrView),
                (FunctionHead::PrimitiveVerb(PrimitiveId::Ravel | PrimitiveId::Reverse |
                    PrimitiveId::Transpose),
                    Valence::Monad, None) => Some(JsourceFamily::ReindexCopyOrView),
                _ => None,
            },
            _ => None,
        };
        if let Some(family) = family {
            debug_assert_eq!(family_rule(family).discovery, DiscoveryCoverage::AnalysisOnly);
            result.push(JsourceOpportunity {
                family,
                source_value: ValueId(idx),
                source_span: node.span.clone(),
                source_basis: basis.clone(),
                source_facts: node.facts.clone(),
                legality: OpportunityLegality::AwaitingSemanticProofs,
                selected: false,
            });
        }
    }
    result
}

impl Plan {
    pub fn jsource_opportunities(&self) -> Vec<JsourceOpportunity> {
        discover(self)
    }
}

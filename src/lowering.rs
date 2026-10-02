//! Target-aware lowering legality for logical basis operations.
//!
//! This module deliberately answers only "is this realization family legal?".
//! Cost ranking, scheduling, bufferization and concrete kernel selection belong
//! to later planning stages.

use crate::{
    Error, Value,
    compilation::CompilationAnalysis,
    execution_semantics::{AccessFact, AccessRelation, ExecutionBasisKind},
    logical_ir::{CallOp, ExecutionBasisPayload, IterationDomain, OpKind, Operation, Plan},
};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetFamily {
    Cpu,
    Gpu,
    External,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetFeature {
    Simd,
    Threads,
    IndexedMemory,
    SubgroupCollective,
    TensorContract,
    ExternalCall,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetCapabilities {
    pub family: TargetFamily,
    pub features: Vec<TargetFeature>,
}

impl TargetCapabilities {
    pub fn cpu_baseline() -> Self {
        Self {
            family: TargetFamily::Cpu,
            features: Vec::new(),
        }
    }

    pub fn cpu_simd() -> Self {
        Self {
            family: TargetFamily::Cpu,
            features: vec![TargetFeature::Simd, TargetFeature::IndexedMemory],
        }
    }

    pub fn gpu_generic() -> Self {
        Self {
            family: TargetFamily::Gpu,
            features: vec![
                TargetFeature::Threads,
                TargetFeature::IndexedMemory,
                TargetFeature::SubgroupCollective,
            ],
        }
    }

    pub fn has(&self, feature: TargetFeature) -> bool {
        self.features.contains(&feature)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RealizationFamily {
    ReferenceSequential,
    GenericCellLoop,
    OrderedReduction,
    MetadataOrIndexReindex,
    CpuSimd,
    CpuVectorGather,
    GpuDataParallel,
    GpuIndexed,
    GpuTreeReduction,
    TensorOrGemm,
    ExternalLibrary,
    /// Execute a witnessed graph rewrite as one composite reference route.
    ReferenceRewriteComposite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requirement {
    Target(TargetFamily),
    Feature(TargetFeature),
    KnownElementwiseAccess,
    KnownReductionAccess,
    KnownResultRank,
    ReassociationAllowed,
    Pure,
    NoObservableError,
    EvaluationOrderRelaxed,
}

impl Requirement {
    fn satisfied(self, call: &CallOp, target: &TargetCapabilities) -> bool {
        match self {
            Self::Target(family) => target.family == family,
            Self::Feature(feature) => target.has(feature),
            Self::KnownElementwiseAccess => {
                call.access == AccessFact::Known(AccessRelation::ElementwiseMap)
            }
            Self::KnownReductionAccess => {
                call.access == AccessFact::Known(AccessRelation::ReduceLeadingAxis)
            }
            Self::KnownResultRank => call.instantiation.result_rank.is_some(),
            Self::ReassociationAllowed => call.contract.allow_reassociation,
            Self::Pure => call.effect.is_pure(),
            Self::NoObservableError => !call.possible_errors.may_raise(),
            Self::EvaluationOrderRelaxed => !call.speculation.preserve_evaluation_order,
        }
    }

    fn target_only_satisfied(self, target: &TargetCapabilities) -> Option<bool> {
        match self {
            Self::Target(family) => Some(target.family == family),
            Self::Feature(feature) => Some(target.has(feature)),
            Self::KnownElementwiseAccess
            | Self::KnownReductionAccess
            | Self::KnownResultRank
            | Self::ReassociationAllowed
            | Self::Pure
            | Self::NoObservableError
            | Self::EvaluationOrderRelaxed => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionBasisLoweringCapability {
    pub basis: ExecutionBasisKind,
    pub realization: RealizationFamily,
    pub requirements: Vec<Requirement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteCompositeLoweringCapability {
    pub rule: crate::j_graph_rewrite::GraphRewriteRuleId,
    pub realization: RealizationFamily,
    /// v0 composite capabilities use target/feature requirements only.
    /// Call-dependent requirements remain unresolved until a richer rewrite
    /// semantic-proof interface is connected.
    pub requirements: Vec<Requirement>,
}

impl ExecutionBasisLoweringCapability {
    pub fn legal_for(&self, call: &CallOp, target: &TargetCapabilities) -> bool {
        self.requirements
            .iter()
            .copied()
            .all(|requirement| requirement.satisfied(call, target))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteRegionClass {
    ValueOnly,
    PureArray,
    SemanticCheck,
    RuntimeSemantic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterizedLoweringRecipe {
    pub basis: ExecutionBasisKind,
    pub realization: RealizationFamily,
    pub payload: ExecutionBasisPayload,
    pub iteration_domain: IterationDomain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteRegion {
    pub class: RouteRegionClass,
    pub operations: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BasisTargetFeasibility {
    /// At least one registered realization needs only target/feature facts and
    /// those facts are satisfied. Full operation legality may still add checks
    /// once a concrete CallOp exists.
    Supported { candidates: Vec<RealizationFamily> },
    /// Matching target families exist, but all surviving capabilities require
    /// call-dependent semantic facts (purity, access, rank, error order, ...).
    RequiresCallFacts,
    /// No registered capability survives target/feature filtering.
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteNodeTargetFeasibility {
    pub node: crate::j_graph_rewrite::RewriteNodeId,
    pub graph_basis: crate::j_graph_ir::GraphBasisKind,
    pub execution_basis: Option<ExecutionBasisKind>,
    pub feasibility: BasisTargetFeasibility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RewriteTargetFeasibilityKind {
    Supported,
    RequiresCallFacts,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteTargetFeasibility {
    pub rule: crate::j_graph_rewrite::GraphRewriteRuleId,
    pub target: TargetCapabilities,
    pub nodes: Vec<RewriteNodeTargetFeasibility>,
    /// Whole-rewrite realizations can be legal even when individual graph-basis
    /// nodes do not have standalone lowering routes.
    pub composite_candidates: Vec<RealizationFamily>,
    pub composite_requires_call_facts: bool,
    pub overall: RewriteTargetFeasibilityKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RewritePlanningState {
    TargetUnsupported,
    NeedsCallFacts,
    NeedsResourceFacts,
    ReadyForCosting,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewritePlanningReport {
    pub candidate_index: usize,
    pub rule: crate::j_graph_rewrite::GraphRewriteRuleId,
    pub target_feasibility: RewriteTargetFeasibility,
    pub resource_evaluation: crate::j_graph_resource::RewriteResourceEvaluation,
    pub state: RewritePlanningState,
    /// Mirrors the rule's proof contract. A report reaching ReadyForCosting
    /// still cannot be early-pruned unless this is true.
    pub early_pruning_allowed: bool,
}

fn execution_basis_for_graph_basis(
    basis: crate::j_graph_ir::GraphBasisKind,
) -> Option<ExecutionBasisKind> {
    use crate::j_graph_ir::GraphBasisKind as G;
    use ExecutionBasisKind as E;
    match basis {
        G::Elementwise => Some(E::Elementwise),
        G::CellApply => Some(E::CellApply),
        G::Reduce => Some(E::Reduce),
        G::Window => Some(E::WindowView),
        G::StaticReindex => Some(E::StaticReindex),
        G::DynamicGather => Some(E::Gather),
        G::Search => Some(E::LookupClassify),
        G::Structured => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteDecision {
    /// Literal/read/function-reference bookkeeping: no array kernel route needed.
    NoKernel,
    /// J-visible check is kept as its own ordered operation.
    SemanticCheck,
    /// The native planner has one or more semantically legal realization families.
    NativeExecutionBasis {
        basis: ExecutionBasisKind,
        candidates: Vec<RealizationFamily>,
    },
    /// Lack of a native candidate is a route limitation, not invalid J.
    RuntimeSemanticFallback,
}

#[derive(Clone, Debug, Default)]
pub struct LoweringRegistry {
    capabilities: Vec<ExecutionBasisLoweringCapability>,
    rewrite_capabilities: Vec<RewriteCompositeLoweringCapability>,
}

impl LoweringRegistry {
    pub fn a3_v0() -> Self {
        use ExecutionBasisKind::*;
        use RealizationFamily::*;
        use Requirement::*;
        use TargetFamily::*;
        use TargetFeature::*;

        let mut registry = Self::default();
        let mut add = |basis, realization, requirements| {
            registry
                .capabilities
                .push(ExecutionBasisLoweringCapability {
                    basis,
                    realization,
                    requirements,
                });
        };

        add(Elementwise, ReferenceSequential, vec![Target(Cpu), Pure]);
        add(
            Elementwise,
            CpuSimd,
            vec![
                Target(Cpu),
                Feature(Simd),
                KnownElementwiseAccess,
                Pure,
                NoObservableError,
                EvaluationOrderRelaxed,
            ],
        );
        add(
            Elementwise,
            GpuDataParallel,
            vec![
                Target(Gpu),
                Feature(Threads),
                KnownElementwiseAccess,
                KnownResultRank,
                Pure,
                NoObservableError,
                EvaluationOrderRelaxed,
            ],
        );

        add(IndexSpace, ReferenceSequential, vec![Target(Cpu), Pure]);
        add(
            IndexSpace,
            CpuSimd,
            vec![
                Target(Cpu),
                Feature(Simd),
                KnownResultRank,
                Pure,
                NoObservableError,
                EvaluationOrderRelaxed,
            ],
        );
        add(
            IndexSpace,
            GpuDataParallel,
            vec![
                Target(Gpu),
                Feature(Threads),
                KnownResultRank,
                Pure,
                NoObservableError,
                EvaluationOrderRelaxed,
            ],
        );

        add(CellApply, GenericCellLoop, vec![Target(Cpu), Pure]);

        add(Reduce, OrderedReduction, vec![Target(Cpu), Pure]);
        add(
            Reduce,
            CpuSimd,
            vec![
                Target(Cpu),
                Feature(Simd),
                KnownReductionAccess,
                ReassociationAllowed,
                Pure,
                NoObservableError,
                EvaluationOrderRelaxed,
            ],
        );
        add(
            Reduce,
            GpuTreeReduction,
            vec![
                Target(Gpu),
                Feature(SubgroupCollective),
                KnownReductionAccess,
                ReassociationAllowed,
                Pure,
                NoObservableError,
                EvaluationOrderRelaxed,
            ],
        );

        add(
            StaticReindex,
            MetadataOrIndexReindex,
            vec![Target(Cpu), KnownResultRank, Pure],
        );
        add(Gather, ReferenceSequential, vec![Target(Cpu), Pure]);
        add(
            Gather,
            CpuVectorGather,
            vec![
                Target(Cpu),
                Feature(IndexedMemory),
                Pure,
                NoObservableError,
                EvaluationOrderRelaxed,
            ],
        );
        add(
            Gather,
            GpuIndexed,
            vec![
                Target(Gpu),
                Feature(IndexedMemory),
                Feature(Threads),
                Pure,
                NoObservableError,
                EvaluationOrderRelaxed,
            ],
        );

        registry
            .rewrite_capabilities
            .push(RewriteCompositeLoweringCapability {
                rule: crate::j_graph_rewrite::GraphRewriteRuleId::FindViaWindowMatch,
                realization: RealizationFamily::ReferenceRewriteComposite,
                requirements: vec![Target(Cpu)],
            });

        registry
    }

    pub fn capabilities(&self) -> &[ExecutionBasisLoweringCapability] {
        &self.capabilities
    }

    pub fn basis_target_feasibility(
        &self,
        basis: ExecutionBasisKind,
        target: &TargetCapabilities,
    ) -> BasisTargetFeasibility {
        let mut supported = Vec::new();
        let mut requires_call_facts = false;

        for capability in self
            .capabilities
            .iter()
            .filter(|capability| capability.basis == basis)
        {
            let mut target_rejected = false;
            let mut unresolved = false;
            for requirement in &capability.requirements {
                match requirement.target_only_satisfied(target) {
                    Some(true) => {}
                    Some(false) => {
                        target_rejected = true;
                        break;
                    }
                    None => unresolved = true,
                }
            }
            if target_rejected {
                continue;
            }
            if unresolved {
                requires_call_facts = true;
            } else if !supported.contains(&capability.realization) {
                supported.push(capability.realization);
            }
        }

        if !supported.is_empty() {
            BasisTargetFeasibility::Supported {
                candidates: supported,
            }
        } else if requires_call_facts {
            BasisTargetFeasibility::RequiresCallFacts
        } else {
            BasisTargetFeasibility::Unsupported
        }
    }

    pub fn rewrite_candidate_target_feasibility(
        &self,
        candidate: &crate::j_graph_rewrite::GraphRewriteCandidate,
        target: &TargetCapabilities,
    ) -> RewriteTargetFeasibility {
        let mut composite_candidates = Vec::new();
        let mut composite_requires_call_facts = false;
        for capability in self
            .rewrite_capabilities
            .iter()
            .filter(|capability| capability.rule == candidate.rule)
        {
            let mut target_rejected = false;
            let mut unresolved = false;
            for requirement in &capability.requirements {
                match requirement.target_only_satisfied(target) {
                    Some(true) => {}
                    Some(false) => {
                        target_rejected = true;
                        break;
                    }
                    None => unresolved = true,
                }
            }
            if target_rejected {
                continue;
            }
            if unresolved {
                composite_requires_call_facts = true;
            } else if !composite_candidates.contains(&capability.realization) {
                composite_candidates.push(capability.realization);
            }
        }

        let mut nodes = Vec::with_capacity(candidate.replacement.nodes.len());
        let mut node_overall = RewriteTargetFeasibilityKind::Supported;

        for (index, node) in candidate.replacement.nodes.iter().enumerate() {
            let execution_basis = execution_basis_for_graph_basis(node.basis);
            let feasibility = match execution_basis {
                Some(basis) => self.basis_target_feasibility(basis, target),
                None => BasisTargetFeasibility::Unsupported,
            };
            node_overall = match (&feasibility, node_overall) {
                (BasisTargetFeasibility::Unsupported, _) => {
                    RewriteTargetFeasibilityKind::Unsupported
                }
                (_, RewriteTargetFeasibilityKind::Unsupported) => {
                    RewriteTargetFeasibilityKind::Unsupported
                }
                (BasisTargetFeasibility::RequiresCallFacts, _) => {
                    RewriteTargetFeasibilityKind::RequiresCallFacts
                }
                (_, RewriteTargetFeasibilityKind::RequiresCallFacts) => {
                    RewriteTargetFeasibilityKind::RequiresCallFacts
                }
                (BasisTargetFeasibility::Supported { .. }, _) => {
                    RewriteTargetFeasibilityKind::Supported
                }
            };
            nodes.push(RewriteNodeTargetFeasibility {
                node: crate::j_graph_rewrite::RewriteNodeId(index),
                graph_basis: node.basis,
                execution_basis,
                feasibility,
            });
        }

        let overall = if !composite_candidates.is_empty() {
            RewriteTargetFeasibilityKind::Supported
        } else if composite_requires_call_facts {
            match node_overall {
                RewriteTargetFeasibilityKind::Supported => RewriteTargetFeasibilityKind::Supported,
                _ => RewriteTargetFeasibilityKind::RequiresCallFacts,
            }
        } else {
            node_overall
        };

        RewriteTargetFeasibility {
            rule: candidate.rule,
            target: target.clone(),
            nodes,
            composite_candidates,
            composite_requires_call_facts,
            overall,
        }
    }

    pub fn rewrite_planning_reports(
        &self,
        analysis: &CompilationAnalysis,
        target: &TargetCapabilities,
    ) -> Vec<RewritePlanningReport> {
        debug_assert_eq!(
            analysis.graph_rewrites.len(),
            analysis.graph_rewrite_resources.len(),
            "every graph rewrite must have exactly one resource evaluation"
        );
        analysis
            .graph_rewrites
            .iter()
            .zip(analysis.graph_rewrite_resources.iter())
            .enumerate()
            .map(|(candidate_index, (candidate, resource_evaluation))| {
                let target_feasibility =
                    self.rewrite_candidate_target_feasibility(candidate, target);
                let resource_comparison = resource_evaluation.comparison();
                let resource_incomplete = resource_evaluation
                    .source
                    .has_unknown_implementation_resource
                    || resource_evaluation
                        .replacement
                        .has_unknown_implementation_resource
                    || matches!(
                        resource_comparison.unfused_internal_traffic,
                        crate::j_graph_resource::ResourceMetricOrdering::Incomparable
                    )
                    || matches!(
                        resource_comparison.elidable_internal_traffic,
                        crate::j_graph_resource::ResourceMetricOrdering::Incomparable
                    );

                let state = match target_feasibility.overall {
                    RewriteTargetFeasibilityKind::Unsupported => {
                        RewritePlanningState::TargetUnsupported
                    }
                    RewriteTargetFeasibilityKind::RequiresCallFacts => {
                        RewritePlanningState::NeedsCallFacts
                    }
                    RewriteTargetFeasibilityKind::Supported if resource_incomplete => {
                        RewritePlanningState::NeedsResourceFacts
                    }
                    RewriteTargetFeasibilityKind::Supported => {
                        RewritePlanningState::ReadyForCosting
                    }
                };

                RewritePlanningReport {
                    candidate_index,
                    rule: candidate.rule,
                    target_feasibility,
                    resource_evaluation: resource_evaluation.clone(),
                    state,
                    early_pruning_allowed: resource_evaluation.early_pruning_allowed,
                }
            })
            .collect()
    }

    /// Execute the v0 whole-rewrite reference realization registered for a
    /// candidate. This is a validation/bootstrap route, not a cost-based
    /// selection policy and not a standalone WindowView kernel API.
    pub fn execute_reference_rewrite(
        &self,
        analysis: &CompilationAnalysis,
        plan: &Plan,
        candidate_index: usize,
        target: &TargetCapabilities,
        source_values: &[Option<Value>],
    ) -> crate::Result<Value> {
        let candidate = analysis
            .graph_rewrites
            .get(candidate_index)
            .ok_or(Error::Index)?;
        let feasibility = self.rewrite_candidate_target_feasibility(candidate, target);
        if !feasibility
            .composite_candidates
            .contains(&RealizationFamily::ReferenceRewriteComposite)
        {
            return Err(Error::Unsupported(
                "reference rewrite composite is not legal for this target".into(),
            ));
        }

        let expansion = crate::expansion::for_graph_rewrite(plan, candidate)
            .map_err(|message| Error::Unsupported(message.into()))?;
        crate::expansion::execute_reference(&expansion, source_values)
    }

    /// Return all legal candidates.  This function intentionally does not rank
    /// them; cost/preference belongs to a later CostProfile/planner layer.
    pub fn legal_candidates(
        &self,
        basis: ExecutionBasisKind,
        call: &CallOp,
        target: &TargetCapabilities,
    ) -> Vec<RealizationFamily> {
        self.capabilities
            .iter()
            .filter(|capability| capability.basis == basis)
            .filter(|capability| capability.legal_for(call, target))
            .map(|capability| capability.realization)
            .collect()
    }

    pub fn route_operation(
        &self,
        operation: &Operation,
        target: &TargetCapabilities,
    ) -> RouteDecision {
        match &operation.kind {
            OpKind::Literal(_) | OpKind::ReadNoun { .. } | OpKind::VerbReference(_) => {
                RouteDecision::NoKernel
            }
            OpKind::SemanticCheck(_) => RouteDecision::SemanticCheck,
            OpKind::SemanticCall(_) => RouteDecision::RuntimeSemanticFallback,
            OpKind::Basis { kind, call, .. } => {
                let candidates = self.legal_candidates(*kind, call, target);
                if candidates.is_empty() {
                    RouteDecision::RuntimeSemanticFallback
                } else {
                    RouteDecision::NativeExecutionBasis {
                        basis: *kind,
                        candidates,
                    }
                }
            }
        }
    }

    pub fn recipes_for_operation(
        &self,
        operation: &Operation,
        target: &TargetCapabilities,
    ) -> Vec<ParameterizedLoweringRecipe> {
        let OpKind::Basis {
            kind,
            payload,
            call,
        } = &operation.kind
        else {
            return Vec::new();
        };
        self.legal_candidates(*kind, call, target)
            .into_iter()
            .map(|realization| ParameterizedLoweringRecipe {
                basis: *kind,
                realization,
                payload: payload.clone(),
                iteration_domain: call.iteration_domain.clone(),
            })
            .collect()
    }

    pub fn partition_plan(&self, plan: &Plan, target: &TargetCapabilities) -> Vec<RouteRegion> {
        fn class(decision: &RouteDecision) -> RouteRegionClass {
            match decision {
                RouteDecision::NoKernel => RouteRegionClass::ValueOnly,
                RouteDecision::SemanticCheck => RouteRegionClass::SemanticCheck,
                RouteDecision::NativeExecutionBasis { .. } => RouteRegionClass::PureArray,
                RouteDecision::RuntimeSemanticFallback => RouteRegionClass::RuntimeSemantic,
            }
        }

        let mut regions: Vec<RouteRegion> = Vec::new();
        for (index, operation) in plan.operations.iter().enumerate() {
            let decision = self.route_operation(operation, target);
            let next_class = class(&decision);
            if let Some(last) = regions.last_mut() {
                if last.class == next_class && last.operations.end == index {
                    last.operations.end += 1;
                    continue;
                }
            }
            regions.push(RouteRegion {
                class: next_class,
                operations: index..index + 1,
            });
        }
        regions
    }
}

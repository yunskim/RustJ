//! Target-aware lowering legality for logical basis operations.
//!
//! This module deliberately answers only "is this realization family legal?".
//! Cost ranking, scheduling, bufferization and concrete kernel selection belong
//! to later planning stages.

use crate::{
    analysis::{AccessFact, AccessRelation, BasisKind},
    logical_ir::{CallOp, OpKind, Operation, Plan},
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasisLoweringCapability {
    pub basis: BasisKind,
    pub realization: RealizationFamily,
    pub requirements: Vec<Requirement>,
}

impl BasisLoweringCapability {
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
pub struct RouteRegion {
    pub class: RouteRegionClass,
    pub operations: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteDecision {
    /// Literal/read/function-reference bookkeeping: no array kernel route needed.
    NoKernel,
    /// J-visible check is kept as its own ordered operation.
    SemanticCheck,
    /// The native planner has one or more semantically legal realization families.
    NativeBasis {
        basis: BasisKind,
        candidates: Vec<RealizationFamily>,
    },
    /// Lack of a native candidate is a route limitation, not invalid J.
    RuntimeSemanticFallback,
}

#[derive(Clone, Debug, Default)]
pub struct LoweringRegistry {
    capabilities: Vec<BasisLoweringCapability>,
}

impl LoweringRegistry {
    pub fn a3_v0() -> Self {
        use BasisKind::*;
        use RealizationFamily::*;
        use Requirement::*;
        use TargetFamily::*;
        use TargetFeature::*;

        let mut registry = Self::default();
        let mut add = |basis, realization, requirements| {
            registry.capabilities.push(BasisLoweringCapability {
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
    }

    pub fn capabilities(&self) -> &[BasisLoweringCapability] {
        &self.capabilities
    }

    /// Return all legal candidates.  This function intentionally does not rank
    /// them; cost/preference belongs to a later CostProfile/planner layer.
    pub fn legal_candidates(
        &self,
        basis: BasisKind,
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
                    RouteDecision::NativeBasis {
                        basis: *kind,
                        candidates,
                    }
                }
            }
        }
    }

    pub fn partition_plan(
        &self,
        plan: &Plan,
        target: &TargetCapabilities,
    ) -> Vec<RouteRegion> {
        fn class(decision: &RouteDecision) -> RouteRegionClass {
            match decision {
                RouteDecision::NoKernel => RouteRegionClass::ValueOnly,
                RouteDecision::SemanticCheck => RouteRegionClass::SemanticCheck,
                RouteDecision::NativeBasis { .. } => RouteRegionClass::PureArray,
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

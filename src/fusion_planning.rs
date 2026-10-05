//! Downstream, target-dependent readiness inspection. Source-basis feasibility
//! is never treated as fused-call legality, kernel support or profitability.

use crate::{
    j_graph_fusion::{FusionAnalysis, FusionRegistry, FusionRuleId, ProofObligation, TargetQuery},
    j_graph_ir::{GraphBasisKind, NodeKind, Plan, ValueId},
    j_graph_work_depth::{FusionWorkDepthBatch, WorkDepthAnalysis},
    lowering::{BasisTargetFeasibility, LoweringRegistry, TargetCapabilities},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FusionReadinessState {
    AwaitingSemanticProofs,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceBasisFeasibility {
    pub value: ValueId,
    /// Target-only metadata queries, not resolved-call lowering legality.
    pub layers: Vec<(GraphBasisKind, BasisTargetFeasibility)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusionReadinessReport {
    pub candidate_index: usize,
    pub rule: FusionRuleId,
    pub source_basis_feasibility: Vec<SourceBasisFeasibility>,
    pub unresolved_obligations: Vec<ProofObligation>,
    pub fused_target_query: TargetQuery,
    pub state: FusionReadinessState,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusionReadinessAnalysis {
    pub target: TargetCapabilities,
    pub fusion: FusionAnalysis,
    pub work_depth: WorkDepthAnalysis,
    pub comparisons: FusionWorkDepthBatch,
    pub reports: Vec<FusionReadinessReport>,
}

impl FusionReadinessAnalysis {
    pub fn from_plan(
        plan: &Plan,
        rules: &FusionRegistry,
        lowering: &LoweringRegistry,
        target: &TargetCapabilities,
    ) -> Result<Self, String> {
        let fusion = plan.fusion_analysis(rules)?;
        let work_depth = plan.work_depth_analysis()?;
        let comparisons = work_depth.fusion_envelope_batch(&fusion, rules, plan)?;
        let reports = fusion
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let rule = rules
                    .rules()
                    .iter()
                    .find(|r| r.id == candidate.rule)
                    .expect("verified registry rule");
                let source_basis_feasibility = candidate
                    .replacement
                    .operations
                    .iter()
                    .map(|value| {
                        let NodeKind::Apply { basis, .. } = &plan.nodes[value.0].kind else {
                            unreachable!()
                        };
                        let layers = basis
                            .layers
                            .iter()
                            .map(|kind| {
                                let query = crate::lowering::execution_basis_for_graph_basis(*kind)
                                    .map_or(BasisTargetFeasibility::Unsupported, |basis| {
                                        lowering.basis_target_feasibility(basis, target)
                                    });
                                (*kind, query)
                            })
                            .collect();
                        SourceBasisFeasibility {
                            value: *value,
                            layers,
                        }
                    })
                    .collect();
                FusionReadinessReport {
                    candidate_index: index,
                    rule: candidate.rule,
                    source_basis_feasibility,
                    unresolved_obligations: rule.obligations.clone(),
                    fused_target_query: TargetQuery::DeferredUntilLegality,
                    state: FusionReadinessState::AwaitingSemanticProofs,
                    selected: false,
                }
            })
            .collect();
        Ok(Self {
            target: target.clone(),
            fusion,
            work_depth,
            comparisons,
            reports,
        })
    }

    pub fn verify(
        &self,
        plan: &Plan,
        rules: &FusionRegistry,
        lowering: &LoweringRegistry,
        target: &TargetCapabilities,
    ) -> Result<(), String> {
        self.fusion.verify(plan, rules)?;
        self.work_depth.verify(plan)?;
        self.comparisons
            .verify(&self.work_depth, &self.fusion, rules, plan)?;
        if *self != Self::from_plan(plan, rules, lowering, target)? {
            return Err("fusion readiness differs from source/registry/target proof state".into());
        }
        Ok(())
    }
}

impl LoweringRegistry {
    pub fn fusion_readiness(
        &self,
        plan: &Plan,
        rules: &FusionRegistry,
        target: &TargetCapabilities,
    ) -> Result<FusionReadinessAnalysis, String> {
        FusionReadinessAnalysis::from_plan(plan, rules, self, target)
    }
}

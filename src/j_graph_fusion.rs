//! Analysis-only fusion registry and source-operation envelopes.
//! An envelope preserves existing operations and provenance. It is not a
//! fused kernel, equivalence proof, target capability or selected partition.

use crate::{
    j_graph_composition::{CompositionAnalysis, CompositionRelation},
    j_graph_ir::{GraphBasisKind, GraphFacts, NodeKind, Plan, RegionId, ValueId},
    j_graph_rewrite::GraphRewriteProvenance,
    j_graph_scan::{ScanAnalysis, ScanCandidate},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FusionRuleId {
    MapMap,
    MapReduce,
    MapScan,
    CommonInputMaps,
}

impl FusionRuleId {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::MapMap => "fusion.map-map",
            Self::MapReduce => "fusion.map-reduce",
            Self::MapScan => "fusion.map-scan",
            Self::CommonInputMaps => "fusion.common-input-maps",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FusionPattern {
    VerticalMapMap,
    VerticalMapReduce,
    VerticalMapScan,
    HorizontalMaps,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofObligation {
    RankCellAssembly,
    NumericPolicy,
    ObservableEffectErrorOrder,
    FanoutAndRetention,
    ResourceWorkDepth,
    TargetCapability,
}

pub const OBLIGATIONS: &[ProofObligation] = &[
    ProofObligation::RankCellAssembly,
    ProofObligation::NumericPolicy,
    ProofObligation::ObservableEffectErrorOrder,
    ProofObligation::FanoutAndRetention,
    ProofObligation::ResourceWorkDepth,
    ProofObligation::TargetCapability,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplacementTemplate {
    /// Keep the source operation subgraph, input slots, outputs and order. A
    /// later legality/realization pass may propose an executable replacement.
    OrderedSourceEnvelope,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetQuery {
    DeferredUntilLegality,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusionRuleSchema {
    pub id: FusionRuleId,
    pub version: u32,
    pub pattern: FusionPattern,
    pub replacement: ReplacementTemplate,
    pub obligations: Vec<ProofObligation>,
    pub target_query: TargetQuery,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusionRegistry {
    rules: Vec<FusionRuleSchema>,
}

fn pattern(id: FusionRuleId) -> FusionPattern {
    match id {
        FusionRuleId::MapMap => FusionPattern::VerticalMapMap,
        FusionRuleId::MapReduce => FusionPattern::VerticalMapReduce,
        FusionRuleId::MapScan => FusionPattern::VerticalMapScan,
        FusionRuleId::CommonInputMaps => FusionPattern::HorizontalMaps,
    }
}

impl FusionRegistry {
    pub fn new(rules: Vec<FusionRuleSchema>) -> Result<Self, String> {
        let mut ids = std::collections::HashSet::new();
        let mut patterns = std::collections::HashSet::new();
        for rule in &rules {
            if rule.version != 1 {
                return Err("unsupported fusion rule version".into());
            }
            if !ids.insert(rule.id) || !patterns.insert(rule.pattern) {
                return Err("duplicate/conflicting fusion rule identity or pattern".into());
            }
            if rule.pattern != pattern(rule.id) || rule.obligations != OBLIGATIONS {
                return Err("fusion rule schema lacks its declared pattern/proof contract".into());
            }
        }
        Ok(Self { rules })
    }

    pub fn rules(&self) -> &[FusionRuleSchema] {
        &self.rules
    }
}

impl Default for FusionRegistry {
    fn default() -> Self {
        Self::new(
            [
                FusionRuleId::MapMap,
                FusionRuleId::MapReduce,
                FusionRuleId::MapScan,
                FusionRuleId::CommonInputMaps,
            ]
            .into_iter()
            .map(|id| FusionRuleSchema {
                id,
                version: 1,
                pattern: pattern(id),
                replacement: ReplacementTemplate::OrderedSourceEnvelope,
                obligations: OBLIGATIONS.to_vec(),
                target_query: TargetQuery::DeferredUntilLegality,
            })
            .collect(),
        )
        .expect("built-in fusion research schemas are valid")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FanoutWitness {
    pub value: ValueId,
    pub source_uses: usize,
    pub internal_occurrences: usize,
    pub external_uses: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusionEnvelope {
    /// Canonical source order, including entire source-operation semantics.
    pub operations: Vec<ValueId>,
    pub inputs: Vec<ValueId>,
    pub outputs: Vec<ValueId>,
    pub retained_values: Vec<ValueId>,
    /// No shared producer is copied or discarded by discovery.
    pub fanout: Vec<FanoutWitness>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FusionLegality {
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusionCandidate {
    pub rule: FusionRuleId,
    pub version: u32,
    pub region: Option<RegionId>,
    /// Reuse the existing witnessed-rewrite provenance carrier, without
    /// borrowing E.'s proof for unrelated fusion identities.
    pub provenance: Vec<GraphRewriteProvenance>,
    pub call_facts: Vec<(ValueId, GraphFacts)>,
    pub replacement: FusionEnvelope,
    pub scan_identity: Option<ScanCandidate>,
    pub legality: FusionLegality,
    /// Resource/WorkDepth transfer and profitability are not yet proved.
    pub resource_transfer_proven: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusionAnalysis {
    pub composition_witness: CompositionAnalysis,
    pub scan_witness: ScanAnalysis,
    pub candidates: Vec<FusionCandidate>,
}

impl FusionAnalysis {
    pub fn from_plan(plan: &Plan, registry: &FusionRegistry) -> Result<Self, String> {
        let composition = plan.composition_analysis()?;
        let scan = plan.scan_analysis()?;
        Ok(Self::derive(plan, registry, composition, scan))
    }

    pub fn verify(&self, plan: &Plan, registry: &FusionRegistry) -> Result<(), String> {
        self.composition_witness.verify(plan)?;
        self.scan_witness.verify(plan)?;
        let expected = Self::derive(
            plan,
            registry,
            self.composition_witness.clone(),
            self.scan_witness.clone(),
        );
        if *self != expected {
            return Err("fusion candidate/witness differs from registered source topology".into());
        }
        Ok(())
    }

    fn derive(
        plan: &Plan,
        registry: &FusionRegistry,
        composition: CompositionAnalysis,
        scan: ScanAnalysis,
    ) -> Self {
        let is_basis = |value: ValueId, kind| {
            matches!(&plan.nodes[value.0].kind,
            NodeKind::Apply { basis, .. } if basis.layers == [kind])
        };
        let mut candidates = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for rule in registry.rules() {
            for relation in &composition.relations {
                let (operations, region, identity) = match (rule.pattern, relation) {
                    (
                        FusionPattern::VerticalMapMap
                        | FusionPattern::VerticalMapReduce
                        | FusionPattern::VerticalMapScan,
                        CompositionRelation::Vertical {
                            producer, consumer, ..
                        },
                    ) if is_basis(*producer, GraphBasisKind::Elementwise) => {
                        let identity = scan
                            .candidates
                            .iter()
                            .find(|c| c.source == *consumer && c.input == *producer);
                        let matches = match rule.pattern {
                            FusionPattern::VerticalMapMap => {
                                is_basis(*consumer, GraphBasisKind::Elementwise)
                            }
                            FusionPattern::VerticalMapReduce => {
                                is_basis(*consumer, GraphBasisKind::Reduce)
                            }
                            FusionPattern::VerticalMapScan => identity.is_some(),
                            _ => false,
                        };
                        if !matches {
                            continue;
                        }
                        (
                            vec![*producer, *consumer],
                            None,
                            if rule.pattern == FusionPattern::VerticalMapScan {
                                identity.cloned()
                            } else {
                                None
                            },
                        )
                    }
                    (
                        FusionPattern::HorizontalMaps,
                        CompositionRelation::HorizontalCandidate {
                            region,
                            branch_results,
                            ..
                        },
                    ) if branch_results.len() == 2
                        && branch_results
                            .iter()
                            .all(|v| is_basis(*v, GraphBasisKind::Elementwise)) =>
                    {
                        (branch_results.clone(), Some(*region), None)
                    }
                    _ => continue,
                };
                // A repeated dyadic input slot is two occurrences, but one
                // envelope candidate. Keep the occurrences in fan-out counts.
                if !seen.insert((rule.id, operations.clone(), region)) {
                    continue;
                }
                candidates.push(candidate(
                    plan,
                    rule,
                    operations,
                    region,
                    identity,
                    &composition.use_counts,
                ));
            }
        }
        Self {
            composition_witness: composition,
            scan_witness: scan,
            candidates,
        }
    }
}

fn candidate(
    plan: &Plan,
    rule: &FusionRuleSchema,
    operations: Vec<ValueId>,
    region: Option<RegionId>,
    scan_identity: Option<ScanCandidate>,
    use_counts: &[usize],
) -> FusionCandidate {
    let mut internal = std::collections::HashMap::<ValueId, usize>::new();
    let mut inputs = Vec::new();
    for value in &operations {
        let NodeKind::Apply { left, right, .. } = &plan.nodes[value.0].kind else {
            unreachable!()
        };
        for input in left.iter().chain(std::iter::once(right)) {
            *internal.entry(*input).or_default() += 1;
            if !operations.contains(input) && !inputs.contains(input) {
                inputs.push(*input);
            }
        }
    }
    let fanout: Vec<_> = operations
        .iter()
        .map(|value| FanoutWitness {
            value: *value,
            source_uses: use_counts[value.0],
            internal_occurrences: internal.get(value).copied().unwrap_or(0),
            external_uses: use_counts[value.0] - internal.get(value).copied().unwrap_or(0),
        })
        .collect();
    let retained_values: Vec<_> = fanout
        .iter()
        .filter(|f| f.external_uses > 0)
        .map(|f| f.value)
        .collect();
    let provenance = operations
        .iter()
        .map(|value| {
            let NodeKind::Apply { basis, .. } = &plan.nodes[value.0].kind else {
                unreachable!()
            };
            GraphRewriteProvenance {
                source_value: *value,
                source_span: plan.nodes[value.0].span.clone(),
                source_basis: basis.clone(),
            }
        })
        .collect();
    let call_facts = inputs
        .iter()
        .chain(operations.iter())
        .map(|value| (*value, plan.nodes[value.0].facts.clone()))
        .collect();
    FusionCandidate {
        rule: rule.id,
        version: rule.version,
        region,
        provenance,
        call_facts,
        replacement: FusionEnvelope {
            operations,
            inputs,
            outputs: retained_values.clone(),
            retained_values,
            fanout,
        },
        scan_identity,
        legality: FusionLegality::Unknown,
        resource_transfer_proven: false,
        selected: false,
    }
}

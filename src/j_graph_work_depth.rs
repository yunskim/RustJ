//! Symbolic operation work and ordered dependency depth, separate from memory
//! resources and wall-clock cost. These are logical models, not schedules.

use crate::{
    j_graph_composition::CompositionAnalysis,
    j_graph_ir::{GraphForm, NodeKind, Plan, RegionId, ValueId},
    j_graph_scan::{ScanAnalysis, ScanCandidate},
    semantic::FunctionHead,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkDepthExprId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    Work,
    Depth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtentKind {
    Atoms,
    LeadingItems,
    ItemAtoms,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorComponent {
    DispatchChecks,
    Element,
    ReducerPair,
    EmptyIdentity,
    ResultAssembly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnknownReason {
    OpaqueOperation,
    NestedCellOrWindow,
    FinalAssignment,
    FusionTransferUnproven,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkDepthExprNode {
    Constant(u64),
    Unknown {
        source: ValueId,
        reason: UnknownReason,
    },
    Extent {
        source: ValueId,
        kind: ExtentKind,
    },
    OperatorCost {
        source: ValueId,
        metric: Metric,
        component: OperatorComponent,
    },
    Sum(Vec<WorkDepthExprId>),
    /// Available for a future legally independent candidate; the ordered
    /// baseline never emits Max merely because source syntax is a fork.
    Max(Vec<WorkDepthExprId>),
    Product(WorkDepthExprId, WorkDepthExprId),
    Predecessors(WorkDepthExprId),
    IfEmpty {
        input: ValueId,
        when_empty: WorkDepthExprId,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkDepthExprGraph {
    pub nodes: Vec<WorkDepthExprNode>,
}

pub trait OperatorCostModel {
    /// Abstract operation weights, not nanoseconds, launches or traffic bytes.
    /// None retains unknown operator cost rather than assuming unit cost.
    fn cost(&self, source: ValueId, metric: Metric, component: OperatorComponent) -> Option<u64>;
}

impl WorkDepthExprGraph {
    fn push(&mut self, node: WorkDepthExprNode) -> WorkDepthExprId {
        let id = WorkDepthExprId(self.nodes.len());
        self.nodes.push(node);
        id
    }
    fn constant(&mut self, n: u64) -> WorkDepthExprId {
        self.push(WorkDepthExprNode::Constant(n))
    }
    fn extent(&mut self, source: ValueId, kind: ExtentKind) -> WorkDepthExprId {
        self.push(WorkDepthExprNode::Extent { source, kind })
    }
    fn cost(
        &mut self,
        source: ValueId,
        metric: Metric,
        component: OperatorComponent,
    ) -> WorkDepthExprId {
        self.push(WorkDepthExprNode::OperatorCost {
            source,
            metric,
            component,
        })
    }
    fn product(&mut self, a: WorkDepthExprId, b: WorkDepthExprId) -> WorkDepthExprId {
        self.push(WorkDepthExprNode::Product(a, b))
    }
    fn sum(&mut self, ids: Vec<WorkDepthExprId>) -> WorkDepthExprId {
        self.push(WorkDepthExprNode::Sum(ids))
    }

    pub fn verify(&self, plan: &Plan) -> Result<(), String> {
        for (i, node) in self.nodes.iter().enumerate() {
            let (source, inputs) = match node {
                WorkDepthExprNode::Unknown { source, .. }
                | WorkDepthExprNode::Extent { source, .. }
                | WorkDepthExprNode::OperatorCost { source, .. } => (Some(*source), vec![]),
                WorkDepthExprNode::IfEmpty { input, when_empty } => {
                    (Some(*input), vec![*when_empty])
                }
                WorkDepthExprNode::Sum(ids) | WorkDepthExprNode::Max(ids) => (None, ids.clone()),
                WorkDepthExprNode::Product(a, b) => (None, vec![*a, *b]),
                WorkDepthExprNode::Predecessors(a) => (None, vec![*a]),
                WorkDepthExprNode::Constant(_) => (None, vec![]),
            };
            if source.is_some_and(|v| v.0 >= plan.nodes.len()) || inputs.iter().any(|v| v.0 >= i) {
                return Err("invalid Work/Depth provenance or expression dependency".into());
            }
        }
        Ok(())
    }

    pub fn evaluate(
        &self,
        expression: WorkDepthExprId,
        plan: &Plan,
        costs: &impl OperatorCostModel,
    ) -> Option<u64> {
        self.verify(plan).ok()?;
        let mut values: Vec<Option<u64>> = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let value = match node {
                WorkDepthExprNode::Constant(n) => Some(*n),
                WorkDepthExprNode::Unknown { .. } => None,
                WorkDepthExprNode::Extent { source, kind } => extent(plan, *source, *kind),
                WorkDepthExprNode::OperatorCost {
                    source,
                    metric,
                    component,
                } => costs.cost(*source, *metric, *component),
                WorkDepthExprNode::Sum(ids) => ids
                    .iter()
                    .try_fold(0u64, |n, id| n.checked_add(values[id.0]?)),
                WorkDepthExprNode::Max(ids) => ids
                    .iter()
                    .try_fold(0u64, |n, id| Some(n.max(values[id.0]?))),
                WorkDepthExprNode::Product(a, b) => values[a.0]
                    .zip(values[b.0])
                    .and_then(|(a, b)| a.checked_mul(b)),
                WorkDepthExprNode::Predecessors(a) => values[a.0].map(|n| n.saturating_sub(1)),
                WorkDepthExprNode::IfEmpty { input, when_empty } => {
                    extent(plan, *input, ExtentKind::LeadingItems).and_then(|n| {
                        if n == 0 {
                            values[when_empty.0]
                        } else {
                            Some(0)
                        }
                    })
                }
            };
            values.push(value);
        }
        values.get(expression.0).copied().flatten()
    }
}

fn extent(plan: &Plan, source: ValueId, kind: ExtentKind) -> Option<u64> {
    let facts = &plan.nodes.get(source.0)?.facts;
    let shape = facts.shape.as_ref()?;
    if facts.rank != Some(shape.len()) {
        return None;
    }
    let dims: &[usize] = match kind {
        ExtentKind::LeadingItems => return u64::try_from(shape.first().copied().unwrap_or(1)).ok(),
        ExtentKind::Atoms => shape,
        ExtentKind::ItemAtoms => {
            if shape.is_empty() {
                &[]
            } else {
                &shape[1..]
            }
        }
    };
    dims.iter()
        .try_fold(1u64, |n, d| n.checked_mul(u64::try_from(*d).ok()?))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkDepthExpr {
    pub work: WorkDepthExprId,
    pub depth: WorkDepthExprId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionWorkDepth {
    pub region: RegionId,
    /// Each original applied operation once, in canonical source order.
    pub operations: Vec<ValueId>,
    pub expressions: WorkDepthExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderedScanModel {
    pub identity: ScanCandidate,
    pub expressions: WorkDepthExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkDepthAnalysis {
    pub schema_version: u32,
    /// Ordered successful-path logical model, not a failed trace or schedule.
    pub successful_path_model: bool,
    pub applied_operations: Vec<ValueId>,
    pub expressions: WorkDepthExprGraph,
    pub nodes: Vec<WorkDepthExpr>,
    pub regions: Vec<RegionWorkDepth>,
    pub total: WorkDepthExpr,
    pub ordered_scan_models: Vec<OrderedScanModel>,
    pub composition_witness: CompositionAnalysis,
    pub scan_witness: ScanAnalysis,
}

impl WorkDepthAnalysis {
    pub fn from_plan(plan: &Plan) -> Result<Self, String> {
        Ok(Self::derive(
            plan,
            plan.composition_analysis()?,
            plan.scan_analysis()?,
        ))
    }
    pub fn verify(&self, plan: &Plan) -> Result<(), String> {
        self.expressions.verify(plan)?;
        self.composition_witness.verify(plan)?;
        self.scan_witness.verify(plan)?;
        if *self
            != Self::derive(
                plan,
                self.composition_witness.clone(),
                self.scan_witness.clone(),
            )
        {
            return Err("Work/Depth model differs from verified source topology/witnesses".into());
        }
        Ok(())
    }
    fn derive(plan: &Plan, composition: CompositionAnalysis, scan: ScanAnalysis) -> Self {
        let mut expressions = WorkDepthExprGraph::default();
        let mut nodes = Vec::new();
        for (i, node) in plan.nodes.iter().enumerate() {
            let source = ValueId(i);
            let mut build = |metric| match &node.kind {
                NodeKind::Apply {
                    function,
                    form,
                    contract,
                    right,
                    ..
                } => {
                    let body = match form {
                        GraphForm::Atomic
                            if matches!(function.head, FunctionHead::PrimitiveVerb(_))
                                && contract.iteration
                                    == crate::j_graph_ir::IterationContract::Elementwise =>
                        {
                            let n = expressions.extent(source, ExtentKind::Atoms);
                            let cost = expressions.cost(source, metric, OperatorComponent::Element);
                            expressions.product(n, cost)
                        }
                        GraphForm::Reduce { operand }
                            if matches!(operand.head, FunctionHead::PrimitiveVerb(id)
                            if crate::contracts::for_primitive(id, crate::contracts::Valence::Dyad).class
                                == crate::contracts::OperationClass::Map) =>
                        {
                            let n = expressions.extent(*right, ExtentKind::LeadingItems);
                            let pairs = expressions.push(WorkDepthExprNode::Predecessors(n));
                            let width = expressions.extent(*right, ExtentKind::ItemAtoms);
                            let pair_atoms = expressions.product(pairs, width);
                            let cost =
                                expressions.cost(source, metric, OperatorComponent::ReducerPair);
                            let reducer = expressions.product(pair_atoms, cost);
                            let identity =
                                expressions.cost(source, metric, OperatorComponent::EmptyIdentity);
                            let empty = expressions.push(WorkDepthExprNode::IfEmpty {
                                input: *right,
                                when_empty: identity,
                            });
                            expressions.sum(vec![reducer, empty])
                        }
                        _ => {
                            return expressions.push(WorkDepthExprNode::Unknown {
                                source,
                                reason: if matches!(
                                    form,
                                    GraphForm::Rank { .. } | GraphForm::PrefixInfix { .. }
                                ) {
                                    UnknownReason::NestedCellOrWindow
                                } else {
                                    UnknownReason::OpaqueOperation
                                },
                            });
                        }
                    };
                    let dispatch =
                        expressions.cost(source, metric, OperatorComponent::DispatchChecks);
                    expressions.sum(vec![dispatch, body])
                }
                _ => expressions.constant(0),
            };
            let work = build(Metric::Work);
            let depth = build(Metric::Depth);
            nodes.push(WorkDepthExpr { work, depth });
        }
        let mut regions = Vec::new();
        for (i, region) in plan.regions.iter().enumerate() {
            let inputs: std::collections::HashSet<_> = region.inputs.iter().copied().collect();
            let mut seen = std::collections::BTreeSet::new();
            let mut stack = vec![region.result];
            while let Some(value) = stack.pop() {
                if inputs.contains(&value) || seen.contains(&value.0) {
                    continue;
                }
                if let NodeKind::Apply { left, right, .. } = &plan.nodes[value.0].kind {
                    seen.insert(value.0);
                    stack.push(*right);
                    stack.extend(left);
                }
            }
            let operations: Vec<_> = seen.into_iter().map(ValueId).collect();
            let work = expressions.sum(operations.iter().map(|v| nodes[v.0].work).collect());
            let depth = expressions.sum(operations.iter().map(|v| nodes[v.0].depth).collect());
            regions.push(RegionWorkDepth {
                region: RegionId(i),
                operations,
                expressions: WorkDepthExpr { work, depth },
            });
        }
        let mut work_ids: Vec<_> = nodes.iter().map(|n| n.work).collect();
        let mut depth_ids: Vec<_> = nodes.iter().map(|n| n.depth).collect();
        if let Some(write) = &plan.write {
            let unknown = expressions.push(WorkDepthExprNode::Unknown {
                source: write.value,
                reason: UnknownReason::FinalAssignment,
            });
            work_ids.push(unknown);
            depth_ids.push(unknown);
        }
        let total = WorkDepthExpr {
            work: expressions.sum(work_ids),
            depth: expressions.sum(depth_ids),
        };
        let mut ordered_scan_models = Vec::new();
        for identity in &scan.candidates {
            let source = identity.source;
            let mut build = |metric| {
                // This is the witnessed ordered Scan identity model, not the
                // source Window implementation or an executed Rust algorithm.
                let pairs = expressions.constant(identity.contract.items.saturating_sub(1) as u64);
                let width = expressions.extent(identity.input, ExtentKind::ItemAtoms);
                let pair_atoms = expressions.product(pairs, width);
                let cost = expressions.cost(source, metric, OperatorComponent::ReducerPair);
                let reducer = expressions.product(pair_atoms, cost);
                let atoms = expressions.constant(identity.contract.atoms as u64);
                let assembly = expressions.cost(source, metric, OperatorComponent::ResultAssembly);
                let assembly = expressions.product(atoms, assembly);
                let dispatch = expressions.cost(source, metric, OperatorComponent::DispatchChecks);
                expressions.sum(vec![dispatch, reducer, assembly])
            };
            let work = build(Metric::Work);
            let depth = build(Metric::Depth);
            ordered_scan_models.push(OrderedScanModel {
                identity: identity.clone(),
                expressions: WorkDepthExpr { work, depth },
            });
        }
        Self {
            schema_version: 1,
            successful_path_model: true,
            applied_operations: plan
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(i, n)| matches!(n.kind, NodeKind::Apply { .. }).then_some(ValueId(i)))
                .collect(),
            expressions,
            nodes,
            regions,
            total,
            ordered_scan_models,
            composition_witness: composition,
            scan_witness: scan,
        }
    }

    /// Evaluate only the preserved source envelope. An unproved transformed
    /// replacement stays Unknown, never an assumed cheaper/faster program.
    pub fn fusion_envelope_model(
        &self,
        fusion: &crate::j_graph_fusion::FusionAnalysis,
        registry: &crate::j_graph_fusion::FusionRegistry,
        candidate_index: usize,
        plan: &Plan,
    ) -> Result<FusionWorkDepthModel, String> {
        self.verify(plan)?;
        fusion.verify(plan, registry)?;
        let candidate = fusion
            .candidates
            .get(candidate_index)
            .ok_or("invalid fusion candidate index")?;
        let operations = &candidate.replacement.operations;
        let mut expressions = self.expressions.clone();
        let source = WorkDepthExpr {
            work: expressions.sum(operations.iter().map(|v| self.nodes[v.0].work).collect()),
            depth: expressions.sum(operations.iter().map(|v| self.nodes[v.0].depth).collect()),
        };
        let unknown = expressions.push(WorkDepthExprNode::Unknown {
            source: *operations.last().ok_or("empty fusion envelope")?,
            reason: UnknownReason::FusionTransferUnproven,
        });
        Ok(FusionWorkDepthModel {
            candidate_index,
            operations: operations.clone(),
            retained_values: candidate.replacement.retained_values.clone(),
            expressions,
            source,
            replacement: WorkDepthExpr {
                work: unknown,
                depth: unknown,
            },
            improvement_proven: false,
        })
    }

    /// Hypothetical cost of additional calls of ONE source operation with
    /// existing inputs. This is not whole-subgraph cloning or legal duplication.
    pub fn single_operation_duplication(
        &self,
        value: ValueId,
        extra_calls: u64,
    ) -> Result<DuplicationModel, String> {
        if !self.applied_operations.contains(&value) {
            return Err("duplication model requires an applied source operation".into());
        }
        let source = self
            .nodes
            .get(value.0)
            .ok_or("invalid duplication source operation")?;
        let mut expressions = self.expressions.clone();
        let (work, depth) = if extra_calls == 0 {
            let zero = expressions.constant(0);
            (zero, zero)
        } else {
            let factor = expressions.constant(extra_calls);
            (
                expressions.product(source.work, factor),
                expressions.product(source.depth, factor),
            )
        };
        Ok(DuplicationModel {
            source: value,
            extra_calls,
            expressions,
            additional: WorkDepthExpr { work, depth },
            duplication_authorized: false,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicationModel {
    pub source: ValueId,
    pub extra_calls: u64,
    pub expressions: WorkDepthExprGraph,
    pub additional: WorkDepthExpr,
    pub duplication_authorized: bool,
}

impl DuplicationModel {
    pub fn verify(&self, analysis: &WorkDepthAnalysis, plan: &Plan) -> Result<(), String> {
        analysis.verify(plan)?;
        if *self != analysis.single_operation_duplication(self.source, self.extra_calls)? {
            return Err("duplication hypothesis differs from source Work/Depth model".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FusionWorkDepthModel {
    pub candidate_index: usize,
    pub operations: Vec<ValueId>,
    pub retained_values: Vec<ValueId>,
    pub expressions: WorkDepthExprGraph,
    pub source: WorkDepthExpr,
    pub replacement: WorkDepthExpr,
    pub improvement_proven: bool,
}

impl FusionWorkDepthModel {
    pub fn verify(
        &self,
        analysis: &WorkDepthAnalysis,
        fusion: &crate::j_graph_fusion::FusionAnalysis,
        registry: &crate::j_graph_fusion::FusionRegistry,
        plan: &Plan,
    ) -> Result<(), String> {
        if *self != analysis.fusion_envelope_model(fusion, registry, self.candidate_index, plan)? {
            return Err("fusion Work/Depth comparison lacks matching source witnesses".into());
        }
        Ok(())
    }
}

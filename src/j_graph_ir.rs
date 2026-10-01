//! J-grammar-preserving applied computation graph.
//!
//! This is the primary JAXA-style optimization surface.  It is deliberately
//! distinct from the execution-oriented logical IR:
//!
//! - J combinator regions (@:, hook, fork) remain first-class provenance.
//! - Their internal stages/branches are also explicit ValueId-producing nodes.
//! - shape/rank facts and symbolic analysis contracts can therefore propagate
//!   before execution lowering.
//! - physical schedule, buffers and concrete target resource numbers do not
//!   belong here.

use crate::{
    Error, Result, Value,
    contracts::{self, OperationClass, Valence},
    facts::{SemanticFacts, TypeFact},
    semantic::{
        BoundProgram, Expr, ExprKind, FunctionEntity, FunctionHead, FunctionOperand, NameVersion,
    },
};
use std::{collections::HashMap, ops::Range, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphSchemaVersion {
    pub major: u16,
    pub minor: u16,
}

pub const J_GRAPH_SCHEMA_VERSION: GraphSchemaVersion =
    GraphSchemaVersion { major: 0, minor: 3 };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphIrHeader {
    pub schema: GraphSchemaVersion,
    pub primitive_registry_version: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphAnalyzability {
    /// Graph topology and the graph-level abstract facts required by current
    /// JAXA analyses are known. This does NOT mean error-free, reorderable,
    /// fusible, or physically schedulable.
    Static,
    /// Topology is static, but one or more shape/type/resource facts are unknown.
    StaticWithUnknownFacts,
    /// A name-bound function must be specialized/version-resolved before analysis.
    RequiresSpecialization,
    /// The graph remains executable semantically, but JAXA-style static analysis
    /// must fall back rather than assuming missing facts.
    DynamicSemanticFallback,
}

impl GraphAnalyzability {
    fn combine(self, other: Self) -> Self {
        use GraphAnalyzability::*;
        match (self, other) {
            (DynamicSemanticFallback, _) | (_, DynamicSemanticFallback) => DynamicSemanticFallback,
            (RequiresSpecialization, _) | (_, RequiresSpecialization) => RequiresSpecialization,
            (StaticWithUnknownFacts, _) | (_, StaticWithUnknownFacts) => StaticWithUnknownFacts,
            _ => Static,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IterationContract {
    Elementwise,
    Structural,
    Gather,
    Search,
    Reduction,
    /// J prefix/infix family: a sequence of prefixes or windows is presented
    /// to the derived operand. Exact scan/window lowering is a later decision.
    WindowFamily,
    CellMap,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessContract {
    CorrespondingElements,
    IndexTransform,
    Indirect,
    SearchDependent,
    ReductionAxis,
    /// Overlapping prefix/window access induced by J `\`.
    WindowRelative,
    CellRelative,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FusionStructure {
    ElementwiseChain,
    ReductionAware,
    /// Neighborhood/prefix reuse exists, but fusion requires boundary/order proof.
    WindowAware,
    AccessSensitive,
    RequiresSemanticProof,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolicResourceExpr {
    /// This logical operation has no requirement of this resource kind.
    /// Physical lowering may still introduce target-local temporaries.
    None,
    /// No symbolic model registered yet. Never interpret as zero.
    Unknown,
    /// The enclosing combinator composes child resource expressions.
    StructuralComposition,
    /// Reduction requires accumulator state; exact size depends on shape/schedule.
    ReductionAccumulator,
    /// Prefix/window reuse requires a logical working set. Exact residency and
    /// size are schedule/target decisions.
    WindowWorkingSet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphOperationContract {
    pub iteration: IterationContract,
    pub access: AccessContract,
    pub fusion_structure: FusionStructure,
    pub temporary: SymbolicResourceExpr,
    pub accumulator: SymbolicResourceExpr,
    pub working_state: SymbolicResourceExpr,
}

impl Default for GraphOperationContract {
    fn default() -> Self {
        Self {
            iteration: IterationContract::Unknown,
            access: AccessContract::Unknown,
            fusion_structure: FusionStructure::Unknown,
            temporary: SymbolicResourceExpr::Unknown,
            accumulator: SymbolicResourceExpr::Unknown,
            working_state: SymbolicResourceExpr::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceCompositionRule {
    Pipeline,
    BranchJoin,
    Reduction,
    Window,
    CellMap,
    Structural,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GraphBasisKind {
    /// Pointwise/cell-local map structure. The scalar/cell function identity
    /// remains on the J FunctionEntity rather than being erased into this class.
    Elementwise,
    /// J rank/cell application boundary. Nested rank boundaries remain distinct.
    CellApply,
    /// Reduction structure whose reducer identity remains on the FunctionEntity.
    Reduce,
    /// Prefix/window access family. The operand identity remains separate, so
    /// e.g. `(+/)\` becomes Window -> Reduce rather than a fused special case.
    Window,
    /// Compile-time-known index remapping such as transpose/reshape/reverse.
    StaticReindex,
    /// Data-dependent indexed access such as general gather.
    DynamicGather,
    /// Search/classification access pattern retained for graph-level reasoning.
    Search,
    /// Named high-level graph operation intentionally kept opaque at this layer
    /// (for example convolution). Execution lowering may decompose it later.
    Structured,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphBasis {
    /// Outer-to-inner graph-basis composition. For example a ranked reduction
    /// is represented as [CellApply, Reduce], not collapsed into one execution op.
    pub layers: Vec<GraphBasisKind>,
}

#[derive(Clone, Debug)]
pub enum NodeKind {
    Literal(Value),
    ReadNoun {
        name: String,
        version: NameVersion,
    },
    VerbValue {
        function: Arc<FunctionEntity>,
    },
    /// An applied operation in the J graph.
    ///
    /// Direct @:/hook/fork are expanded into explicit stage/branch Apply nodes;
    /// their original combinator is retained in Plan::regions.
    Apply {
        function: Arc<FunctionEntity>,
        form: GraphForm,
        basis: GraphBasis,
        hints: GraphHints,
        rules: GraphRuleRefs,
        contract: GraphOperationContract,
        resource_composition: ResourceCompositionRule,
        valence: Valence,
        left: Option<ValueId>,
        right: ValueId,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// Early abstract facts used by J-graph analysis.  This intentionally excludes
/// layout/representation, guards, effects, error ordering and resolved call
/// instantiation.  Those belong to Execution IR.
pub struct GraphFacts {
    pub dtype: TypeFact,
    pub shape: Option<Vec<usize>>,
    pub rank: Option<usize>,
}

impl GraphFacts {
    fn from_semantic_facts(facts: &SemanticFacts) -> Self {
        Self {
            dtype: facts.dtype,
            shape: facts.shape.clone(),
            rank: facts.rank,
        }
    }

    fn as_semantic_facts(&self) -> SemanticFacts {
        SemanticFacts {
            dtype: self.dtype,
            shape: self.shape.clone(),
            rank: self.rank,
        }
    }

    pub fn agrees_with(
        &self,
        dtype: TypeFact,
        shape: Option<&[usize]>,
        rank: Option<usize>,
    ) -> bool {
        let dtype_agrees = match self.dtype {
            TypeFact::Unknown => true,
            TypeFact::Exact(expected) => dtype == TypeFact::Exact(expected),
            TypeFact::IntOrFloat => matches!(
                dtype,
                TypeFact::IntOrFloat
                    | TypeFact::Exact(crate::types::DType::Int)
                    | TypeFact::Exact(crate::types::DType::Float)
            ),
        };
        dtype_agrees
            && self
                .shape
                .as_deref()
                .map_or(true, |expected| Some(expected) == shape)
            && self.rank.map_or(true, |expected| Some(expected) == rank)
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: NodeKind,
    pub span: Range<usize>,
    /// Call-instance facts available at J Graph time. Unknown is explicit.
    pub facts: GraphFacts,
    pub analyzability: GraphAnalyzability,
}

#[derive(Clone, Debug)]
pub struct Write {
    pub name: String,
    pub value: ValueId,
    pub previous: Option<NameVersion>,
    pub proposed: NameVersion,
    pub span: Range<usize>,
}

#[derive(Clone, Debug)]
pub enum RegionKind {
    Pipeline {
        /// Results after each stage, in execution order.
        stage_results: Vec<ValueId>,
    },
    Hook {
        /// Identity/retained branch first, g(y) second.
        branch_results: Vec<ValueId>,
        join_result: ValueId,
        live_across: Vec<ValueId>,
    },
    Fork {
        /// J observable execution order: h branch, f branch.
        branch_results: Vec<ValueId>,
        join_result: ValueId,
        live_across: Vec<ValueId>,
    },
}

#[derive(Clone, Debug)]
pub struct Region {
    pub function: Arc<FunctionEntity>,
    pub kind: RegionKind,
    pub inputs: Vec<ValueId>,
    pub result: ValueId,
    pub span: Range<usize>,
    pub hints: GraphHints,
    pub resource_composition: ResourceCompositionRule,
    pub analyzability: GraphAnalyzability,
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub header: GraphIrHeader,
    pub source: String,
    pub nodes: Vec<Node>,
    pub regions: Vec<Region>,
    pub result: Option<ValueId>,
    pub write: Option<Write>,
    /// Names used as functions remain dynamically resolved today; retain their
    /// semantic source references so later binding/specialization can version
    /// them without reparsing.
    pub verb_references: Vec<(String, Range<usize>)>,
}

#[derive(Clone, Debug)]
pub enum GraphForm {
    Atomic,
    /// Classification form used while constructing a Pipeline region.
    Pipeline {
        stages: Vec<Arc<FunctionEntity>>,
    },
    Hook {
        f: Arc<FunctionEntity>,
        g: Arc<FunctionEntity>,
    },
    Fork {
        f: Arc<FunctionEntity>,
        g: Arc<FunctionEntity>,
        h: Arc<FunctionEntity>,
    },
    Reduce {
        operand: Arc<FunctionEntity>,
    },
    /// J `u\` derived verb. Monadic application is prefix-family; dyadic
    /// application is infix/window-family. We preserve the source adverb here
    /// and defer scan-vs-window execution refinement.
    PrefixInfix {
        operand: Arc<FunctionEntity>,
    },
    Rank {
        operand: Arc<FunctionEntity>,
        rank_spec: Option<Value>,
    },
    Modifier {
        head: FunctionHead,
        operands: Vec<Arc<FunctionEntity>>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GraphHint {
    PipelineFusionCandidate,
    IntermediateMaterializationElision,
    BranchJoinFusionCandidate,
    RetainedValueCandidate,
    ParallelBranchCandidate,
    ReductionStructure,
    WindowStructure,
    CellParallelStructure,
    /// Logical index/view transform can potentially remain virtual.
    VirtualIndexingCandidate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphRuleRef {
    Primitive(crate::primitive::PrimitiveId),
    StructuralComposition,
    DynamicOrUnknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceRuleRef {
    Unknown,
    StructuralComposition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphRuleRefs {
    pub shape: GraphRuleRef,
    pub dtype: GraphRuleRef,
    pub rank_cell: GraphRuleRef,
    pub effect: GraphRuleRef,
    pub resource: ResourceRuleRef,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphHints {
    pub items: Vec<GraphHint>,
}

impl GraphHints {
    fn push(&mut self, hint: GraphHint) {
        if !self.items.contains(&hint) {
            self.items.push(hint);
        }
    }

    pub fn contains(&self, hint: GraphHint) -> bool {
        self.items.contains(&hint)
    }
}

fn function_operands(function: &FunctionEntity) -> Vec<Arc<FunctionEntity>> {
    function
        .operands
        .iter()
        .filter_map(|operand| match operand {
            FunctionOperand::Function(function) => Some(function.clone()),
            FunctionOperand::Noun { .. } => None,
        })
        .collect()
}

fn flatten_atop(function: &Arc<FunctionEntity>, out: &mut Vec<Arc<FunctionEntity>>) {
    if matches!(
        &function.head,
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Atop)
    ) {
        if let [
            FunctionOperand::Function(outer),
            FunctionOperand::Function(inner),
        ] = function.operands.as_slice()
        {
            flatten_atop(inner, out);
            flatten_atop(outer, out);
            return;
        }
    }
    out.push(function.clone());
}

fn noun_operand_value(function: &FunctionEntity) -> Option<Value> {
    function.operands.iter().find_map(|operand| match operand {
        FunctionOperand::Noun { value, .. } => Some(value.clone()),
        FunctionOperand::Function(_) => None,
    })
}

fn rule_refs(function: &FunctionEntity) -> GraphRuleRefs {
    match &function.head {
        FunctionHead::PrimitiveVerb(id) => GraphRuleRefs {
            shape: GraphRuleRef::Primitive(*id),
            dtype: GraphRuleRef::Primitive(*id),
            rank_cell: GraphRuleRef::Primitive(*id),
            effect: GraphRuleRef::Primitive(*id),
            resource: ResourceRuleRef::Unknown,
        },
        FunctionHead::PrimitiveAdverb(_)
        | FunctionHead::PrimitiveConjunction(_)
        | FunctionHead::Hook
        | FunctionHead::Fork => GraphRuleRefs {
            shape: GraphRuleRef::StructuralComposition,
            dtype: GraphRuleRef::StructuralComposition,
            rank_cell: GraphRuleRef::StructuralComposition,
            effect: GraphRuleRef::StructuralComposition,
            resource: ResourceRuleRef::StructuralComposition,
        },
        FunctionHead::NameRef(_) => GraphRuleRefs {
            shape: GraphRuleRef::DynamicOrUnknown,
            dtype: GraphRuleRef::DynamicOrUnknown,
            rank_cell: GraphRuleRef::DynamicOrUnknown,
            effect: GraphRuleRef::DynamicOrUnknown,
            resource: ResourceRuleRef::Unknown,
        },
    }
}

fn graph_basis(
    function: &Arc<FunctionEntity>,
    valence: Valence,
    form: &GraphForm,
) -> GraphBasis {
    use crate::primitive::PrimitiveId::*;
    use GraphBasisKind::*;

    let mut layers = Vec::new();
    match form {
        GraphForm::Rank { operand, .. } => {
            layers.push(CellApply);
            let (inner_form, _) = classify_function(operand);
            layers.extend(graph_basis(operand, valence, &inner_form).layers);
        }
        GraphForm::Reduce { operand } => {
            layers.push(Reduce);
            let (inner_form, _) = classify_function(operand);
            if matches!(
                inner_form,
                GraphForm::Rank { .. } | GraphForm::Reduce { .. } | GraphForm::PrefixInfix { .. }
            ) {
                layers.extend(graph_basis(operand, Valence::Dyad, &inner_form).layers);
            }
        }
        GraphForm::PrefixInfix { operand } => {
            layers.push(Window);
            let (inner_form, _) = classify_function(operand);
            // Prefix/infix presents each selected prefix/window to u monadically.
            // Keep any meaningful inner graph-basis structure visible.
            layers.extend(graph_basis(operand, Valence::Monad, &inner_form).layers);
        }
        GraphForm::Atomic => {
            let FunctionHead::PrimitiveVerb(id) = &function.head else {
                return GraphBasis { layers };
            };
            let classified = match (*id, valence) {
                (Shape, Valence::Dyad)
                | (Ravel, Valence::Monad)
                | (Reverse, _)
                | (Transpose, _)
                | (Take, _)
                | (Drop, _) => Some(StaticReindex),
                (From, Valence::Dyad) => Some(DynamicGather),
                _ => match contracts::for_primitive(*id, valence).class {
                    OperationClass::Map => Some(Elementwise),
                    OperationClass::Gather => Some(DynamicGather),
                    OperationClass::Search => Some(Search),
                    OperationClass::Structural | OperationClass::Unknown => None,
                },
            };
            if let Some(kind) = classified {
                layers.push(kind);
            }
        }
        GraphForm::Modifier { .. }
        | GraphForm::Pipeline { .. }
        | GraphForm::Hook { .. }
        | GraphForm::Fork { .. } => {}
    }
    GraphBasis { layers }
}

fn node_resource_composition(form: &GraphForm) -> ResourceCompositionRule {
    match form {
        GraphForm::Reduce { .. } => ResourceCompositionRule::Reduction,
        GraphForm::PrefixInfix { .. } => ResourceCompositionRule::Window,
        GraphForm::Rank { .. } => ResourceCompositionRule::CellMap,
        GraphForm::Modifier { .. } => ResourceCompositionRule::Structural,
        GraphForm::Atomic => ResourceCompositionRule::Unknown,
        GraphForm::Pipeline { .. } | GraphForm::Hook { .. } | GraphForm::Fork { .. } => {
            ResourceCompositionRule::Structural
        }
    }
}

fn base_operation_contract(
    function: &Arc<FunctionEntity>,
    valence: Valence,
    form: &GraphForm,
) -> GraphOperationContract {
    match form {
        GraphForm::Reduce { .. } => GraphOperationContract {
            iteration: IterationContract::Reduction,
            access: AccessContract::ReductionAxis,
            fusion_structure: FusionStructure::ReductionAware,
            temporary: SymbolicResourceExpr::None,
            accumulator: SymbolicResourceExpr::ReductionAccumulator,
            working_state: SymbolicResourceExpr::None,
        },
        GraphForm::PrefixInfix { operand } => {
            let (inner_form, _) = classify_function(operand);
            let inner = base_operation_contract(operand, Valence::Monad, &inner_form);
            GraphOperationContract {
                iteration: IterationContract::WindowFamily,
                access: AccessContract::WindowRelative,
                fusion_structure: FusionStructure::WindowAware,
                temporary: inner.temporary,
                accumulator: inner.accumulator,
                working_state: SymbolicResourceExpr::WindowWorkingSet,
            }
        },
        GraphForm::Rank { .. } => GraphOperationContract {
            iteration: IterationContract::CellMap,
            access: AccessContract::CellRelative,
            fusion_structure: FusionStructure::RequiresSemanticProof,
            temporary: SymbolicResourceExpr::StructuralComposition,
            working_state: SymbolicResourceExpr::StructuralComposition,
            ..GraphOperationContract::default()
        },
        GraphForm::Atomic => {
            let FunctionHead::PrimitiveVerb(id) = &function.head else {
                return GraphOperationContract::default();
            };
            match contracts::for_primitive(*id, valence).class {
                OperationClass::Map => GraphOperationContract {
                    iteration: IterationContract::Elementwise,
                    access: AccessContract::CorrespondingElements,
                    fusion_structure: FusionStructure::ElementwiseChain,
                    temporary: SymbolicResourceExpr::None,
                    accumulator: SymbolicResourceExpr::None,
                    working_state: SymbolicResourceExpr::None,
                },
                OperationClass::Structural => GraphOperationContract {
                    iteration: IterationContract::Structural,
                    access: AccessContract::IndexTransform,
                    fusion_structure: FusionStructure::AccessSensitive,
                    temporary: SymbolicResourceExpr::None,
                    accumulator: SymbolicResourceExpr::None,
                    working_state: SymbolicResourceExpr::None,
                },
                OperationClass::Gather => GraphOperationContract {
                    iteration: IterationContract::Gather,
                    access: AccessContract::Indirect,
                    fusion_structure: FusionStructure::AccessSensitive,
                    ..GraphOperationContract::default()
                },
                OperationClass::Search => GraphOperationContract {
                    iteration: IterationContract::Search,
                    access: AccessContract::SearchDependent,
                    fusion_structure: FusionStructure::RequiresSemanticProof,
                    ..GraphOperationContract::default()
                },
                OperationClass::Unknown => GraphOperationContract::default(),
            }
        }
        GraphForm::Modifier { .. } => GraphOperationContract {
            temporary: SymbolicResourceExpr::StructuralComposition,
            working_state: SymbolicResourceExpr::StructuralComposition,
            fusion_structure: FusionStructure::RequiresSemanticProof,
            ..GraphOperationContract::default()
        },
        // These are construction-only forms. Direct applications are expanded
        // into explicit nodes and Region records before a NodeKind::Apply exists.
        GraphForm::Pipeline { .. } | GraphForm::Hook { .. } | GraphForm::Fork { .. } => {
            GraphOperationContract {
                temporary: SymbolicResourceExpr::StructuralComposition,
                working_state: SymbolicResourceExpr::StructuralComposition,
                fusion_structure: FusionStructure::RequiresSemanticProof,
                ..GraphOperationContract::default()
            }
        }
    }
}

fn apply_hints(
    function: &Arc<FunctionEntity>,
    form: &GraphForm,
    mut hints: GraphHints,
) -> GraphHints {
    if let FunctionHead::PrimitiveVerb(
        crate::primitive::PrimitiveId::Ravel
        | crate::primitive::PrimitiveId::Reverse
        | crate::primitive::PrimitiveId::Transpose,
    ) = &function.head
    {
        hints.push(GraphHint::VirtualIndexingCandidate);
    }
    if matches!(form, GraphForm::Reduce { .. }) {
        hints.push(GraphHint::ReductionStructure);
    }
    if matches!(form, GraphForm::PrefixInfix { .. }) {
        hints.push(GraphHint::WindowStructure);
    }
    if matches!(form, GraphForm::Rank { .. }) {
        hints.push(GraphHint::CellParallelStructure);
    }
    hints
}

pub fn classify_function(function: &Arc<FunctionEntity>) -> (GraphForm, GraphHints) {
    let mut hints = GraphHints::default();
    let form = match &function.head {
        FunctionHead::Hook => {
            hints.push(GraphHint::BranchJoinFusionCandidate);
            hints.push(GraphHint::RetainedValueCandidate);
            let [
                FunctionOperand::Function(f),
                FunctionOperand::Function(g),
            ] = function.operands.as_slice()
            else {
                return (
                    GraphForm::Modifier {
                        head: function.head.clone(),
                        operands: function_operands(function),
                    },
                    hints,
                );
            };
            GraphForm::Hook {
                f: f.clone(),
                g: g.clone(),
            }
        }
        FunctionHead::Fork => {
            hints.push(GraphHint::BranchJoinFusionCandidate);
            hints.push(GraphHint::RetainedValueCandidate);
            hints.push(GraphHint::ParallelBranchCandidate);
            let [
                FunctionOperand::Function(f),
                FunctionOperand::Function(g),
                FunctionOperand::Function(h),
            ] = function.operands.as_slice()
            else {
                return (
                    GraphForm::Modifier {
                        head: function.head.clone(),
                        operands: function_operands(function),
                    },
                    hints,
                );
            };
            GraphForm::Fork {
                f: f.clone(),
                g: g.clone(),
                h: h.clone(),
            }
        }
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Atop) => {
            hints.push(GraphHint::PipelineFusionCandidate);
            hints.push(GraphHint::IntermediateMaterializationElision);
            let mut stages = Vec::new();
            flatten_atop(function, &mut stages);
            GraphForm::Pipeline { stages }
        }
        FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert) => {
            hints.push(GraphHint::ReductionStructure);
            let operand = function_operands(function)
                .into_iter()
                .next()
                .unwrap_or_else(|| function.clone());
            GraphForm::Reduce { operand }
        }
        FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::PrefixInfix) => {
            hints.push(GraphHint::WindowStructure);
            let operand = function_operands(function)
                .into_iter()
                .next()
                .unwrap_or_else(|| function.clone());
            GraphForm::PrefixInfix { operand }
        }
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank) => {
            hints.push(GraphHint::CellParallelStructure);
            let operand = function_operands(function)
                .into_iter()
                .next()
                .unwrap_or_else(|| function.clone());
            GraphForm::Rank {
                operand,
                rank_spec: noun_operand_value(function),
            }
        }
        FunctionHead::PrimitiveAdverb(_) | FunctionHead::PrimitiveConjunction(_) => {
            GraphForm::Modifier {
                head: function.head.clone(),
                operands: function_operands(function),
            }
        }
        FunctionHead::PrimitiveVerb(_) | FunctionHead::NameRef(_) => GraphForm::Atomic,
    };
    (form, hints)
}

fn analyzability_for(
    function: &Arc<FunctionEntity>,
    facts: &GraphFacts,
    contract: &GraphOperationContract,
) -> GraphAnalyzability {
    if matches!(&function.head, FunctionHead::NameRef(_)) {
        return GraphAnalyzability::RequiresSpecialization;
    }
    if facts.shape.is_none()
        || facts.rank.is_none()
        || matches!(contract.iteration, IterationContract::Unknown)
    {
        GraphAnalyzability::StaticWithUnknownFacts
    } else {
        GraphAnalyzability::Static
    }
}

impl Plan {
    pub fn from_bound(bound: BoundProgram) -> Result<Self> {
        Self::from_bound_with_graph_facts(bound, &|_| GraphFacts::default())
    }

    pub fn from_bound_with_graph_facts(
        bound: BoundProgram,
        noun_facts: &dyn Fn(&str) -> GraphFacts,
    ) -> Result<Self> {
        let reads = bound
            .reads
            .iter()
            .map(|read| {
                (
                    (read.name.clone(), read.span.start, read.span.end),
                    read.version,
                )
            })
            .collect::<HashMap<_, _>>();

        let mut builder = Builder {
            nodes: Vec::new(),
            regions: Vec::new(),
            reads,
            noun_facts,
        };
        let result = bound
            .program
            .expression
            .map(|expression| builder.expression(expression))
            .transpose()?;
        let write = bound
            .write
            .map(|write| {
                Ok(Write {
                    name: write.name,
                    value: result.ok_or_else(|| Error::Syntax("assignment without value".into()))?,
                    previous: write.previous,
                    proposed: write.proposed,
                    span: write.span,
                })
            })
            .transpose()?;

        let plan = Self {
            header: GraphIrHeader {
                schema: J_GRAPH_SCHEMA_VERSION,
                primitive_registry_version: crate::primitive::REGISTRY_VERSION,
            },
            source: bound.program.source,
            nodes: builder.nodes,
            regions: builder.regions,
            result,
            write,
            verb_references: bound.verb_references,
        };
        plan.verify().map_err(|message| {
            Error::Unsupported(format!("J graph IR verification failed: {message}"))
        })?;
        Ok(plan)
    }

    pub fn graph_form(&self, value: ValueId) -> Option<&GraphForm> {
        let NodeKind::Apply { form, .. } = &self.nodes.get(value.0)?.kind else {
            return None;
        };
        Some(form)
    }

    pub fn graph_hints(&self, value: ValueId) -> Option<&GraphHints> {
        let NodeKind::Apply { hints, .. } = &self.nodes.get(value.0)?.kind else {
            return None;
        };
        Some(hints)
    }

    /// Return the outermost/latest combinator region producing this value.
    /// Multiple nested regions may legitimately share a result ValueId.
    pub fn region_for_result(&self, value: ValueId) -> Option<(RegionId, &Region)> {
        self.regions
            .iter()
            .enumerate()
            .rev()
            .find(|(_, region)| region.result == value)
            .map(|(i, region)| (RegionId(i), region))
    }

    pub fn regions_for_result(
        &self,
        value: ValueId,
    ) -> impl DoubleEndedIterator<Item = (RegionId, &Region)> {
        self.regions
            .iter()
            .enumerate()
            .filter(move |(_, region)| region.result == value)
            .map(|(i, region)| (RegionId(i), region))
    }

    pub fn analyzability(&self) -> GraphAnalyzability {
        self.nodes
            .iter()
            .fold(GraphAnalyzability::Static, |state, node| {
                state.combine(node.analyzability)
            })
    }

    /// Graph-level use counts are available before execution lowering.  This is
    /// enough to identify common inputs and seed later liveness/materialization
    /// analysis without reconstructing J topology from the execution DAG.
    pub fn static_memory_analysis(&self) -> crate::j_graph_memory::StaticMemoryAnalysis {
        crate::j_graph_memory::analyze(self)
    }

    pub fn symbolic_resource_analysis(
        &self,
    ) -> crate::j_graph_resource::GraphResourceSummary {
        let memory = self.static_memory_analysis();
        crate::j_graph_resource::analyze(self, &memory)
    }

    pub fn use_counts(&self) -> Vec<usize> {
        let mut counts = vec![0usize; self.nodes.len()];
        for node in &self.nodes {
            if let NodeKind::Apply { left, right, .. } = &node.kind {
                counts[right.0] = counts[right.0].saturating_add(1);
                if let Some(left) = left {
                    counts[left.0] = counts[left.0].saturating_add(1);
                }
            }
        }
        if let Some(write) = &self.write {
            counts[write.value.0] = counts[write.value.0].saturating_add(1);
        }
        if let Some(result) = self.result {
            counts[result.0] = counts[result.0].saturating_add(1);
        }
        counts
    }

    pub fn verify(&self) -> std::result::Result<(), String> {
        if self.header.schema != J_GRAPH_SCHEMA_VERSION {
            return Err("unsupported J Graph IR schema version".into());
        }
        if self.header.primitive_registry_version != crate::primitive::REGISTRY_VERSION {
            return Err("J Graph IR primitive registry provenance mismatch".into());
        }

        let source_len = self.source.len();
        for (index, node) in self.nodes.iter().enumerate() {
            if node.span.start > node.span.end
                || node.span.end > source_len
                || !self.source.is_char_boundary(node.span.start)
                || !self.source.is_char_boundary(node.span.end)
            {
                return Err(format!("node {index} has invalid source span"));
            }
            let check = |value: ValueId, label: &str| {
                if value.0 >= index {
                    Err(format!("node {index} {label} must reference an earlier value"))
                } else {
                    Ok(())
                }
            };
            if let (Some(rank), Some(shape)) = (node.facts.rank, node.facts.shape.as_ref()) {
                if rank != shape.len() {
                    return Err(format!("node {index} GraphFacts rank/shape mismatch"));
                }
            }
            if let NodeKind::Apply {
                function,
                form,
                basis,
                hints,
                rules,
                contract,
                resource_composition,
                valence,
                left,
                right,
            } = &node.kind
            {
                if matches!(
                    form,
                    GraphForm::Pipeline { .. } | GraphForm::Hook { .. } | GraphForm::Fork { .. }
                ) {
                    return Err(format!(
                        "node {index} retains a composite form that should be represented as a region"
                    ));
                }
                let (expected_form, expected_hints) = classify_function(function);
                if matches!(
                    expected_form,
                    GraphForm::Pipeline { .. } | GraphForm::Hook { .. } | GraphForm::Fork { .. }
                ) {
                    return Err(format!(
                        "node {index} composite function bypassed J Graph region expansion"
                    ));
                }
                if *rules != rule_refs(function) {
                    return Err(format!(
                        "node {index} graph rule refs do not match J function structure"
                    ));
                }
                if std::mem::discriminant(form) != std::mem::discriminant(&expected_form) {
                    return Err(format!(
                        "node {index} graph form does not match J function structure"
                    ));
                }
                let expected_basis = graph_basis(function, *valence, &expected_form);
                if *basis != expected_basis {
                    return Err(format!(
                        "node {index} graph basis does not match J function structure"
                    ));
                }
                let expected_hints = apply_hints(function, &expected_form, expected_hints);
                if *hints != expected_hints {
                    return Err(format!(
                        "node {index} graph hints do not match J function structure"
                    ));
                }
                if *contract != base_operation_contract(function, *valence, form) {
                    return Err(format!(
                        "node {index} graph operation contract does not match J function structure"
                    ));
                }
                if *resource_composition != node_resource_composition(form) {
                    return Err(format!(
                        "node {index} resource composition does not match J graph form"
                    ));
                }
                if node.analyzability != analyzability_for(function, &node.facts, contract) {
                    return Err(format!("node {index} graph analyzability is stale"));
                }
                check(*right, "right input")?;
                match (valence, left) {
                    (Valence::Monad, None) => {}
                    (Valence::Dyad, Some(left)) => check(*left, "left input")?,
                    _ => return Err(format!("node {index} valence/operand mismatch")),
                }
            }
        }

        for (index, region) in self.regions.iter().enumerate() {
            if region.span.start > region.span.end
                || region.span.end > source_len
                || !self.source.is_char_boundary(region.span.start)
                || !self.source.is_char_boundary(region.span.end)
            {
                return Err(format!("region {index} has invalid source span"));
            }
            let check = |value: ValueId, label: &str| {
                if value.0 >= self.nodes.len() {
                    Err(format!("region {index} {label} is out of bounds"))
                } else {
                    Ok(())
                }
            };
            for input in &region.inputs {
                check(*input, "input")?;
            }
            check(region.result, "result")?;
            let (expected_form, expected_hints) = classify_function(&region.function);
            if region.hints != expected_hints {
                return Err(format!("region {index} hints are stale"));
            }

            match (&region.kind, expected_form) {
                (
                    RegionKind::Pipeline { stage_results },
                    GraphForm::Pipeline { .. },
                ) => {
                    if region.resource_composition != ResourceCompositionRule::Pipeline {
                        return Err(format!("region {index} pipeline composition mismatch"));
                    }
                    if stage_results.len() < 2 {
                        return Err(format!("region {index} pipeline has fewer than two stages"));
                    }
                    for result in stage_results {
                        check(*result, "stage result")?;
                    }
                    if stage_results.last().copied() != Some(region.result) {
                        return Err(format!("region {index} pipeline result mismatch"));
                    }
                    let expected_analyzability = stage_results.iter().fold(
                        GraphAnalyzability::Static,
                        |state, value| state.combine(self.nodes[value.0].analyzability),
                    );
                    if region.analyzability != expected_analyzability {
                        return Err(format!("region {index} pipeline analyzability is stale"));
                    }

                    let mut previous = *region
                        .inputs
                        .last()
                        .ok_or_else(|| format!("region {index} pipeline has no input"))?;
                    for (stage_index, stage_result) in stage_results.iter().copied().enumerate() {
                        let NodeKind::Apply {
                            left,
                            right,
                            valence,
                            ..
                        } = &self.nodes[stage_result.0].kind
                        else {
                            return Err(format!(
                                "region {index} pipeline stage {stage_index} is not an Apply node"
                            ));
                        };
                        if *right != previous {
                            return Err(format!(
                                "region {index} pipeline stage {stage_index} is not chained"
                            ));
                        }
                        if stage_index == 0 && region.inputs.len() == 2 {
                            if *valence != Valence::Dyad || *left != Some(region.inputs[0]) {
                                return Err(format!(
                                    "region {index} dyadic pipeline first stage mismatch"
                                ));
                            }
                        } else if *left != None {
                            return Err(format!(
                                "region {index} pipeline stage {stage_index} must be monadic"
                            ));
                        }
                        previous = stage_result;
                    }
                }
                (
                    RegionKind::Hook {
                        branch_results,
                        join_result,
                        live_across,
                    },
                    GraphForm::Hook { .. },
                ) => {
                    if region.resource_composition != ResourceCompositionRule::BranchJoin {
                        return Err(format!("region {index} hook composition mismatch"));
                    }
                    if branch_results.len() != 2 || live_across.len() != 1 {
                        return Err(format!("region {index} malformed hook topology"));
                    }
                    let retained = branch_results[0];
                    if live_across[0] != retained || !region.inputs.contains(&retained) {
                        return Err(format!("region {index} hook retained-value mismatch"));
                    }
                    let NodeKind::Apply {
                        left,
                        right,
                        valence,
                        ..
                    } = &self.nodes[join_result.0].kind
                    else {
                        return Err(format!("region {index} hook join is not Apply"));
                    };
                    if *valence != Valence::Dyad
                        || *left != Some(retained)
                        || *right != branch_results[1]
                    {
                        return Err(format!("region {index} hook join wiring mismatch"));
                    }
                    if *join_result != region.result {
                        return Err(format!("region {index} join/result mismatch"));
                    }
                    let expected_analyzability = self.nodes[branch_results[1].0]
                        .analyzability
                        .combine(self.nodes[join_result.0].analyzability);
                    if region.analyzability != expected_analyzability {
                        return Err(format!("region {index} hook analyzability is stale"));
                    }
                }
                (
                    RegionKind::Fork {
                        branch_results,
                        join_result,
                        live_across,
                    },
                    GraphForm::Fork { .. },
                ) => {
                    if region.resource_composition != ResourceCompositionRule::BranchJoin {
                        return Err(format!("region {index} fork composition mismatch"));
                    }
                    if branch_results.len() != 2 {
                        return Err(format!("region {index} malformed fork topology"));
                    }
                    for input in &region.inputs {
                        if !live_across.contains(input) {
                            return Err(format!("region {index} fork live-across mismatch"));
                        }
                    }
                    let NodeKind::Apply {
                        left,
                        right,
                        valence,
                        ..
                    } = &self.nodes[join_result.0].kind
                    else {
                        return Err(format!("region {index} fork join is not Apply"));
                    };
                    // branch_results preserve J observable branch order: h, then f.
                    if *valence != Valence::Dyad
                        || *left != Some(branch_results[1])
                        || *right != branch_results[0]
                    {
                        return Err(format!("region {index} fork join wiring mismatch"));
                    }
                    if *join_result != region.result {
                        return Err(format!("region {index} join/result mismatch"));
                    }
                    let expected_analyzability = self.nodes[branch_results[0].0]
                        .analyzability
                        .combine(self.nodes[branch_results[1].0].analyzability)
                        .combine(self.nodes[join_result.0].analyzability);
                    if region.analyzability != expected_analyzability {
                        return Err(format!("region {index} fork analyzability is stale"));
                    }
                }
                _ => {
                    return Err(format!(
                        "region {index} kind does not match originating J combinator"
                    ));
                }
            }
        }

        if let Some(result) = self.result {
            if result.0 >= self.nodes.len() {
                return Err("result is out of bounds".into());
            }
        }
        if let Some(write) = &self.write {
            if write.value.0 >= self.nodes.len() {
                return Err("write value is out of bounds".into());
            }
        }
        Ok(())
    }
}

struct Builder<'a> {
    nodes: Vec<Node>,
    regions: Vec<Region>,
    reads: HashMap<(String, usize, usize), NameVersion>,
    noun_facts: &'a dyn Fn(&str) -> GraphFacts,
}

impl Builder<'_> {
    fn push(
        &mut self,
        kind: NodeKind,
        span: Range<usize>,
        facts: GraphFacts,
        analyzability: GraphAnalyzability,
    ) -> ValueId {
        let id = ValueId(self.nodes.len());
        self.nodes.push(Node {
            kind,
            span,
            facts,
            analyzability,
        });
        id
    }

    fn value_facts(&self, id: ValueId) -> &GraphFacts {
        &self.nodes[id.0].facts
    }

    fn expression(&mut self, expression: Expr) -> Result<ValueId> {
        let span = expression.span;
        match expression.kind {
            ExprKind::Group(inner) => self.expression(*inner),
            ExprKind::Literal(value) => {
                let semantic_facts = SemanticFacts::of(&value);
                let facts = GraphFacts::from_semantic_facts(&semantic_facts);
                Ok(self.push(
                    NodeKind::Literal(value),
                    span,
                    facts,
                    GraphAnalyzability::Static,
                ))
            }
            ExprKind::VerbValue(verb) => Ok(self.push(
                NodeKind::VerbValue {
                    function: verb.entity,
                },
                span,
                GraphFacts::default(),
                GraphAnalyzability::StaticWithUnknownFacts,
            )),
            ExprKind::ReadName(name) => {
                let version = *self
                    .reads
                    .get(&(name.clone(), span.start, span.end))
                    .ok_or_else(|| Error::Value(name.clone()))?;
                let facts = (self.noun_facts)(&name);
                let analyzability = if facts.shape.is_some() && facts.rank.is_some() {
                    GraphAnalyzability::Static
                } else {
                    GraphAnalyzability::StaticWithUnknownFacts
                };
                Ok(self.push(
                    NodeKind::ReadNoun { name, version },
                    span,
                    facts,
                    analyzability,
                ))
            }
            ExprKind::Monad { verb, argument } => {
                let right = self.expression(*argument)?;
                self.apply_function(verb.entity, None, right, span)
            }
            ExprKind::Dyad { verb, left, right } => {
                // Preserve J's current analysis/evaluation ordering: right first.
                let right = self.expression(*right)?;
                let left = self.expression(*left)?;
                self.apply_function(verb.entity, Some(left), right, span)
            }
        }
    }

    fn apply_function(
        &mut self,
        function: Arc<FunctionEntity>,
        left: Option<ValueId>,
        right: ValueId,
        span: Range<usize>,
    ) -> Result<ValueId> {
        let (form, base_hints) = classify_function(&function);
        match form {
            GraphForm::Pipeline { stages } => {
                if stages.len() < 2 {
                    return Err(Error::Unsupported("malformed atop pipeline".into()));
                }
                let mut inputs = Vec::with_capacity(2);
                if let Some(left) = left {
                    inputs.push(left);
                }
                inputs.push(right);

                let mut current = right;
                let mut stage_results = Vec::with_capacity(stages.len());
                let mut analyzability = GraphAnalyzability::Static;
                for (index, stage) in stages.into_iter().enumerate() {
                    current = self.apply_function(
                        stage.clone(),
                        if index == 0 { left } else { None },
                        current,
                        stage.span.clone(),
                    )?;
                    analyzability =
                        analyzability.combine(self.nodes[current.0].analyzability);
                    stage_results.push(current);
                }
                let result = current;
                self.regions.push(Region {
                    function,
                    kind: RegionKind::Pipeline { stage_results },
                    inputs,
                    result,
                    span,
                    hints: base_hints,
                    resource_composition: ResourceCompositionRule::Pipeline,
                    analyzability,
                });
                Ok(result)
            }
            GraphForm::Hook { f, g } => {
                let g_result = self.apply_function(g.clone(), None, right, g.span.clone())?;
                let f_left = left.unwrap_or(right);
                let f_span = f.span.clone();
                let join_result =
                    self.apply_function(f, Some(f_left), g_result, f_span)?;
                let inputs = match left {
                    Some(left) => vec![left, right],
                    None => vec![right],
                };
                let live_across = vec![f_left];
                let analyzability = self.nodes[g_result.0]
                    .analyzability
                    .combine(self.nodes[join_result.0].analyzability);
                self.regions.push(Region {
                    function,
                    kind: RegionKind::Hook {
                        branch_results: vec![f_left, g_result],
                        join_result,
                        live_across,
                    },
                    inputs,
                    result: join_result,
                    span,
                    hints: base_hints,
                    resource_composition: ResourceCompositionRule::BranchJoin,
                    analyzability,
                });
                Ok(join_result)
            }
            GraphForm::Fork { f, g, h } => {
                // J observable order for a general fork: h, f, then g.
                let h_result =
                    self.apply_function(h.clone(), left, right, h.span.clone())?;
                let f_result =
                    self.apply_function(f.clone(), left, right, f.span.clone())?;
                let g_span = g.span.clone();
                let join_result =
                    self.apply_function(g, Some(f_result), h_result, g_span)?;
                let mut inputs = Vec::with_capacity(2);
                if let Some(left) = left {
                    inputs.push(left);
                }
                inputs.push(right);
                let analyzability = self.nodes[h_result.0]
                    .analyzability
                    .combine(self.nodes[f_result.0].analyzability)
                    .combine(self.nodes[join_result.0].analyzability);
                self.regions.push(Region {
                    function,
                    kind: RegionKind::Fork {
                        branch_results: vec![h_result, f_result],
                        join_result,
                        live_across: inputs.clone(),
                    },
                    inputs,
                    result: join_result,
                    span,
                    hints: base_hints,
                    resource_composition: ResourceCompositionRule::BranchJoin,
                    analyzability,
                });
                Ok(join_result)
            }
            form => {
                let valence = if left.is_some() {
                    Valence::Dyad
                } else {
                    Valence::Monad
                };
                let basis = graph_basis(&function, valence, &form);
                let hints = apply_hints(&function, &form, base_hints);
                let rules = rule_refs(&function);
                let contract = base_operation_contract(&function, valence, &form);
                let resource_composition = node_resource_composition(&form);

                let right_facts = self.value_facts(right).as_semantic_facts();
                let left_facts =
                    left.map(|id| self.value_facts(id).as_semantic_facts());
                let inferred = crate::facts::infer_semantic_projection(
                    &function,
                    left_facts.as_ref(),
                    &right_facts,
                );
                let facts = GraphFacts::from_semantic_facts(&inferred);

                let analyzability = analyzability_for(&function, &facts, &contract);
                Ok(self.push(
                    NodeKind::Apply {
                        function,
                        form,
                        basis,
                        hints,
                        rules,
                        contract,
                        resource_composition,
                        valence,
                        left,
                        right,
                    },
                    span,
                    facts,
                    analyzability,
                ))
            }
        }
    }
}

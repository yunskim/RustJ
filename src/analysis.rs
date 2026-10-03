//! Execution-oriented semantic lowering from J Graph IR.
//!
//! J grammar/topology discovery belongs to `j_graph_ir`. This module resolves
//! target-independent execution semantics and constructs canonical A3 Logical IR
//! directly. It does not own a second plan container.

use crate::{
    Error, Result,
    contracts::{self, Contract, Valence},
    facts::{Facts, ValueRole, ValueRoleFacts},
    j_graph_ir,
    logical_ir::{Plan, PlanBuilder, ValueId, Write},
    opportunity::{OpportunitySource, StructuralOpportunity, StructuralTopology},
    semantic::{FunctionEntity, FunctionHead, FunctionOperand, FunctionPartOfSpeech},
};
use std::{collections::HashMap, ops::Range, sync::Arc};

pub use crate::compilation::CompilationAnalysis;
pub use crate::execution_semantics::{
    AccessFact, AccessRelation, CallTarget, Callable, ExecutionBasis, ExecutionBasisKind,
    ResolvedInstantiation, Scope, Symbol, SymbolId,
};

fn primitive_execution_basis(
    id: crate::primitive::PrimitiveId,
    valence: Valence,
) -> Option<ExecutionBasisKind> {
    use crate::primitive::PrimitiveId::*;

    let dyad = valence == Valence::Dyad;
    match (id, dyad) {
        (IndexOf | Steps, false) => Some(ExecutionBasisKind::IndexSpace),
        (Equal, false) => Some(ExecutionBasisKind::LookupClassify),
        (Indices, false) => Some(ExecutionBasisKind::ReplicateCompactExpand),
        (Shape, true) | (Ravel, false) | (Reverse, _) | (Transpose, _) | (Take, _) | (Drop, _) => {
            Some(ExecutionBasisKind::StaticReindex)
        }
        (Ravel, true) => Some(ExecutionBasisKind::ConcatAssemble),
        (From, true) => Some(ExecutionBasisKind::Gather),
        (IndexOf | Steps | Indices | Member, true) => Some(ExecutionBasisKind::LookupClassify),
        (Magnitude, true) => Some(ExecutionBasisKind::Elementwise),
        _ if contracts::for_primitive(id, valence).class
            == crate::contracts::OperationClass::Map =>
        {
            Some(ExecutionBasisKind::Elementwise)
        }
        _ => None,
    }
}

fn append_execution_basis(
    function: &Arc<FunctionEntity>,
    valence: Valence,
    layers: &mut Vec<ExecutionBasisKind>,
) {
    match &function.head {
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank) => {
            layers.push(ExecutionBasisKind::CellApply);
            if let Some(FunctionOperand::Function(operand)) = function.operands.first() {
                append_execution_basis(operand, valence, layers);
            }
        }
        FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert) => {
            layers.push(ExecutionBasisKind::Reduce);
            if let Some(FunctionOperand::Function(operand)) = function.operands.first() {
                if matches!(
                    operand.head,
                    FunctionHead::PrimitiveAdverb(_) | FunctionHead::PrimitiveConjunction(_)
                ) {
                    append_execution_basis(operand, Valence::Dyad, layers);
                }
            }
        }
        FunctionHead::PrimitiveVerb(id) => {
            if let Some(kind) = primitive_execution_basis(*id, valence) {
                layers.push(kind);
            }
        }
        FunctionHead::NameRef(_) => {}
        FunctionHead::PrimitiveAdverb(_)
        | FunctionHead::PrimitiveConjunction(_)
        | FunctionHead::DefinitionConstructor(_)
        | FunctionHead::ExplicitDefinition(_)
        | FunctionHead::ModifierTrain
        | FunctionHead::Hook
        | FunctionHead::Fork => {}
    }
}

fn execution_basis(callable: &Callable, left: Option<ValueId>) -> ExecutionBasis {
    let valence = if left.is_some() {
        Valence::Dyad
    } else {
        Valence::Monad
    };
    let mut layers = Vec::new();
    append_execution_basis(&callable.semantic, valence, &mut layers);
    ExecutionBasis { layers }
}

fn outer_rank_boundary(function: &FunctionEntity) -> Option<[i64; 3]> {
    if !matches!(
        function.head,
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank)
    ) {
        return None;
    }
    let [
        FunctionOperand::Function(_),
        FunctionOperand::Noun { value, .. },
    ] = function.operands.as_slice()
    else {
        return None;
    };
    crate::semantic::rank_noun_contract(value).ok()
}

fn input_roles(
    target: CallTarget,
    left: Option<ValueId>,
    right: ValueId,
) -> Vec<(ValueId, ValueRole)> {
    use crate::primitive::PrimitiveId::*;

    let CallTarget::Primitive(id) = target else {
        return Vec::new();
    };
    match (id, left) {
        (IndexOf, None) => vec![(right, ValueRole::ShapeVector)],
        (Shape, Some(left)) => vec![(left, ValueRole::ShapeVector)],
        (From, Some(left)) => vec![(left, ValueRole::IndexVector)],
        (Take | Drop, Some(left)) => vec![(left, ValueRole::CountVector)],
        (Transpose, Some(left)) => vec![(left, ValueRole::AxisPermutation)],
        _ => Vec::new(),
    }
}

fn result_roles(target: CallTarget, dyad: bool) -> ValueRoleFacts {
    use crate::primitive::PrimitiveId::*;

    let mut roles = ValueRoleFacts::default();
    let CallTarget::Primitive(id) = target else {
        return roles;
    };
    match (id, dyad) {
        (Shape, false) => roles.insert(ValueRole::ShapeVector),
        (Indices, false) | (IndexOf | Steps, true) => roles.insert(ValueRole::IndexVector),
        _ => {}
    }
    roles
}

fn access_fact(callable: &Callable, left: Option<ValueId>, contract: Contract) -> AccessFact {
    if left.is_none()
        && matches!(
            &callable.semantic.head,
            FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert)
        )
    {
        return AccessFact::Known(AccessRelation::ReduceLeadingAxis);
    }
    if matches!(&callable.semantic.head, FunctionHead::PrimitiveVerb(_))
        && contract.class == crate::contracts::OperationClass::Map
    {
        return AccessFact::Known(AccessRelation::ElementwiseMap);
    }
    AccessFact::Opaque
}

pub(crate) fn lower_graph(
    graph: crate::j_graph_ir::Plan,
    noun_facts: &dyn Fn(&str) -> Facts,
) -> Result<Plan> {
    let source = graph.source.clone();
    let graph_node_count = graph.nodes.len();
    let graph_result = graph.result;
    let graph_write = graph.write.clone();
    let graph_region_count = graph.regions.len();
    let graph_regions = graph.regions.clone();

    let mut builder = Builder {
        noun_facts,
        symbols: Vec::new(),
        names: HashMap::new(),
        current_j_origin: None,
        logical: PlanBuilder::new(source, graph_node_count, graph_region_count),
    };
    let mut value_map = Vec::with_capacity(graph_node_count);

    for (index, node) in graph.nodes.into_iter().enumerate() {
        let origin = crate::j_graph_ir::ValueId(index);
        builder.current_j_origin = Some(origin);
        let graph_facts = node.facts.clone();
        let span = node.span;
        let value = match node.kind {
            crate::j_graph_ir::NodeKind::Literal(value) => builder.push_literal(value, span),
            crate::j_graph_ir::NodeKind::ReadNoun { name, version } => {
                builder.push_read_noun(name, version, span)
            }
            crate::j_graph_ir::NodeKind::VerbValue { function } => {
                builder.push_verb_reference(function, span)?
            }
            crate::j_graph_ir::NodeKind::Apply {
                function,
                left,
                right,
                ..
            } => {
                let left = left.map(|value| value_map[value.0]);
                let right = value_map[right.0];
                builder.call_entity(function, left, right, span)?
            }
        };

        let execution_facts = builder.logical.facts(value);
        if !graph_facts.agrees_with(
            execution_facts.dtype,
            execution_facts.shape.as_deref(),
            execution_facts.rank,
        ) {
            return Err(Error::Unsupported(format!(
                "J Graph/Execution fact drift at graph value {index}"
            )));
        }
        value_map.push(value);
    }
    builder.current_j_origin = None;

    let mut opportunities = Vec::new();
    for (region_index, region) in graph_regions.into_iter().enumerate() {
        let region_origin = Some(crate::j_graph_ir::RegionId(region_index));
        let map = |value: crate::j_graph_ir::ValueId| value_map[value.0];
        match region.kind {
            crate::j_graph_ir::RegionKind::Pipeline { stage_results } => {
                opportunities.push(StructuralOpportunity {
                    source: OpportunitySource::Atop,
                    j_region_origin: region_origin,
                    span: region.span,
                    topology: StructuralTopology::Pipeline {
                        inputs: region.inputs.into_iter().map(map).collect(),
                        stage_results: stage_results.into_iter().map(map).collect(),
                    },
                });
            }
            crate::j_graph_ir::RegionKind::Hook {
                branch_results,
                join_result,
                live_across,
            } => {
                let shared_inputs = if region.inputs.len() == 1 {
                    region.inputs.into_iter().map(map).collect()
                } else {
                    Vec::new()
                };
                opportunities.push(StructuralOpportunity {
                    source: OpportunitySource::Hook,
                    j_region_origin: region_origin,
                    span: region.span,
                    topology: StructuralTopology::BranchJoin {
                        shared_inputs,
                        branch_results: branch_results.into_iter().map(map).collect(),
                        join_result: map(join_result),
                        live_across: live_across.into_iter().map(map).collect(),
                    },
                });
            }
            crate::j_graph_ir::RegionKind::Fork {
                branch_results,
                join_result,
                live_across,
            } => {
                opportunities.push(StructuralOpportunity {
                    source: OpportunitySource::Fork,
                    j_region_origin: region_origin,
                    span: region.span,
                    topology: StructuralTopology::BranchJoin {
                        shared_inputs: region.inputs.into_iter().map(map).collect(),
                        branch_results: branch_results.into_iter().map(map).collect(),
                        join_result: map(join_result),
                        live_across: live_across.into_iter().map(map).collect(),
                    },
                });
            }
        }
    }

    let result = graph_result.map(|value| value_map[value.0]);
    let write = if let Some(write) = graph_write {
        Some(Write {
            symbol: builder.symbol(&write.name),
            value: value_map[write.value.0],
            previous: write.previous,
            proposed: write.proposed,
            span: write.span,
            after: builder.logical.last_ordered(),
        })
    } else {
        None
    };

    let Builder {
        symbols, logical, ..
    } = builder;
    let plan = logical.finish(symbols, opportunities, result, write);
    plan.verify()
        .map_err(|error| Error::Unsupported(error.to_string()))?;
    Ok(plan)
}

struct Builder<'a> {
    noun_facts: &'a dyn Fn(&str) -> Facts,
    symbols: Vec<Symbol>,
    names: HashMap<String, SymbolId>,
    current_j_origin: Option<j_graph_ir::ValueId>,
    logical: PlanBuilder,
}

impl Builder<'_> {
    fn symbol(&mut self, name: &str) -> SymbolId {
        if let Some(id) = self.names.get(name) {
            return *id;
        }
        let id = SymbolId(self.symbols.len());
        self.symbols.push(Symbol {
            name: name.into(),
            scope: Scope::CurrentGlobal,
        });
        self.names.insert(name.into(), id);
        id
    }

    fn callable_entity(&mut self, semantic: Arc<FunctionEntity>) -> Result<Callable> {
        let mut current = semantic.clone();
        loop {
            match &current.head {
                FunctionHead::PrimitiveVerb(id) => {
                    return Ok(Callable {
                        target: CallTarget::Primitive(*id),
                        semantic,
                    });
                }
                FunctionHead::NameRef(name) => {
                    return Ok(Callable {
                        target: CallTarget::Dynamic(self.symbol(name)),
                        semantic,
                    });
                }
                FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert)
                    if current.result_pos == FunctionPartOfSpeech::Verb =>
                {
                    let [FunctionOperand::Function(base)] = current.operands.as_slice() else {
                        return Err(Error::Unsupported(
                            "malformed insert semantic entity".into(),
                        ));
                    };
                    current = base.clone();
                }
                FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank)
                    if current.result_pos == FunctionPartOfSpeech::Verb =>
                {
                    let [
                        FunctionOperand::Function(base),
                        FunctionOperand::Noun { value, .. },
                    ] = current.operands.as_slice()
                    else {
                        return Err(Error::Unsupported("malformed rank semantic entity".into()));
                    };
                    crate::semantic::rank_noun_contract(value)?;
                    current = base.clone();
                }
                FunctionHead::PrimitiveAdverb(_)
                | FunctionHead::PrimitiveConjunction(_)
                | FunctionHead::DefinitionConstructor(_)
                | FunctionHead::ExplicitDefinition(_)
                | FunctionHead::ModifierTrain
                | FunctionHead::Hook
                | FunctionHead::Fork => {
                    return Err(Error::Unsupported(
                        "semantic function requires structural lowering".into(),
                    ));
                }
            }
        }
    }

    fn push_literal(&mut self, value: crate::Value, span: Range<usize>) -> ValueId {
        let facts = Facts::of(&value);
        self.logical
            .push_literal(value, facts, self.current_j_origin, span)
    }

    fn push_read_noun(
        &mut self,
        name: String,
        version: crate::semantic::NameVersion,
        span: Range<usize>,
    ) -> ValueId {
        let facts = (self.noun_facts)(&name);
        let symbol = self.symbol(&name);
        self.logical
            .push_read_noun(symbol, version, facts, self.current_j_origin, span)
    }

    fn push_verb_reference(
        &mut self,
        function: Arc<FunctionEntity>,
        span: Range<usize>,
    ) -> Result<ValueId> {
        let callable = self.callable_entity(function)?;
        Ok(self
            .logical
            .push_verb_reference(callable, self.current_j_origin, span))
    }

    fn resolved_instantiation(
        &self,
        callable: &Callable,
        left: Option<ValueId>,
        right: ValueId,
        result: &Facts,
    ) -> ResolvedInstantiation {
        let left_facts = left.map(|value| self.logical.facts(value));
        let right_facts = self.logical.facts(right);
        ResolvedInstantiation {
            target: callable.target,
            valence: if left.is_some() {
                Valence::Dyad
            } else {
                Valence::Monad
            },
            left_dtype: left_facts.map(|facts| facts.dtype),
            left_rank: left_facts.and_then(|facts| facts.rank),
            right_dtype: right_facts.dtype,
            right_rank: right_facts.rank,
            result_dtype: result.dtype,
            result_rank: result.rank,
            rank_boundary: outer_rank_boundary(&callable.semantic),
        }
    }

    fn call_entity(
        &mut self,
        semantic: Arc<FunctionEntity>,
        left: Option<ValueId>,
        right: ValueId,
        span: Range<usize>,
    ) -> Result<ValueId> {
        match &semantic.head {
            FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Atop)
            | FunctionHead::DefinitionConstructor(_)
            | FunctionHead::ExplicitDefinition(_)
            | FunctionHead::ModifierTrain
            | FunctionHead::Hook
            | FunctionHead::Fork => {
                return Err(Error::Unsupported(
                    "composite J function reached execution lowering without J Graph expansion"
                        .into(),
                ));
            }
            _ => {}
        }

        let callable = self.callable_entity(semantic)?;
        let valence = if left.is_some() {
            Valence::Dyad
        } else {
            Valence::Monad
        };
        let contract = match &callable.semantic.head {
            FunctionHead::PrimitiveVerb(id) => contracts::for_primitive(*id, valence),
            _ => contracts::lookup("", valence),
        };

        let left_facts = left.map(|value| self.logical.facts(value).clone());
        let right_facts = self.logical.facts(right).clone();
        let (facts, rank_plan) = crate::facts::infer_semantic_call(
            &callable.semantic,
            left_facts.as_ref(),
            &right_facts,
        );

        let basis = execution_basis(&callable, left);
        let instantiation = self.resolved_instantiation(&callable, left, right, &facts);
        let access = access_fact(&callable, left, contract);

        for (value, role) in input_roles(callable.target, left, right) {
            self.logical.insert_role(value, role);
        }
        let roles = result_roles(callable.target, left.is_some());

        Ok(self.logical.push_call(
            callable,
            basis,
            left,
            right,
            contract,
            facts,
            rank_plan,
            access,
            instantiation,
            roles,
            self.current_j_origin,
            span,
        ))
    }
}

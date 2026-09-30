//! J-grammar-preserving applied computation graph.
//!
//! This is distinct from the execution-oriented logical IR.  Nodes here keep
//! J function/combinator identity (primitive, modifier application, hook, fork,
//! @:, names) while making noun dependencies explicit with ValueIds.
//! Shape/basis/target/schedule decisions do not belong here.

use crate::{
    Error, Result, Value,
    contracts::Valence,
    semantic::{
        BoundProgram, Expr, ExprKind, FunctionEntity, FunctionHead, FunctionOperand, NameVersion,
    },
};
use std::{collections::HashMap, ops::Range, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueId(pub usize);

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
    Apply {
        function: Arc<FunctionEntity>,
        form: GraphForm,
        hints: GraphHints,
        valence: Valence,
        left: Option<ValueId>,
        right: ValueId,
    },
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: NodeKind,
    pub span: Range<usize>,
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
pub struct Plan {
    pub source: String,
    pub nodes: Vec<Node>,
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
    /// J @: chain, stored in execution order (inner-most first).
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
    /// Adjacent producer-consumer stages are syntactically exposed by @:.
    PipelineFusionCandidate,
    /// Intermediate array values may be avoidable if later legality allows.
    IntermediateMaterializationElision,
    /// Hook/fork expose a fan-out/fan-in topology.
    BranchJoinFusionCandidate,
    /// A shared/original input remains live across another branch.
    RetainedValueCandidate,
    /// Branches are dependency-independent candidates; J observable order still
    /// has to be proven relaxable before parallel execution.
    ParallelBranchCandidate,
    /// Insert/reduce syntax exposes a collective reduction.
    ReductionStructure,
    /// Rank syntax exposes repeated cell application / frame parallelism.
    CellParallelStructure,
}

#[derive(Clone, Debug, Default)]
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
        function.head,
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

fn classify(function: &Arc<FunctionEntity>) -> (GraphForm, GraphHints) {
    let mut hints = GraphHints::default();
    let form = match &function.head {
        FunctionHead::Hook => {
            hints.push(GraphHint::BranchJoinFusionCandidate);
            hints.push(GraphHint::RetainedValueCandidate);
            hints.push(GraphHint::ParallelBranchCandidate);
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

impl Plan {
    pub fn from_bound(bound: BoundProgram) -> Result<Self> {
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
            reads,
        };
        let result = bound
            .program
            .expression
            .map(|expression| builder.expression(expression))
            .transpose()?;
        let write = bound.write.map(|write| {
            Ok(Write {
                name: write.name,
                value: result.ok_or_else(|| Error::Syntax("assignment without value".into()))?,
                previous: write.previous,
                proposed: write.proposed,
                span: write.span,
            })
        }).transpose()?;

        let plan = Self {
            source: bound.program.source,
            nodes: builder.nodes,
            result,
            write,
            verb_references: bound.verb_references,
        };
        plan.verify()
            .map_err(|message| Error::Unsupported(format!("J graph IR verification failed: {message}")))?;
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

    pub fn verify(&self) -> std::result::Result<(), String> {
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
            if let NodeKind::Apply {
                function,
                form,
                hints,
                valence,
                left,
                right,
            } = &node.kind
            {
                let (expected_form, expected_hints) = classify(function);
                if std::mem::discriminant(form) != std::mem::discriminant(&expected_form) {
                    return Err(format!("node {index} graph form does not match J function structure"));
                }
                if hints.items != expected_hints.items {
                    return Err(format!("node {index} graph hints do not match J function structure"));
                }
                check(*right, "right input")?;
                match (valence, left) {
                    (Valence::Monad, None) => {}
                    (Valence::Dyad, Some(left)) => check(*left, "left input")?,
                    _ => return Err(format!("node {index} valence/operand mismatch")),
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

struct Builder {
    nodes: Vec<Node>,
    reads: HashMap<(String, usize, usize), NameVersion>,
}

impl Builder {
    fn push(&mut self, kind: NodeKind, span: Range<usize>) -> ValueId {
        let id = ValueId(self.nodes.len());
        self.nodes.push(Node { kind, span });
        id
    }

    fn expression(&mut self, expression: Expr) -> Result<ValueId> {
        let span = expression.span;
        match expression.kind {
            ExprKind::Group(inner) => self.expression(*inner),
            ExprKind::Literal(value) => Ok(self.push(NodeKind::Literal(value), span)),
            ExprKind::VerbValue(verb) => Ok(self.push(
                NodeKind::VerbValue {
                    function: verb.entity,
                },
                span,
            )),
            ExprKind::ReadName(name) => {
                let version = *self
                    .reads
                    .get(&(name.clone(), span.start, span.end))
                    .ok_or_else(|| Error::Value(name.clone()))?;
                Ok(self.push(NodeKind::ReadNoun { name, version }, span))
            }
            ExprKind::Monad { verb, argument } => {
                let right = self.expression(*argument)?;
                let (form, hints) = classify(&verb.entity);
                Ok(self.push(
                    NodeKind::Apply {
                        function: verb.entity,
                        form,
                        hints,
                        valence: Valence::Monad,
                        left: None,
                        right,
                    },
                    span,
                ))
            }
            ExprKind::Dyad { verb, left, right } => {
                // Preserve the existing J analysis order: right argument first.
                let right = self.expression(*right)?;
                let left = self.expression(*left)?;
                let (form, hints) = classify(&verb.entity);
                Ok(self.push(
                    NodeKind::Apply {
                        function: verb.entity,
                        form,
                        hints,
                        valence: Valence::Dyad,
                        left: Some(left),
                        right,
                    },
                    span,
                ))
            }
        }
    }
}

//! Checked ownership boundary around the existing Program + FrontendContext.
//! This adapter is not another semantic IR or an execution authorization.
use crate::{
    Error, Result,
    frontend_context::{FrontendContext, NamePolicy, NodeId, NodeKind, ParseRealization},
    semantic::{Expr, ExprKind, FunctionEntity, FunctionHead, Program},
    source::SourceOrigin,
};
use std::{collections::HashSet, sync::Arc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Requirements {
    pub name_environment: bool,
    pub ordered_name_effects: bool,
    pub late_function_lookup: bool,
    pub construction_observations: bool,
    pub deferred_definition_bodies: bool,
}

#[derive(Clone, Debug)]
pub struct VerifiedFrontend {
    program: Arc<Program>,
    requirements: Requirements,
}

fn defect(detail: &str) -> Error {
    Error::Verification(format!("frontend handoff: {detail}"))
}

fn value_node(context: &FrontendContext, mut id: NodeId) -> Result<NodeId> {
    for _ in 0..=context.nodes.len() {
        match context.nodes.get(id.0).map(|n| &n.kind) {
            Some(NodeKind::WriteName { value, .. }) => id = *value,
            Some(_) => return Ok(id),
            None => return Err(defect("semantic node is missing")),
        }
    }
    Err(defect("cyclic assignment value"))
}

fn origin(context: &FrontendContext, expr: &Expr) -> Result<NodeId> {
    value_node(
        context,
        expr.origin
            .ok_or_else(|| defect("expression has no origin"))?,
    )
}

fn same_function(context: &FrontendContext, id: NodeId, function: &Arc<FunctionEntity>) -> bool {
    matches!(&context.nodes[id.0].kind,
        NodeKind::Function(actual) | NodeKind::Construct { function: Some(actual), .. }
            if Arc::ptr_eq(actual, function))
}

impl VerifiedFrontend {
    pub fn from_program(mut program: Program) -> Result<Self> {
        let context = program
            .frontend
            .as_ref()
            .ok_or_else(|| Error::Unsupported("frontend handoff requires provenance".into()))?;
        context.verify().map_err(Error::Verification)?;
        if !context.complete || context.realization != ParseRealization::Deferred {
            return Err(Error::Unsupported(
                "frontend handoff requires completed deferred parsing; observations cannot replay"
                    .into(),
            ));
        }
        if program.source != context.source.as_ref() || context.source_origin.is_none() {
            return Err(defect("source identity/text mismatch"));
        }
        let root = context.root.and_then(|id| context.items[id.0].semantic);
        match (&program.expression, root) {
            (Some(expr), Some(root)) if origin(context, expr)? == value_node(context, root)? => {}
            (None, None) => {}
            _ => return Err(defect("Program root differs from parser root")),
        }
        let write = root.and_then(|id| match &context.nodes[id.0].kind {
            NodeKind::WriteName { target, copula, .. } => Some((*target, *copula)),
            _ => None,
        });
        if program.has_assignment() != write.is_some() {
            return Err(defect(
                "final assignment metadata disagrees with parser root",
            ));
        }
        if let Some((target, copula)) = write {
            let target_words = context.items[target.0].word_range.clone();
            let target_span = context.words[target_words.start].span.start
                ..context.words[target_words.end - 1].span.end;
            if program.assignment_span.as_ref() != Some(&target_span) {
                return Err(defect("final assignment target span"));
            }
            let source = program
                .assignment_source
                .as_ref()
                .ok_or_else(|| defect("assignment source metadata"))?;
            if source.target.word_range != target_words
                || source.copula.word_range != (copula.0..copula.0 + 1)
                || source.flags != context.words[copula.0].flags
                || source.noun_target != program.noun_assignment.is_some()
            {
                return Err(defect(
                    "assignment source metadata disagrees with parser occurrence",
                ));
            }
            if let Some(name) = &program.assignment
                && program.noun_assignment.is_none()
            {
                let item = &context.items[target.0];
                let crate::frontend_context::ItemProducer::Word(word) = item.producer else {
                    return Err(defect("simple assignment target is not a source word"));
                };
                if context.words[word.0].name.as_ref() != Some(name) {
                    return Err(defect("simple assignment spelling"));
                }
            }
            if let Some(noun) = &program.noun_assignment {
                let mut target_expr = &noun.target;
                while let ExprKind::Group(inner) = &target_expr.kind {
                    target_expr = inner;
                }
                let ExprKind::Literal(value) = &target_expr.kind else {
                    return Err(defect(
                        "noun assignment target is not a completed immutable noun",
                    ));
                };
                let names = crate::parser::literal_assignment_names(value)
                    .map_err(|e| defect(&e.to_string()))?;
                if names != noun.names
                    || program.assignment.as_ref() != ((names.len() == 1).then(|| &names[0]))
                {
                    return Err(defect("noun assignment resolved names"));
                }
                let node = context.items[target.0]
                    .semantic
                    .ok_or_else(|| defect("noun assignment target origin"))?;
                if origin(context, &noun.target)? != value_node(context, node)? {
                    return Err(defect("noun assignment target edges"));
                }
            }
        }
        let mut pending = program.expression.iter().collect::<Vec<_>>();
        if let Some(target) = &program.noun_assignment {
            pending.push(&target.target);
        }
        while let Some(expr) = pending.pop() {
            if context.source.get(expr.span.clone()).is_none() {
                return Err(defect("expression span"));
            }
            let id = origin(context, expr)?;
            let kind = &context.nodes[id.0].kind;
            let valid = match (&expr.kind, kind) {
                (
                    ExprKind::Literal(_),
                    NodeKind::Literal | NodeKind::Construct { function: None, .. },
                ) => true,
                (ExprKind::ReadName(name), NodeKind::ReadNoun(usage))
                | (ExprKind::TakeName { name, .. }, NodeKind::TakeName(usage)) => {
                    context.words[context.name_uses[usage.0].word.0]
                        .name
                        .as_ref()
                        == Some(name)
                }
                (ExprKind::Group(inner), _) => {
                    pending.push(inner);
                    origin(context, inner)? == id
                }
                (ExprKind::VerbValue(verb), _) => same_function(context, id, &verb.entity),
                (ExprKind::ModifierValue(function), _) => same_function(context, id, function),
                (
                    ExprKind::Monad { verb, argument },
                    NodeKind::Monad {
                        function,
                        argument: actual,
                    },
                ) => {
                    pending.push(argument);
                    same_function(context, *function, &verb.entity)
                        && origin(context, argument)? == value_node(context, *actual)?
                }
                (
                    ExprKind::Dyad { verb, left, right },
                    NodeKind::Dyad {
                        function,
                        left: a,
                        right: b,
                    },
                ) => {
                    pending.extend([left.as_ref(), right.as_ref()]);
                    same_function(context, *function, &verb.entity)
                        && origin(context, left)? == value_node(context, *a)?
                        && origin(context, right)? == value_node(context, *b)?
                }
                _ => false,
            };
            if !valid {
                return Err(defect("expression and occurrence edges disagree"));
            }
            if let ExprKind::TakeName { single_word, .. } = &expr.kind
                && *single_word != (context.words.len() == 1)
            {
                return Err(defect("abandon single-word rule"));
            }
        }
        let mut requirements = Requirements {
            name_environment: !context.name_uses.is_empty(),
            ordered_name_effects: context
                .nodes
                .iter()
                .any(|n| matches!(n.kind, NodeKind::WriteName { .. } | NodeKind::TakeName(_)))
                || context
                    .name_uses
                    .iter()
                    .any(|n| n.policy == NamePolicy::CaptureAndAbandon),
            late_function_lookup: context
                .name_uses
                .iter()
                .any(|n| n.policy == NamePolicy::LateAtCall),
            construction_observations: !program.modifier_snapshots.is_empty()
                || !program.fork_name_reads.is_empty()
                || !program.name_rank_snapshots.is_empty(),
            deferred_definition_bodies: false,
        };
        let mut functions = context
            .nodes
            .iter()
            .filter_map(|n| match &n.kind {
                NodeKind::Function(f)
                | NodeKind::Construct {
                    function: Some(f), ..
                } => Some(f.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut seen = HashSet::new();
        while let Some(function) = functions.pop() {
            if !seen.insert(std::ptr::from_ref(function)) {
                continue;
            }
            if let FunctionHead::ExplicitDefinition(code) = &function.head {
                code.verify().map_err(|e| defect(&e.to_string()))?;
                requirements.deferred_definition_bodies = true;
            }
            functions.extend(
                function
                    .operands
                    .iter()
                    .filter_map(|operand| match operand {
                        crate::semantic::FunctionOperand::Function(child) => Some(child.as_ref()),
                        _ => None,
                    }),
            );
        }
        // Shared literal ownership makes handing the same Program to analysis
        // adapters cheap; no array is evaluated or duplicated for provenance.
        let mut pending = program.expression.iter_mut().collect::<Vec<_>>();
        if let Some(target) = &mut program.noun_assignment {
            pending.push(&mut target.target);
        }
        while let Some(expr) = pending.pop() {
            match &mut expr.kind {
                ExprKind::Literal(value) => {
                    *value = std::mem::replace(value, crate::Value::scalar(0)).into_shared()
                }
                ExprKind::Group(inner) => pending.push(inner),
                ExprKind::Monad { argument, .. } => pending.push(argument),
                ExprKind::Dyad { left, right, .. } => {
                    pending.extend([left.as_mut(), right.as_mut()])
                }
                _ => {}
            }
        }
        Ok(Self {
            program: Arc::new(program),
            requirements,
        })
    }

    pub fn program(&self) -> &Arc<Program> {
        &self.program
    }
    pub fn context(&self) -> &Arc<FrontendContext> {
        self.program.frontend.as_ref().expect("verified context")
    }
    pub fn source_origin(&self) -> &SourceOrigin {
        self.context()
            .source_origin
            .as_ref()
            .expect("verified source")
    }
    pub fn requirements(&self) -> Requirements {
        self.requirements
    }

    /// A bounded downstream adapter; never reparses or executes the sentence.
    pub fn lower_name_effects(&self) -> Result<crate::name_effect_ir::Plan> {
        crate::name_effect_ir::Plan::from_program((*self.program).clone())
    }
}

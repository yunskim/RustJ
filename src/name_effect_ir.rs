//! Ordered semantic lowering for a bounded simple-NAME sentence.
//! This is not a captured execution, a physical schedule, or pure array IR.
use std::{collections::HashMap, ops::Range, sync::Arc};

use crate::{
    Error, Result, Value,
    frontend_context::{ItemId, NamePolicy, NodeId, NodeKind, ParseRealization, ParseStep, WordId},
    parser::{ParseClass, ParseRow},
    primitive::PrimitiveId,
    semantic::{ExprKind, FunctionEntity, FunctionHead, Program},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueId(pub usize);

/// Success continuation. Failure never issues the next token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectToken(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Literal(NodeId),
    Function(NodeId),
    Read {
        name: String,
        expected: ParseClass,
    },
    Take {
        name: String,
        expected: ParseClass,
        single_word: bool,
    },
    Apply {
        primitive: PrimitiveId,
        function: NodeId,
        left: Option<ValueId>,
        right: ValueId,
    },
    Commit {
        name: String,
        value: ValueId,
    },
}

impl Operation {
    pub(crate) fn inputs(&self) -> impl Iterator<Item = ValueId> {
        let pair = match self {
            Self::Apply { left, right, .. } => [*left, Some(*right)],
            Self::Commit { value, .. } => [Some(*value), None],
            _ => [None, None],
        };
        pair.into_iter().flatten()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub operation: Operation,
    pub output: Option<ValueId>,
    pub before: EffectToken,
    pub after: EffectToken,
    pub parser_step: usize,
    pub span: Range<usize>,
    pub blame: WordId,
}

#[derive(Clone, Debug)]
pub struct Plan {
    program: Program,
    constants: HashMap<NodeId, Value>,
    functions: HashMap<NodeId, Arc<FunctionEntity>>,
    steps: Vec<Step>,
    result: ValueId,
    value_count: usize,
}

fn unsupported() -> Error {
    Error::Unsupported("ordered NAME effect lowering boundary".into())
}

fn simple_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && !name.ends_with('_')
        && !name.contains("__")
}

/// Assignment returns its RHS; follow explicit semantic edges, never spans.
fn value_origin(mut node: NodeId, program: &Program) -> Result<NodeId> {
    let context = program.frontend.as_ref().ok_or_else(unsupported)?;
    for _ in 0..=context.nodes.len() {
        match context.nodes.get(node.0).map(|node| &node.kind) {
            Some(NodeKind::WriteName { value, .. }) => node = *value,
            Some(_) => return Ok(node),
            None => return Err(unsupported()),
        }
    }
    Err(unsupported())
}

impl Plan {
    pub(crate) fn from_program(mut program: Program) -> Result<Self> {
        let context = program.frontend.as_ref().ok_or_else(unsupported)?;
        context.verify().map_err(|_| unsupported())?;
        if !context.complete
            || context.realization != ParseRealization::Deferred
            || program.source != context.source.as_ref()
            || program.noun_assignment.is_some()
            || !program.modifier_snapshots.is_empty()
            || !program.fork_name_reads.is_empty()
            || !program.name_rank_snapshots.is_empty()
        {
            return Err(unsupported());
        }
        // Freeze literals once; Program and lowering share their payloads.
        let mut literals = Vec::new();
        let mut function_roots = Vec::new();
        let mut pending = program.expression.iter_mut().collect::<Vec<_>>();
        while let Some(expr) = pending.pop() {
            match &mut expr.kind {
                ExprKind::Literal(value) => {
                    *value = std::mem::replace(value, Value::scalar(0)).into_shared();
                    literals.push((expr.origin.ok_or_else(unsupported)?, value.clone()));
                }
                ExprKind::Group(inner) => pending.push(inner),
                ExprKind::Monad { verb, argument } => {
                    function_roots.push(verb.entity.clone());
                    pending.push(argument);
                }
                ExprKind::Dyad { verb, left, right } => {
                    function_roots.push(verb.entity.clone());
                    pending.push(left);
                    pending.push(right);
                }
                ExprKind::VerbValue(verb) => function_roots.push(verb.entity.clone()),
                ExprKind::ModifierValue(function) => function_roots.push(function.clone()),
                _ => {}
            }
        }
        let constants = literals
            .into_iter()
            .map(|(origin, value)| Ok((value_origin(origin, &program)?, value)))
            .collect::<Result<HashMap<_, _>>>()?;
        // Function payloads come from semantic Program roots. Context supplies
        // occurrence links only; it never becomes an executable authority.
        let functions = program
            .frontend
            .as_ref()
            .expect("verified frontend")
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let NodeKind::Function(function) = &node.kind else {
                    return None;
                };
                function_roots
                    .iter()
                    .find(|root| Arc::ptr_eq(root, function))
                    .map(|root| (NodeId(index), root.clone()))
            })
            .collect();
        let (steps, result, value_count) = lower(&program, &constants, &functions)?;
        let plan = Self {
            program,
            constants,
            functions,
            steps,
            result,
            value_count,
        };
        plan.verify()?;
        Ok(plan)
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }
    pub fn program(&self) -> &Program {
        &self.program
    }
    pub fn result(&self) -> ValueId {
        self.result
    }
    pub(crate) fn value_count(&self) -> usize {
        self.value_count
    }
    pub(crate) fn literal(&self, id: NodeId) -> &Value {
        &self.constants[&id]
    }
    pub(crate) fn function(&self, id: NodeId) -> &Arc<FunctionEntity> {
        &self.functions[&id]
    }

    /// Re-derive the ordering contract from the original parser identities.
    /// This detects omitted/reordered effects, forged tokens and value edges.
    pub fn verify(&self) -> Result<()> {
        self.program
            .frontend
            .as_ref()
            .ok_or_else(|| Error::Verification("missing ordered NAME frontend".into()))?
            .verify()
            .map_err(Error::Verification)?;
        let (steps, result, value_count) = lower(&self.program, &self.constants, &self.functions)
            .map_err(|error| {
            Error::Verification(format!("invalid ordered NAME provenance: {error}"))
        })?;
        if steps != self.steps || result != self.result || value_count != self.value_count {
            return Err(Error::Verification(
                "invalid ordered NAME effect plan".into(),
            ));
        }
        Ok(())
    }
}

fn lower(
    program: &Program,
    constants: &HashMap<NodeId, Value>,
    functions: &HashMap<NodeId, Arc<FunctionEntity>>,
) -> Result<(Vec<Step>, ValueId, usize)> {
    let context = program.frontend.as_ref().ok_or_else(unsupported)?;
    let mut steps = Vec::new();
    let mut values = HashMap::<ItemId, ValueId>::new();
    let mut classes = Vec::new();
    for (index, parser_step) in context.steps.iter().enumerate() {
        let (operation, item, class, blame) = match parser_step {
            ParseStep::FrontMark(_) => continue,
            ParseStep::Stack { resolved, .. } => {
                let record = &context.items[resolved.0];
                let Some(node) = record.semantic else {
                    continue;
                };
                let blame = record.blame_word.ok_or_else(unsupported)?;
                let operation = match &context.nodes[node.0].kind {
                    NodeKind::Literal if constants.contains_key(&node) => Operation::Literal(node),
                    NodeKind::ReadNoun(id) | NodeKind::TakeName(id) => {
                        let usage = &context.name_uses[id.0];
                        let name = context.words[usage.word.0]
                            .name
                            .clone()
                            .ok_or_else(unsupported)?;
                        if !simple_name(&name) {
                            return Err(unsupported());
                        }
                        if usage.policy == NamePolicy::CaptureAndAbandon {
                            Operation::Take {
                                name,
                                expected: record.class,
                                single_word: context.words.len() == 1,
                            }
                        } else {
                            Operation::Read {
                                name,
                                expected: record.class,
                            }
                        }
                    }
                    NodeKind::Function(function) => match &function.head {
                        FunctionHead::TakeName { name, single_word } if simple_name(name) => {
                            Operation::Take {
                                name: name.clone(),
                                expected: record.class,
                                single_word: *single_word,
                            }
                        }
                        FunctionHead::PrimitiveVerb(_)
                        | FunctionHead::PrimitiveAdverb(_)
                        | FunctionHead::PrimitiveConjunction(_)
                            if function.operands.is_empty() && functions.contains_key(&node) =>
                        {
                            Operation::Function(node)
                        }
                        _ => return Err(unsupported()),
                    },
                    _ => return Err(unsupported()),
                };
                (operation, *resolved, Some(record.class), blame)
            }
            ParseStep::Reduce(id) => {
                let reduction = &context.reductions[id.0];
                let item = reduction.produced;
                if reduction.row == ParseRow::Parenthesis {
                    let value = *values.get(&reduction.consumed[1]).ok_or_else(unsupported)?;
                    values.insert(item, value);
                    continue;
                }
                let record = &context.items[item.0];
                let node = record.semantic.ok_or_else(unsupported)?;
                match &context.nodes[node.0].kind {
                    NodeKind::WriteName { target, copula, .. } => {
                        if index + 1 != context.steps.len() || !program.has_assignment() {
                            return Err(unsupported());
                        }
                        let target = &context.items[target.0];
                        if target.class != ParseClass::Name {
                            return Err(unsupported());
                        }
                        let name = context.words[target.word_range.start]
                            .name
                            .clone()
                            .ok_or_else(unsupported)?;
                        if !simple_name(&name) {
                            return Err(unsupported());
                        }
                        let value = *values.get(&reduction.consumed[2]).ok_or_else(unsupported)?;
                        values.insert(item, value);
                        (Operation::Commit { name, value }, item, None, *copula)
                    }
                    NodeKind::Monad { .. } | NodeKind::Dyad { .. } => {
                        let dyad = reduction.row == ParseRow::DyadNVN;
                        let function_index = usize::from(dyad);
                        let function_item = &context.items[reduction.consumed[function_index].0];
                        let function_node = function_item.semantic.ok_or_else(unsupported)?;
                        let function = functions.get(&function_node).ok_or_else(unsupported)?;
                        let FunctionHead::PrimitiveVerb(primitive) = function.head else {
                            return Err(unsupported());
                        };
                        if matches!(
                            primitive,
                            PrimitiveId::OperandU | PrimitiveId::OperandV | PrimitiveId::Cap
                        ) || !function.operands.is_empty()
                        {
                            return Err(unsupported());
                        }
                        let operand = |position: usize| -> Result<ValueId> {
                            let value = *values
                                .get(&reduction.consumed[position])
                                .ok_or_else(unsupported)?;
                            if classes[value.0] != ParseClass::Noun {
                                return Err(unsupported());
                            }
                            Ok(value)
                        };
                        let left = if dyad { Some(operand(0)?) } else { None };
                        let right = operand(if dyad { 2 } else { 1 })?;
                        (
                            Operation::Apply {
                                primitive,
                                function: function_node,
                                left,
                                right,
                            },
                            item,
                            Some(ParseClass::Noun),
                            function_item.blame_word.ok_or_else(unsupported)?,
                        )
                    }
                    _ => return Err(unsupported()),
                }
            }
        };
        let output = class.map(|class| {
            let value = ValueId(classes.len());
            classes.push(class);
            values.insert(item, value);
            value
        });
        let token = steps.len();
        steps.push(Step {
            operation,
            output,
            before: EffectToken(token),
            after: EffectToken(token + 1),
            parser_step: index,
            span: context.words[blame.0].span.clone(),
            blame,
        });
    }
    let result = *values
        .get(&context.root.ok_or_else(unsupported)?)
        .ok_or_else(unsupported)?;
    Ok((steps, result, classes.len()))
}

/// Observed binding transitions contain no retained array payloads and confer
/// no permission to reuse a generation/version as an executable guard.
#[derive(Clone, Debug)]
pub struct NameObservation {
    pub step: usize,
    pub before: crate::frontend_context::LookupObservation,
    pub after: crate::frontend_context::LookupObservation,
    pub deleted: bool,
}

#[derive(Debug)]
pub struct Execution {
    pub result: Result<Option<Value>>,
    pub completed: EffectToken,
    pub names: Vec<NameObservation>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verifier_rejects_reordered_omitted_or_forged_effects() {
        let mut engine = crate::Engine::new();
        engine.eval("a=:7").unwrap();
        let original = Plan::from_program(engine.parse_frontend("a_:+a").unwrap()).unwrap();
        let mut plan = original.clone();
        plan.steps.swap(0, 2);
        assert!(plan.verify().is_err());
        let mut plan = original.clone();
        plan.steps[0].after = EffectToken(99);
        assert!(plan.verify().is_err());
        let mut plan = original.clone();
        plan.steps.remove(2);
        assert!(plan.verify().is_err());
        let mut plan = original;
        let last = plan.steps.last_mut().unwrap();
        if let Operation::Apply { right, .. } = &mut last.operation {
            *right = ValueId(999);
        }
        assert!(plan.verify().is_err());
    }
}

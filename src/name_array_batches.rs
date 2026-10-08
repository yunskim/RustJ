//! Ordered multi-Apply regions. Immutable literal/function transport retains
//! zero-operation checkpoints; no NAME step or check changes semantic order.
use crate::{
    Error, Result,
    name_effect_ir::{EffectToken, Operation, Plan, ValueId},
};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub step: usize,
    pub operations: std::ops::Range<usize>,
    pub entry: EffectToken,
    pub success: EffectToken,
}

#[derive(Clone, Debug)]
pub struct ArrayBatch {
    start: usize,
    end: usize,
    inputs: Vec<(ValueId, usize)>,
    constants: Vec<(ValueId, crate::frontend_context::NodeId)>,
    outputs: Vec<(ValueId, crate::logical_ir::ValueId)>,
    checkpoints: Vec<Checkpoint>,
    graph: crate::j_graph_ir::Plan,
    logical: crate::logical_ir::Plan,
}

impl ArrayBatch {
    pub fn steps(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
    /// Each input is imported once; the count records parent uses replaced.
    pub fn inputs(&self) -> &[(ValueId, usize)] {
        &self.inputs
    }
    /// Immutable Program literal payloads supplied after ordinary imports.
    /// Their parser steps retain zero-operation checkpoints inside the batch.
    pub fn constants(&self) -> &[(ValueId, crate::frontend_context::NodeId)] {
        &self.constants
    }
    pub fn outputs(&self) -> &[(ValueId, crate::logical_ir::ValueId)] {
        &self.outputs
    }
    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }
    pub fn graph(&self) -> &crate::j_graph_ir::Plan {
        &self.graph
    }
    pub fn logical(&self) -> &crate::logical_ir::Plan {
        &self.logical
    }
}

fn invalid() -> Error {
    Error::Verification("invalid ordered array batch".into())
}

pub(crate) fn build(plan: &Plan) -> Result<Vec<ArrayBatch>> {
    let mut total_uses = vec![0; plan.value_count()];
    for step in plan.steps() {
        for input in step.operation.inputs() {
            total_uses[input.0] += 1;
        }
    }
    total_uses[plan.result().0] += 1;
    let mut batches = Vec::new();
    let mut start = 0;
    while start < plan.steps().len() {
        if !matches!(plan.steps()[start].operation, Operation::Apply { .. }) {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        let mut scan = end;
        while scan < plan.steps().len() {
            match &plan.steps()[scan].operation {
                Operation::Apply { .. } => end = scan + 1,
                Operation::Literal(_) => {}
                Operation::Function(_)
                    if plan.steps()[scan]
                        .output
                        .is_some_and(|value| total_uses[value.0] == 0) => {}
                _ => break,
            }
            scan += 1;
        }
        let mut inputs: Vec<(ValueId, usize)> = Vec::new();
        let mut constants = Vec::new();
        let mut produced = std::collections::HashSet::new();
        let mut internal_uses = vec![0; plan.value_count()];
        for step in &plan.steps()[start..end] {
            for value in step.operation.inputs() {
                internal_uses[value.0] += 1;
                if !produced.contains(&value) {
                    if let Some((_, uses)) = inputs.iter_mut().find(|(id, _)| *id == value) {
                        *uses += 1;
                    } else {
                        inputs.push((value, 1));
                    }
                }
            }
            produced.insert(step.output.ok_or_else(invalid)?);
            if let Operation::Literal(node) = step.operation {
                constants.push((step.output.ok_or_else(invalid)?, node));
            }
        }
        let mut mapping: HashMap<_, _> = inputs
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (*id, crate::j_graph_ir::ValueId(i)))
            .collect();
        for (offset, (id, _)) in constants.iter().enumerate() {
            mapping.insert(*id, crate::j_graph_ir::ValueId(inputs.len() + offset));
        }
        let input_count = inputs.len() + constants.len();
        let mut calls = Vec::new();
        for step in &plan.steps()[start..end] {
            let Operation::Apply {
                function,
                left,
                right,
                ..
            } = &step.operation
            else {
                continue;
            };
            calls.push(crate::j_graph_ir::ArrayCall {
                function: plan.function(*function).clone(),
                left: left.map(|value| mapping[&value]),
                right: mapping[right],
                span: step.span.clone(),
            });
            mapping.insert(
                step.output.ok_or_else(invalid)?,
                crate::j_graph_ir::ValueId(input_count + calls.len() - 1),
            );
        }
        let graph = crate::j_graph_ir::Plan::from_array_calls(
            plan.program().source.clone(),
            input_count,
            calls,
        )?;
        let logical =
            crate::analysis::lower_graph(graph.clone(), &|_| crate::facts::Facts::default())?;
        let mut checkpoints = Vec::new();
        let mut outputs = Vec::new();
        let mut cursor = input_count;
        for (offset, step) in plan.steps()[start..end].iter().enumerate() {
            let output = step.output.ok_or_else(invalid)?;
            let mut logical_output = None;
            let mut range = cursor..cursor;
            if matches!(step.operation, Operation::Apply { .. }) {
                let origin = mapping[&output];
                let indices: Vec<_> = logical
                    .operations
                    .iter()
                    .enumerate()
                    .filter(|(_, op)| op.j_origin == Some(origin))
                    .collect();
                let first = indices.first().ok_or_else(invalid)?.0;
                let (last, operation) = indices.last().ok_or_else(invalid)?;
                range = first..last + 1;
                cursor = range.end;
                logical_output = operation.results.first().copied();
            } else if matches!(step.operation, Operation::Literal(_)) {
                logical_output = Some(crate::logical_ir::ValueId(mapping[&output].0));
            }
            checkpoints.push(Checkpoint {
                step: start + offset,
                operations: range,
                entry: step.before,
                success: step.after,
            });
            if total_uses[output.0] > internal_uses[output.0] {
                outputs.push((output, logical_output.ok_or_else(invalid)?));
            }
        }
        batches.push(ArrayBatch {
            start,
            end,
            inputs,
            constants,
            outputs,
            checkpoints,
            graph,
            logical,
        });
        start = end;
    }
    Ok(batches)
}

fn same_call(a: &crate::logical_ir::CallOp, b: &crate::logical_ir::CallOp) -> bool {
    a.callable.target == b.callable.target
        && std::sync::Arc::ptr_eq(&a.callable.semantic, &b.callable.semantic)
        && a.left == b.left
        && a.right == b.right
        && a.execution_basis == b.execution_basis
        && a.contract == b.contract
        && a.iteration_domain == b.iteration_domain
        && a.effect == b.effect
        && a.speculation == b.speculation
        && a.possible_errors == b.possible_errors
        && a.destination == b.destination
        && a.instantiation == b.instantiation
        && a.rank_plan == b.rank_plan
        && a.access == b.access
        && a.constraints == b.constraints
}

/// Authenticate canonical lowering rather than accepting an arbitrary valid
/// SSA plan with missing checks or substituted primitives/facts.
pub(crate) fn verify(actual: &[ArrayBatch], plan: &Plan) -> Result<()> {
    use crate::{j_graph_ir::NodeKind, logical_ir::OpKind};
    let expected = build(plan)?;
    if actual.len() != expected.len() {
        return Err(invalid());
    }
    for (a, b) in actual.iter().zip(&expected) {
        a.graph.verify().map_err(|_| invalid())?;
        a.logical.verify().map_err(|_| invalid())?;
        if a.start != b.start
            || a.end != b.end
            || a.inputs != b.inputs
            || a.constants != b.constants
            || a.outputs != b.outputs
            || a.checkpoints != b.checkpoints
            || a.graph.nodes.len() != b.graph.nodes.len()
            || a.logical.operations.len() != b.logical.operations.len()
            || a.logical.values.len() != b.logical.values.len()
            || a.logical.result != b.logical.result
            || a.graph.result != b.graph.result
            || a.graph.source != b.graph.source
            || a.logical.source != b.logical.source
            || a.graph.write.is_some()
            || a.logical.write.is_some()
            || !a.logical.symbols.is_empty()
        {
            return Err(invalid());
        }
        for (a, b) in a.graph.nodes.iter().zip(&b.graph.nodes) {
            if a.span != b.span || a.facts != b.facts {
                return Err(invalid());
            }
            let valid = match (&a.kind, &b.kind) {
                (NodeKind::Input { index: a }, NodeKind::Input { index: b }) => a == b,
                (
                    NodeKind::Apply {
                        function: a,
                        left: ax,
                        right: ay,
                        ..
                    },
                    NodeKind::Apply {
                        function: b,
                        left: bx,
                        right: by,
                        ..
                    },
                ) => std::sync::Arc::ptr_eq(a, b) && ax == bx && ay == by,
                _ => false,
            };
            if !valid {
                return Err(invalid());
            }
        }
        for (a, b) in a.logical.values.iter().zip(&b.logical.values) {
            if a.producer != b.producer || a.facts != b.facts || a.roles != b.roles {
                return Err(invalid());
            }
        }
        for (a, b) in a.logical.operations.iter().zip(&b.logical.operations) {
            if a.results != b.results
                || a.j_origin != b.j_origin
                || a.span != b.span
                || a.order_after != b.order_after
            {
                return Err(invalid());
            }
            let valid = match (&a.kind, &b.kind) {
                (OpKind::Input { index: a }, OpKind::Input { index: b }) => a == b,
                (OpKind::SemanticCheck(a), OpKind::SemanticCheck(b)) => a == b,
                (
                    OpKind::Basis {
                        kind: a,
                        payload: ap,
                        call: ac,
                    },
                    OpKind::Basis {
                        kind: b,
                        payload: bp,
                        call: bc,
                    },
                ) => a == b && ap == bp && same_call(ac, bc),
                (OpKind::SemanticCall(a), OpKind::SemanticCall(b)) => same_call(a, b),
                _ => false,
            };
            if !valid {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_forged_exports_checkpoints_and_checks() {
        let engine = crate::Engine::new();
        let plan = engine.prepare_name_effects("1+2+3").unwrap();
        let batches = build(&plan).unwrap();
        assert!(batches.iter().any(|b| {
            b.checkpoints
                .iter()
                .filter(|point| !point.operations.is_empty())
                .count()
                == 2
        }));
        let mut bad = batches.clone();
        bad[0].checkpoints[0].success = EffectToken(0);
        assert!(verify(&bad, &plan).is_err());
        let mut bad = batches.clone();
        bad[0].outputs.clear();
        assert!(verify(&bad, &plan).is_err());
        let mut bad = batches.clone();
        bad[0].logical.values[0].facts.shape = Some(vec![1]);
        assert!(verify(&bad, &plan).is_err());
        let mut bad = batches;
        let check = bad[0]
            .logical
            .operations
            .iter_mut()
            .find_map(|operation| {
                if let crate::logical_ir::OpKind::SemanticCheck(check) = &mut operation.kind {
                    Some(check)
                } else {
                    None
                }
            })
            .unwrap();
        check.error = crate::logical_ir::SemanticErrorKind::Domain;
        assert!(bad[0].logical.verify().is_ok());
        assert!(verify(&bad, &plan).is_err());
    }
}

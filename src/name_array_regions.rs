//! Array lowering adapter around the ordered NAME plan. NAME effects remain
//! outside reusable J Graph/Logical regions and cannot be replayed by them.
use crate::{
    Error, Result,
    name_effect_ir::{EffectToken, Operation, Plan, ValueId},
};

#[derive(Clone, Debug)]
pub struct ArrayRegion {
    step: usize,
    entry: EffectToken,
    success: EffectToken,
    inputs: Vec<ValueId>,
    output: ValueId,
    graph: crate::j_graph_ir::Plan,
    logical: crate::logical_ir::Plan,
}

impl ArrayRegion {
    pub fn step(&self) -> usize {
        self.step
    }
    pub fn entry(&self) -> EffectToken {
        self.entry
    }
    pub fn success(&self) -> EffectToken {
        self.success
    }
    pub fn inputs(&self) -> &[ValueId] {
        &self.inputs
    }
    pub fn output(&self) -> ValueId {
        self.output
    }
    pub fn graph(&self) -> &crate::j_graph_ir::Plan {
        &self.graph
    }
    pub fn logical(&self) -> &crate::logical_ir::Plan {
        &self.logical
    }
}

#[derive(Clone, Debug)]
pub struct ArrayPlan {
    effects: Plan,
    regions: Vec<ArrayRegion>,
    batches: Vec<crate::name_array_batches::ArrayBatch>,
}

fn invalid() -> Error {
    Error::Verification("invalid NAME array region boundary".into())
}

impl ArrayPlan {
    pub fn from_effects(effects: Plan) -> Result<Self> {
        effects.verify()?;
        let regions = effects
            .steps()
            .iter()
            .enumerate()
            .filter_map(|(index, step)| {
                let Operation::Apply { function, left, .. } = &step.operation else {
                    return None;
                };
                Some((|| {
                    let graph = crate::j_graph_ir::Plan::from_array_call(
                        effects.program().source.clone(),
                        effects.function(*function).clone(),
                        left.is_some(),
                        step.span.clone(),
                    )?;
                    let logical = crate::analysis::lower_graph(graph.clone(), &|_| {
                        crate::facts::Facts::default()
                    })?;
                    Ok(ArrayRegion {
                        step: index,
                        entry: step.before,
                        success: step.after,
                        inputs: step.operation.inputs().collect(),
                        output: step.output.ok_or_else(invalid)?,
                        graph,
                        logical,
                    })
                })())
            })
            .collect::<Result<Vec<_>>>()?;
        let batches = crate::name_array_batches::build(&effects)?;
        let plan = Self {
            effects,
            regions,
            batches,
        };
        plan.verify()?;
        Ok(plan)
    }
    pub fn effects(&self) -> &Plan {
        &self.effects
    }
    pub fn regions(&self) -> &[ArrayRegion] {
        &self.regions
    }
    pub fn batches(&self) -> &[crate::name_array_batches::ArrayBatch] {
        &self.batches
    }
    pub(crate) fn batch_at_step(
        &self,
        step: usize,
    ) -> Option<&crate::name_array_batches::ArrayBatch> {
        self.batches
            .binary_search_by_key(&step, |batch| batch.steps().start)
            .ok()
            .map(|index| &self.batches[index])
    }
    pub fn verify(&self) -> Result<()> {
        use crate::{execution_semantics::CallTarget, j_graph_ir::NodeKind, logical_ir::OpKind};
        self.effects.verify()?;
        crate::name_array_batches::verify(&self.batches, &self.effects)?;
        let calls: Vec<_> = self
            .effects
            .steps()
            .iter()
            .enumerate()
            .filter(|(_, step)| matches!(step.operation, Operation::Apply { .. }))
            .collect();
        if calls.len() != self.regions.len() {
            return Err(invalid());
        }
        for ((index, step), region) in calls.into_iter().zip(&self.regions) {
            let Operation::Apply {
                primitive,
                function,
                left,
                ..
            } = &step.operation
            else {
                unreachable!()
            };
            if region.step != index
                || region.entry != step.before
                || region.success != step.after
                || Some(region.output) != step.output
                || region.inputs != step.operation.inputs().collect::<Vec<_>>()
                || region.graph.write.is_some()
                || region.logical.write.is_some()
                || !region.graph.regions.is_empty()
                || !region.logical.symbols.is_empty()
                || region.graph.source != self.effects.program().source
                || region.logical.source != region.graph.source
            {
                return Err(invalid());
            }
            region.graph.verify().map_err(|_| invalid())?;
            region.logical.verify().map_err(|_| invalid())?;
            // This boundary currently admits canonical, unspecialized lowering
            // only. Checks and their ordering cannot disappear even if a
            // modified Logical plan would pass its general SSA verifier.
            let expected = crate::analysis::lower_graph(region.graph.clone(), &|_| {
                crate::facts::Facts::default()
            })?;
            if region.logical.operations.len() != expected.operations.len() {
                return Err(invalid());
            }
            for (actual, expected) in region.logical.operations.iter().zip(&expected.operations) {
                if actual.results != expected.results
                    || actual.j_origin != expected.j_origin
                    || actual.span != expected.span
                    || actual.order_after != expected.order_after
                {
                    return Err(invalid());
                }
                let matches = match (&actual.kind, &expected.kind) {
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
                    ) => {
                        a == b
                            && ap == bp
                            && ac.constraints == bc.constraints
                            && ac.effect == bc.effect
                            && ac.possible_errors == bc.possible_errors
                            && ac.speculation == bc.speculation
                    }
                    (OpKind::SemanticCall(a), OpKind::SemanticCall(b)) => {
                        a.constraints == b.constraints
                            && a.effect == b.effect
                            && a.possible_errors == b.possible_errors
                            && a.speculation == b.speculation
                    }
                    _ => false,
                };
                if !matches {
                    return Err(invalid());
                }
            }
            let function = self.effects.function(*function);
            let arity = region.inputs.len();
            if region.graph.nodes.len() != arity + 1
                || region.graph.result != Some(crate::j_graph_ir::ValueId(arity))
            {
                return Err(invalid());
            }
            for (index, node) in region.graph.nodes[..arity].iter().enumerate() {
                if !matches!(node.kind, NodeKind::Input { index: actual } if actual == index) {
                    return Err(invalid());
                }
            }
            if !matches!(&region.graph.nodes[arity].kind,
                NodeKind::Apply { function: actual, left: x, right, .. }
                if std::sync::Arc::ptr_eq(actual, function)
                    && *x == left.map(|_| crate::j_graph_ir::ValueId(0))
                    && *right == crate::j_graph_ir::ValueId(arity - 1))
            {
                return Err(invalid());
            }
            let mut inputs = Vec::new();
            let mut seen_call = false;
            for operation in &region.logical.operations {
                match &operation.kind {
                    OpKind::Input { .. } if !seen_call => inputs.push(operation.results[0]),
                    OpKind::SemanticCheck(_) if !seen_call && inputs.len() == arity => {}
                    OpKind::Basis { call, .. } | OpKind::SemanticCall(call) if !seen_call => {
                        if inputs.len() != arity
                            || call.callable.target != CallTarget::Primitive(*primitive)
                            || !std::sync::Arc::ptr_eq(&call.callable.semantic, function)
                            || call.left != left.map(|_| inputs[0])
                            || call.right != inputs[arity - 1]
                            || operation.results.first().copied() != region.logical.result
                        {
                            return Err(invalid());
                        }
                        seen_call = true;
                    }
                    _ => return Err(invalid()),
                }
            }
            if !seen_call {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effect_boundaries_cannot_be_swapped_omitted_or_rebound() {
        let mut engine = crate::Engine::new();
        engine.eval("a=:7").unwrap();
        let plan =
            ArrayPlan::from_effects(engine.prepare_name_effects("a_:+1+2").unwrap()).unwrap();
        assert_eq!(plan.regions.len(), 2);
        let mut invalid_plan = plan.clone();
        invalid_plan.regions.swap(0, 1);
        assert!(invalid_plan.verify().is_err());
        let mut invalid_plan = plan.clone();
        invalid_plan.regions.pop();
        assert!(invalid_plan.verify().is_err());
        let mut invalid_plan = plan.clone();
        invalid_plan.regions[0].entry = EffectToken(0);
        assert!(invalid_plan.verify().is_err());
        let mut invalid_plan = plan.clone();
        invalid_plan.regions[0].inputs.reverse();
        assert!(invalid_plan.verify().is_err());
        let mut invalid_plan = plan;
        let check = invalid_plan.regions[0]
            .logical
            .operations
            .iter_mut()
            .find_map(|op| {
                if let crate::logical_ir::OpKind::SemanticCheck(check) = &mut op.kind {
                    Some(check)
                } else {
                    None
                }
            })
            .expect("unknown array shapes require a check");
        check.error = crate::logical_ir::SemanticErrorKind::Domain;
        assert!(
            invalid_plan.regions[0].logical.verify().is_ok(),
            "SSA alone does not authenticate a check"
        );
        assert!(invalid_plan.verify().is_err());
    }
}

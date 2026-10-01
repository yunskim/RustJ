//! Correctness-first executor for closed A3-v0 plans.
//!
//! This is not the native physical executor.  It deliberately reuses the
//! existing semantic kernels so A3 sequencing, checks and SSA wiring can be
//! tested before bufferization/scheduling are implemented.

use crate::{
    Error, Result, Value,
    analysis::CallTarget,
    logical_ir::{Constraint, OpKind, Plan, SemanticCheck, ValueId},
    semantic::{FunctionEntity, FunctionHead, FunctionOperand},
};

fn value_at(values: &[Option<Value>], id: ValueId) -> Result<&Value> {
    values
        .get(id.0)
        .and_then(Option::as_ref)
        .ok_or_else(|| Error::Unsupported("A3 reference value is unavailable".into()))
}

fn prefix_agrees(left: &[usize], right: &[usize]) -> bool {
    let (short, long) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    long.starts_with(short)
}

fn frame_rank(shape: &[usize], requested: i64) -> usize {
    let cell_rank = if requested < 0 {
        shape
            .len()
            .saturating_sub(usize::try_from(requested.unsigned_abs()).unwrap_or(usize::MAX))
    } else {
        shape
            .len()
            .min(usize::try_from(requested).unwrap_or(usize::MAX))
    };
    shape.len() - cell_rank
}

fn execute_check(check: &SemanticCheck, values: &[Option<Value>]) -> Result<()> {
    match check.constraint {
        Constraint::PrefixAgreement { left, right } => {
            let left = value_at(values, left)?;
            let right = value_at(values, right)?;
            if prefix_agrees(left.shape(), right.shape()) {
                Ok(())
            } else {
                Err(Error::Length)
            }
        }
        Constraint::CellFrameAgreement {
            left,
            right,
            left_rank,
            right_rank,
        } => {
            let left = value_at(values, left)?;
            let right = value_at(values, right)?;
            let left_frame = &left.shape()[..frame_rank(left.shape(), left_rank)];
            let right_frame = &right.shape()[..frame_rank(right.shape(), right_rank)];
            if prefix_agrees(left_frame, right_frame) {
                Ok(())
            } else {
                Err(Error::Length)
            }
        }
        Constraint::IndicesInBounds { indices, source } => {
            let indices = value_at(values, indices)?;
            let source = value_at(values, source)?;
            let items = source.shape().first().copied().unwrap_or(1);
            for position in 0..indices.len() {
                let index = indices.int_at(position)?;
                let index = if index < 0 {
                    (items as i64).checked_add(index).ok_or(Error::Index)?
                } else {
                    index
                };
                if index < 0 || index as usize >= items {
                    return Err(Error::Index);
                }
            }
            Ok(())
        }
    }
}

fn flattened_reference_semantics_supported(function: &FunctionEntity) -> bool {
    match &function.head {
        FunctionHead::PrimitiveVerb(_) => true,
        FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert) => matches!(
            function.operands.first(),
            Some(FunctionOperand::Function(operand))
                if matches!(operand.head, FunctionHead::PrimitiveVerb(_))
        ),
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank) => {
            match function.operands.first() {
                Some(FunctionOperand::Function(operand))
                    if matches!(operand.head, FunctionHead::PrimitiveVerb(_)) =>
                {
                    true
                }
                Some(FunctionOperand::Function(operand))
                    if matches!(
                        operand.head,
                        FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert)
                    ) =>
                {
                    matches!(
                        operand.operands.first(),
                        Some(FunctionOperand::Function(base))
                            if matches!(base.head, FunctionHead::PrimitiveVerb(_))
                    )
                }
                _ => false,
            }
        }
        FunctionHead::NameRef(_)
        | FunctionHead::PrimitiveAdverb(_)
        | FunctionHead::PrimitiveConjunction(_)
        | FunctionHead::Hook
        | FunctionHead::Fork => false,
    }
}

fn execute_call(
    call: &crate::logical_ir::CallOp,
    values: &[Option<Value>],
) -> Result<Value> {
    let CallTarget::Primitive(id) = call.callable.target else {
        return Err(Error::Unsupported(
            "A3 reference executor does not resolve dynamic calls".into(),
        ));
    };
    if !flattened_reference_semantics_supported(&call.callable.semantic) {
        return Err(Error::Unsupported(
            "A3 reference executor requires structural derived-modifier execution".into(),
        ));
    }

    let right = value_at(values, call.right)?.clone();
    if let Some(left) = call.left {
        let left = value_at(values, left)?.clone();
        if let Some(rank) = call.callable.rank {
            crate::kernels::ranked_dyad_ranks(
                id.spelling(),
                rank[1],
                rank[2],
                left,
                right,
            )
        } else {
            // This matches the current runtime's dyadic path.  A dyadic derived
            // form that needs structural semantics must stay outside this
            // reference subset until that semantic call is implemented.
            crate::kernels::dyad(id.spelling(), left, right)
        }
    } else if let Some(rank) = call.callable.rank {
        crate::kernels::ranked(
            id.spelling(),
            call.callable.reduce,
            rank[0],
            right,
        )
    } else if call.callable.reduce {
        crate::kernels::reduce(id.spelling(), right)
    } else {
        crate::kernels::monad(id.spelling(), right)
    }
}

fn store_single_result(
    operation: &crate::logical_ir::Operation,
    result: Value,
    values: &mut [Option<Value>],
) -> Result<()> {
    let [id] = operation.results.as_slice() else {
        return Err(Error::Unsupported(
            "A3 reference executor requires one result for value operations".into(),
        ));
    };
    values[id.0] = Some(result);
    Ok(())
}

/// Execute an A3 plan that is closed over noun values.
///
/// Name reads, verb-valued results and dynamic calls deliberately remain
/// unsupported.  Those require explicit runtime environment/state interfaces.
pub fn execute_closed(plan: &Plan) -> Result<Option<Value>> {
    plan.verify()
        .map_err(|error| Error::Unsupported(error.to_string()))?;

    let mut values = vec![None; plan.values.len()];

    for operation in &plan.operations {
        match &operation.kind {
            OpKind::Literal(value) => {
                store_single_result(operation, value.clone(), &mut values)?;
            }
            OpKind::ReadNoun { .. } => {
                return Err(Error::Unsupported(
                    "A3 closed reference executor cannot read names".into(),
                ));
            }
            OpKind::VerbReference(_) => {
                return Err(Error::Domain);
            }
            OpKind::SemanticCheck(check) => execute_check(check, &values)?,
            OpKind::Basis { call, .. } | OpKind::SemanticCall(call) => {
                let result = execute_call(call, &values)?;
                store_single_result(operation, result, &mut values)?;
            }
        }
    }

    plan.result
        .map(|result| {
            values[result.0]
                .clone()
                .ok_or_else(|| Error::Unsupported("A3 result value is unavailable".into()))
        })
        .transpose()
}

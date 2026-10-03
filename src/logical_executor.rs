//! Correctness-first executor for closed A3-v0 plans.
//!
//! This is not the native physical executor.  It deliberately reuses the
//! existing semantic kernels so A3 sequencing, checks and SSA wiring can be
//! tested before bufferization/scheduling are implemented.

use crate::{
    Error, Result, Value,
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

fn semantic_function_operand(function: &FunctionEntity) -> Option<&std::sync::Arc<FunctionEntity>> {
    function.operands.iter().find_map(|operand| match operand {
        FunctionOperand::Function(function) => Some(function),
        FunctionOperand::Noun { .. } => None,
    })
}

fn semantic_rank_triplet(function: &FunctionEntity) -> Option<[i64; 3]> {
    let value = function.operands.iter().find_map(|operand| match operand {
        FunctionOperand::Noun { value, .. } => Some(value),
        FunctionOperand::Function(_) => None,
    })?;
    crate::semantic::rank_noun_contract(value).ok()
}

fn cell_rank(array_rank: usize, requested: i64) -> usize {
    if requested < 0 {
        array_rank.saturating_sub(usize::try_from(requested.unsigned_abs()).unwrap_or(usize::MAX))
    } else {
        array_rank.min(usize::try_from(requested).unwrap_or(usize::MAX))
    }
}

fn assemble_uniform_cells(
    frame: Vec<usize>,
    cells: impl IntoIterator<Item = Result<Value>>,
) -> Result<Value> {
    let mut cells = cells.into_iter();
    let first = cells.next().ok_or_else(|| {
        Error::Unsupported("rank over empty frame (prototype inference)".into())
    })??;
    let result_shape = first.shape().to_vec();
    let mut shape = frame;
    shape.extend_from_slice(&result_shape);
    let mut builder = crate::assembly::CellBuilder::new(&first, crate::value::count(&shape)?)?;
    for cell in cells {
        let cell = cell?;
        if cell.shape() != result_shape.as_slice() {
            return Err(Error::Unsupported("rank result padding".into()));
        }
        builder.push(&cell)?;
    }
    Value::new(shape, builder.finish())
}

fn execute_ranked_semantic(
    function: &FunctionEntity,
    ranks: [i64; 3],
    left: Option<Value>,
    right: Value,
) -> Result<Value> {
    if let Some(left) = left {
        let ar = cell_rank(left.shape().len(), ranks[1]);
        let br = cell_rank(right.shape().len(), ranks[2]);
        let af = left.shape()[..left.shape().len() - ar].to_vec();
        let bf = right.shape()[..right.shape().len() - br].to_vec();

        if af.is_empty() && bf.is_empty() {
            return execute_semantic(function, Some(left), right);
        }
        let (short, frame) = if af.len() <= bf.len() {
            (af.as_slice(), bf.as_slice())
        } else {
            (bf.as_slice(), af.as_slice())
        };
        if !frame.starts_with(short) {
            return Err(Error::Length);
        }
        let frame = frame.to_vec();
        let frames = crate::value::count(&frame)?;
        if frames == 0 {
            return Err(Error::Unsupported(
                "dyadic rank over empty frame (prototype inference)".into(),
            ));
        }
        let ad = crate::value::count(&frame[af.len()..])?;
        let bd = crate::value::count(&frame[bf.len()..])?;
        let cells = (0..frames).map(|i| {
            let x = left.view().cell(ar, i / ad)?.to_owned()?;
            let y = right.view().cell(br, i / bd)?.to_owned()?;
            execute_semantic(function, Some(x), y)
        });
        assemble_uniform_cells(frame, cells)
    } else {
        let rank = cell_rank(right.shape().len(), ranks[0]);
        let frame_rank = right.shape().len() - rank;
        if frame_rank == 0 {
            return execute_semantic(function, None, right);
        }
        let frame = right.shape()[..frame_rank].to_vec();
        let frames = crate::value::count(&frame)?;
        if frames == 0 {
            return Err(Error::Unsupported(
                "rank over empty frame (prototype inference)".into(),
            ));
        }
        let cells = (0..frames).map(|i| {
            let cell = right.view().cell(rank, i)?.to_owned()?;
            execute_semantic(function, None, cell)
        });
        assemble_uniform_cells(frame, cells)
    }
}

fn execute_derived_reduction(function: &FunctionEntity, right: Value) -> Result<Value> {
    if let FunctionHead::PrimitiveVerb(id) = function.head {
        return crate::kernels::reduce(id.spelling(), right);
    }
    if right.shape().is_empty() {
        return Ok(right);
    }
    let items = right.shape()[0];
    if items == 0 {
        return Err(Error::Unsupported(
            "empty derived reduction identity is not implemented".into(),
        ));
    }
    let cell_rank = right.shape().len() - 1;
    let mut out = right.view().cell(cell_rank, items - 1)?.to_owned()?;
    for row in (0..items - 1).rev() {
        let left = right.view().cell(cell_rank, row)?.to_owned()?;
        out = execute_semantic(function, Some(left), out)?;
    }
    Ok(out)
}

fn execute_semantic(function: &FunctionEntity, left: Option<Value>, right: Value) -> Result<Value> {
    match &function.head {
        FunctionHead::PrimitiveVerb(id) => match left {
            Some(left) => crate::kernels::dyad(id.spelling(), left, right),
            None => crate::kernels::monad(id.spelling(), right),
        },
        FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert) => {
            if left.is_some() {
                return Err(Error::Unsupported("dyadic insert-derived call".into()));
            }
            let operand = semantic_function_operand(function)
                .ok_or_else(|| Error::Unsupported("malformed insert semantic entity".into()))?;
            execute_derived_reduction(operand, right)
        }
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank) => {
            let operand = semantic_function_operand(function)
                .ok_or_else(|| Error::Unsupported("malformed rank semantic entity".into()))?;
            let ranks = semantic_rank_triplet(function)
                .ok_or_else(|| Error::Unsupported("unsupported rank semantic entity".into()))?;
            execute_ranked_semantic(operand, ranks, left, right)
        }
        FunctionHead::NameRef(_) => Err(Error::Unsupported(
            "A3 reference executor does not resolve dynamic calls".into(),
        )),
        FunctionHead::PrimitiveAdverb(_)
        | FunctionHead::PrimitiveConjunction(_)
        | FunctionHead::DefinitionConstructor(_)
        | FunctionHead::ExplicitDefinition(_)
        | FunctionHead::ModifierTrain
        | FunctionHead::Hook
        | FunctionHead::Fork => Err(Error::Unsupported(
            "A3 reference executor does not implement this semantic function".into(),
        )),
    }
}

fn execute_call(call: &crate::logical_ir::CallOp, values: &[Option<Value>]) -> Result<Value> {
    let right = value_at(values, call.right)?.clone();
    let left = call
        .left
        .map(|left| value_at(values, left).cloned())
        .transpose()?;
    execute_semantic(&call.callable.semantic, left, right)
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

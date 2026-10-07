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
    match function.operands.first()? {
        FunctionOperand::Function(function) => Some(function),
        FunctionOperand::Noun { .. } => None,
    }
}

fn semantic_rank_triplet(function: &FunctionEntity) -> Option<[i64; 3]> {
    semantic_function_operand(function)?;
    function.requested_ranks()
}

pub(crate) fn cell_rank(array_rank: usize, requested: i64) -> usize {
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

/// Conservative value-only fill witness for a concrete semantic entity.
/// Nested Rank of an already accepted primitive is another structural
/// cell-application boundary; dynamic names and user functions remain barred.
fn has_value_only_rank_fill_semantics(function: &FunctionEntity) -> bool {
    match &function.head {
        FunctionHead::PrimitiveVerb(_) => true,
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank) => {
            function.requested_ranks().is_some()
                && semantic_function_operand(function)
                    .is_some_and(|operand| has_value_only_rank_fill_semantics(operand))
        }
        _ => false,
    }
}

fn execute_ranked_semantic(
    function: &FunctionEntity,
    ranks: [i64; 3],
    left: Option<Value>,
    right: Value,
) -> Result<Value> {
    // Admit only a concrete primitive or a structural Rank chain above one.
    // Unknown and user-defined functions may execute observable fill effects.
    let primitive_fill = has_value_only_rank_fill_semantics(function);
    let atomic_add = matches!(
        function.head,
        FunctionHead::PrimitiveVerb(crate::primitive::PrimitiveId::Add)
    );
    let primitive_catenate = matches!(
        function.head,
        FunctionHead::PrimitiveVerb(crate::primitive::PrimitiveId::Ravel)
    );
    apply_ranked(
        ranks,
        left,
        right,
        primitive_fill,
        atomic_add,
        primitive_catenate,
        |x, y| execute_semantic(function, x, y),
    )
}

/// Interpret only a *value-only, zero-result-frame* fill-cell call.
/// Pinned jsource jsrc/cr.c::jtrank1ex/jtrank2ex substitutes a scalar 0
/// after non-exigent computation failure. RustJ currently coalesces several
/// J internal errors into Domain; handle only that verified public error class
/// here, while leaving resource/unsupported/unknown errors observable.
/// Never call this on ordinary (including empty) cells or user-defined verbs.
/// EVINHOMO type retry, other non-exigent classes, and effectful definitions
/// remain separate semantic obligations (RK-07/RK-08).
/// The primitive + has J scalar cells even when an enclosing Rank applies
/// whole vector cells. A char/numeric scalar fill failure produces one
/// integer-zero result per atom, not one scalar for the entire outer cell.
/// This is a verified *shape witness* for this primitive only.
pub(crate) fn atomic_add_mixed_char_fill_shape(left: &Value, right: &Value) -> Option<Vec<usize>> {
    let left_char = left.type_code() == 2;
    let right_char = right.type_code() == 2;
    if left_char == right_char {
        return None;
    }
    let (short, long) = if left.shape().len() <= right.shape().len() {
        (left.shape(), right.shape())
    } else {
        (right.shape(), left.shape())
    };
    long.starts_with(short).then(|| long.to_vec())
}

/// Explicit proof token for a value-only fill-cell call on a zero result
/// frame. No error code, input dtype or zero atom count may synthesize it.
/// Ordinary cells and unresolved/effectful calls pass no token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VerifiedValueOnlyZeroFrame;

pub(crate) fn recover_zero_frame_fill_domain(
    outcome: Result<Value>,
    atomic_add_cell_shape: Option<&[usize]>,
    proof: Option<VerifiedValueOnlyZeroFrame>,
) -> Result<Value> {
    match outcome {
        Err(error) if proof.is_some() && matches!(error.root(), Error::Domain) => {
            if let Some(shape) = atomic_add_cell_shape {
                let n = crate::value::count(shape)?;
                return Value::ints(shape.to_vec(), crate::value::generate(n, |_| 0)?);
            }
            Ok(Value::scalar(0))
        }
        other => other,
    }
}

/// J dense type priorities from pinned jsrc/j.h::TYPEPRIORITY.
fn dense_rank_type_priority(code: i32) -> Option<u8> {
    match code {
        1 => Some(0), // B01
        2 => Some(1), // LIT
        4 => Some(4), // INT
        8 => Some(8), // FL
        _ => None,
    }
}

/// A narrow internal-EVINHOMO witness for the *concrete catenate verb*.
/// C jsrc/vf.c + jsrc/cr.c distinguish this from a plain arithmetic Domain.
/// Only mixed char/numeric dense cells can enter this supported retry.
/// A normal/user-defined error never acquires retry permission from Domain alone.
pub(crate) fn inhomogeneous_catenate_retry_type(
    left: &Value,
    right: &Value,
    original_left_has_atoms: bool,
    original_right_has_atoms: bool,
) -> Option<i32> {
    let lt = left.type_code();
    let rt = right.type_code();
    if (lt == 2) == (rt == 2) {
        return None;
    }
    let lp = dense_rank_type_priority(lt)?;
    let rp = dense_rank_type_priority(rt)?;
    if original_left_has_atoms {
        Some(lt)
    } else if original_right_has_atoms {
        Some(rt)
    } else if lp > rp {
        Some(lt)
    } else {
        Some(rt)
    }
}

/// Simulate pinned cr.c::jtrank2ex's EVINHOMO branch without erasing the
/// initial error or changing the operands of non-fill real-cell calls.
/// First call is mandatory for this pure, concrete built-in only. If it fails
/// with a recognized type-inhomogeneity Domain, rebuild *only the mismatched
/// type's synthetic fill cell* and retry once, preserving every cell shape.
pub(crate) fn retry_inhomogeneous_catenate_fill(
    original_left: &Value,
    original_right: &Value,
    left_fill: Value,
    right_fill: Value,
    mut call: impl FnMut(Value, Value) -> Result<Value>,
) -> Result<Value> {
    let retry_type = inhomogeneous_catenate_retry_type(
        &left_fill,
        &right_fill,
        !original_left.is_empty(),
        !original_right.is_empty(),
    );
    let Some(target) = retry_type else {
        return call(left_fill, right_fill);
    };
    let first = call(left_fill.clone(), right_fill.clone());
    if !matches!(&first, Err(error) if matches!(error.root(), Error::Domain)) {
        return first;
    }
    let left = if left_fill.type_code() == target {
        left_fill
    } else {
        left_fill.rank_refill_as(target)?
    };
    let right = if right_fill.type_code() == target {
        right_fill
    } else {
        right_fill.rank_refill_as(target)?
    };
    call(left, right)
}

pub(crate) fn apply_ranked(
    ranks: [i64; 3],
    left: Option<Value>,
    right: Value,
    primitive_fill: bool,
    atomic_add: bool,
    primitive_catenate: bool,
    mut call: impl FnMut(Option<Value>, Value) -> Result<Value>,
) -> Result<Value> {
    if let Some(left) = left {
        let ar = cell_rank(left.shape().len(), ranks[1]);
        let br = cell_rank(right.shape().len(), ranks[2]);
        let af = left.shape()[..left.shape().len() - ar].to_vec();
        let bf = right.shape()[..right.shape().len() - br].to_vec();

        if af.is_empty() && bf.is_empty() {
            return call(Some(left), right);
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
            if !primitive_fill {
                return Err(Error::Unsupported(
                    "rank fill execution for user-defined function".into(),
                ));
            }
            // jsource cr.c evaluates one rank fill-cell to determine result
            // type and shape, then prepends the empty output frame.
            // This is not a physical search strategy.
            let x = left.rank_fill_cell(ar)?;
            let y = right.rank_fill_cell(br)?;
            let atomic_shape = atomic_add
                .then(|| atomic_add_mixed_char_fill_shape(&x, &y))
                .flatten();
            let outcome = if primitive_catenate {
                retry_inhomogeneous_catenate_fill(&left, &right, x, y, |x, y| call(Some(x), y))
            } else {
                call(Some(x), y)
            };
            let prototype = recover_zero_frame_fill_domain(
                outcome,
                atomic_shape.as_deref(),
                Some(VerifiedValueOnlyZeroFrame),
            )?;
            return prototype.empty_rank_result(&frame);
        }
        let ad = crate::value::count(&frame[af.len()..])?;
        let bd = crate::value::count(&frame[bf.len()..])?;
        let cells = (0..frames).map(|i| {
            let x = left.view().cell(ar, i / ad)?.to_owned()?;
            let y = right.view().cell(br, i / bd)?.to_owned()?;
            call(Some(x), y)
        });
        assemble_uniform_cells(frame, cells)
    } else {
        let rank = cell_rank(right.shape().len(), ranks[0]);
        let frame_rank = right.shape().len() - rank;
        if frame_rank == 0 {
            return call(None, right);
        }
        let frame = right.shape()[..frame_rank].to_vec();
        let frames = crate::value::count(&frame)?;
        if frames == 0 {
            if !primitive_fill {
                return Err(Error::Unsupported(
                    "rank fill execution for user-defined function".into(),
                ));
            }
            let fill = right.rank_fill_cell(rank)?;
            let prototype = recover_zero_frame_fill_domain(
                call(None, fill),
                None,
                Some(VerifiedValueOnlyZeroFrame),
            )?;
            return prototype.empty_rank_result(&frame);
        }
        let cells = (0..frames).map(|i| {
            let cell = right.view().cell(rank, i)?.to_owned()?;
            call(None, cell)
        });
        assemble_uniform_cells(frame, cells)
    }
}

fn execute_derived_reduction(function: &FunctionEntity, right: Value) -> Result<Value> {
    if let FunctionHead::PrimitiveVerb(id) = function.head {
        return crate::kernels::reduce(id.spelling(), right);
    }
    apply_reduction(right, |x, y| execute_semantic(function, x, y))
}

pub(crate) fn apply_reduction(
    right: Value,
    mut call: impl FnMut(Option<Value>, Value) -> Result<Value>,
) -> Result<Value> {
    if right.is_sparse() {
        return Err(Error::Unsupported("sparse derived reduction".into()));
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
        out = call(Some(left), out)?;
    }
    Ok(out)
}

fn execute_semantic(function: &FunctionEntity, left: Option<Value>, right: Value) -> Result<Value> {
    match &function.head {
        FunctionHead::PrimitiveVerb(id) => match left {
            // FW-02: independent dense sequential oracle, no physical routing.
            Some(left) => match id {
                crate::primitive::PrimitiveId::IndexOf => {
                    crate::search_reference::index_of(&left, &right, false)
                }
                crate::primitive::PrimitiveId::Steps => {
                    crate::search_reference::index_of(&left, &right, true)
                }
                crate::primitive::PrimitiveId::Member => {
                    crate::search_reference::member(&left, &right)
                }
                _ => crate::kernels::dyad(id.spelling(), left, right),
            },
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
        FunctionHead::VocabularyPrimitive(_)
        | FunctionHead::TakeName { .. }
        | FunctionHead::PrimitiveAdverb(_)
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

#[cfg(test)]
mod rank_fill_error_tests {
    use super::*;

    #[test]
    fn catenate_inhomogeneity_retries_once_and_uses_original_atom_presence() {
        use crate::storage::CpuStorage;
        use crate::value::Data;

        let char_empty = Value::new([0, 3], Data::Char(CpuStorage::new(vec![]))).unwrap();
        let int_empty = Value::ints([0, 3], vec![]).unwrap();
        let char_fill = char_empty.rank_fill_cell(1).unwrap();
        let int_fill = int_empty.rank_fill_cell(1).unwrap();
        assert_eq!(
            inhomogeneous_catenate_retry_type(&char_fill, &int_fill, false, false),
            Some(4),
        );

        let mut calls = 0;
        let result = retry_inhomogeneous_catenate_fill(
            &char_empty,
            &int_empty,
            char_fill.clone(),
            int_fill.clone(),
            |a, b| {
                calls += 1;
                crate::kernels::dyad(",", a, b)
            },
        )
        .unwrap();
        assert_eq!(calls, 2, "initial EVINHOMO and exactly one retry");
        assert_eq!(result.shape(), &[6]);
        assert_eq!(result.type_code(), 4);
        for i in 0..result.len() {
            assert_eq!(result.int_at(i).unwrap(), 0);
        }

        let char_nonempty = Value::new([3], Data::Char(CpuStorage::new(b"abc".to_vec()))).unwrap();
        assert_eq!(
            inhomogeneous_catenate_retry_type(&char_nonempty, &int_fill, true, false,),
            Some(2),
        );
        let char_result = retry_inhomogeneous_catenate_fill(
            &char_nonempty,
            &int_empty,
            char_nonempty.clone(),
            int_fill.clone(),
            |a, b| crate::kernels::dyad(",", a, b),
        )
        .unwrap();
        assert_eq!(char_result.shape(), &[6]);
        assert_eq!(char_result.type_code(), 2);
        assert_eq!(char_result.display(), "abc   ");

        let bool_fill = Value::new([3], Data::Bool(CpuStorage::new(vec![0; 3]))).unwrap();
        assert_eq!(
            inhomogeneous_catenate_retry_type(&char_fill, &bool_fill, false, false),
            Some(2),
        );

        let mut count = 0;
        let err = retry_inhomogeneous_catenate_fill(
            &char_empty,
            &int_empty,
            char_fill,
            int_fill,
            |_, _| {
                count += 1;
                Err(Error::Limit)
            },
        )
        .unwrap_err();
        assert_eq!(err.kind(), "limit error");
        assert_eq!(count, 1, "resource failures must never retry");
    }

    #[test]
    fn rank_fill_recovery_requires_verified_prototype_origin() {
        for scenario in ["ordinary cell", "unknown or effectful"] {
            let original = Error::Domain.at(1..4);
            let unchanged = recover_zero_frame_fill_domain(Err(original.clone()), Some(&[3]), None)
                .expect_err("an ordinary or unproven Domain is observable");
            assert_eq!(unchanged, original, "{scenario}");
        }

        // Even a verified zero-frame call never converts a resource or
        // unsupported-function failure into an integer prototype.
        for error in [Error::Limit, Error::Unsupported("unknown call".into())] {
            let expected = error.clone();
            assert_eq!(
                recover_zero_frame_fill_domain(
                    Err(error),
                    Some(&[3]),
                    Some(VerifiedValueOnlyZeroFrame),
                )
                .unwrap_err(),
                expected,
            );
        }
    }

    #[test]
    fn rank_zero_frame_recovery_does_not_erase_exigent_or_unknown_errors() {
        let fallback = recover_zero_frame_fill_domain(
            Err(Error::Domain),
            None,
            Some(VerifiedValueOnlyZeroFrame),
        )
        .expect("J non-exigent computational fill failure");
        assert_eq!(fallback.type_code(), 4);
        assert_eq!(fallback.shape(), &[]);
        assert_eq!(fallback.int_at(0).unwrap(), 0);

        // The diagnostic wrapper must not accidentally change the J class.
        let wrapped = Error::Domain.at(1..4);
        let fallback =
            recover_zero_frame_fill_domain(Err(wrapped), None, Some(VerifiedValueOnlyZeroFrame))
                .unwrap();
        assert_eq!(fallback.int_at(0).unwrap(), 0);

        for error in [
            Error::Length,
            Error::Rank,
            Error::Index,
            Error::Limit,
            Error::Valence,
            Error::Value("unresolved".into()),
            Error::Unsupported("effectful or unknown call".into()),
        ] {
            let expected = error.kind();
            let observed =
                recover_zero_frame_fill_domain(Err(error), None, Some(VerifiedValueOnlyZeroFrame))
                    .unwrap_err();
            assert_eq!(observed.kind(), expected);
        }
    }
}

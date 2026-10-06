//! M4's bounded monadic reindex realization within the existing PhysicalPlan.
//! No additional canonical IR, runtime fallback, threads, or device assumptions.

use super::*;
use crate::{
    execution_semantics::{CallTarget, ExecutionBasisKind},
    logical_ir::{ExecutionBasisPayload, ReindexKind},
    lowering::{LoweringRegistry, TargetCapabilities},
    primitive::PrimitiveId,
    storage::{CpuStorage, CpuView},
};

struct Reindex<'a> {
    value: &'a Value,
    kind: ReindexKind,
}

fn semantic_reindex(logical: &LogicalPlan) -> PlanResult<Reindex<'_>> {
    checked_a3(logical)?;
    if logical.write.is_some() || logical.operations.len() != 2 || logical.values.len() != 2 {
        return Err(PhysicalPlanError::Unsupported(
            "reindex route requires exactly one literal and one pure monadic operation",
        ));
    }
    let (OpKind::Literal(value), OpKind::Basis { kind, payload, call }) =
        (&logical.operations[0].kind, &logical.operations[1].kind)
    else {
        return Err(PhysicalPlanError::Unsupported(
            "reindex route does not support named inputs or dynamic calls",
        ));
    };
    let ExecutionBasisPayload::StaticReindex { kind: reindex } = payload else {
        return Err(PhysicalPlanError::Unsupported("missing reindex payload"));
    };
    if *kind != ExecutionBasisKind::StaticReindex
        || call.left.is_some()
        || call.right != ValueId(0)
        || call.rank_plan.is_some()
        || !call.effect.is_pure()
        || call.constraints.unresolved().next().is_some()
    {
        return Err(PhysicalPlanError::Unsupported(
            "reindex call has unproven rank, effects or constraints",
        ));
    }
    match (*reindex, call.callable.target) {
        (ReindexKind::Reverse, CallTarget::Primitive(PrimitiveId::Reverse))
        | (ReindexKind::Transpose, CallTarget::Primitive(PrimitiveId::Transpose)) => {}
        _ => {
            return Err(PhysicalPlanError::Unsupported(
                "only resolved primitive monadic Reverse/Transpose are supported",
            ));
        }
    }
    if !LoweringRegistry::a3_v0()
        .legal_candidates(*kind, call, &TargetCapabilities::cpu_baseline())
        .contains(&RealizationFamily::MetadataOrIndexReindex)
    {
        return Err(PhysicalPlanError::Unsupported(
            "CPU static reindex lowering is not proven legal",
        ));
    }
    if logical.result != Some(ValueId(1))
        || logical.values[0].producer != OpId(0)
        || logical.values[1].producer != OpId(1)
        || logical.operations[0].results.as_slice() != [ValueId(0)]
        || logical.operations[1].results.as_slice() != [ValueId(1)]
    {
        return Err(PhysicalPlanError::Unsupported(
            "reindex route must retain the exact A3 def/use/result chain",
        ));
    }
    dense_encoding(value).ok_or(PhysicalPlanError::Unsupported(
        "boxed and sparse reindex routes are not yet supported",
    ))?;
    Ok(Reindex {
        value,
        kind: *reindex,
    })
}

fn view_recipe(input: &Value, kind: ReindexKind) -> PlanResult<(Vec<usize>, Vec<isize>, isize)> {
    let mut shape = input.shape().to_vec();
    let mut strides = row_major_strides(&shape)?;
    let mut offset = 0isize;
    match kind {
        ReindexKind::Reverse => {
            // Monadic J |. reverses the leading axis, not each scalar atom.
            if !shape.is_empty() && input.len() > 0 {
                let index = isize::try_from(shape[0] - 1)
                    .map_err(|_| PhysicalPlanError::Unsupported("reverse extent overflow"))?;
                offset = index.checked_mul(strides[0])
                    .ok_or(PhysicalPlanError::Unsupported("reverse offset overflow"))?;
                strides[0] = -strides[0];
            }
        }
        ReindexKind::Transpose => {
            // Monadic J |: reverses the axis order.
            shape.reverse();
            strides.reverse();
        }
        _ => return Err(PhysicalPlanError::Unsupported("unproven view mapping")),
    }
    Ok((shape, strides, offset))
}

fn descriptors(
    input: &Value,
    kind: ReindexKind,
) -> PlanResult<(Vec<BufferRequirement>, Vec<PlannedView>, Vec<PhysicalOp>)> {
    let encoding = dense_encoding(input)
        .ok_or(PhysicalPlanError::Unsupported("unknown CPU encoding"))?;
    let (shape, strides, offset) = view_recipe(input, kind)?;
    let buffers = vec![
        BufferRequirement {
            memory: MemorySpace::Host,
            encoding,
            atoms: input.len(),
            alignment: 1,
            ownership: BufferOwnership::Input,
        },
        BufferRequirement {
            memory: MemorySpace::Host,
            encoding,
            atoms: input.len(),
            alignment: 1,
            ownership: BufferOwnership::Output,
        },
    ];
    let views = vec![
        PlannedView {
            buffer: PlanBufferId(0),
            encoding,
            shape: input.shape().to_vec(),
            strides: row_major_strides(input.shape())?,
            offset: 0,
            access: ViewAccess::ReadOnly,
        },
        PlannedView {
            buffer: PlanBufferId(0),
            encoding,
            shape: shape.clone(),
            strides,
            offset,
            access: ViewAccess::ReadOnly,
        },
        PlannedView {
            buffer: PlanBufferId(1),
            encoding,
            shape: shape.clone(),
            strides: row_major_strides(&shape)?,
            offset: 0,
            access: ViewAccess::ReadOnly,
        },
    ];
    let ops = vec![
        PhysicalOp::BindInput {
            source_op: OpId(0),
            logical_value: ValueId(0),
            buffer: PlanBufferId(0),
            view: PhysicalViewId(0),
        },
        PhysicalOp::View {
            input: PhysicalViewId(0),
            result: PhysicalViewId(1),
        },
        PhysicalOp::Materialize {
            input: PhysicalViewId(1),
            result: PhysicalViewId(2),
        },
        PhysicalOp::Return {
            logical_value: ValueId(1),
            view: PhysicalViewId(2),
        },
    ];
    Ok((buffers, views, ops))
}

pub(super) fn build(logical: &LogicalPlan) -> PlanResult<PhysicalPlan> {
    let Reindex { value, kind } = semantic_reindex(logical)?;
    let (buffers, views, operations) = descriptors(value, kind)?;
    let plan = PhysicalPlan {
        source_header: logical.header.clone(),
        source_text: logical.source.clone(),
        device: ExecutionDevice::Cpu,
        buffers,
        views,
        operations,
    };
    plan.verify(logical)?;
    Ok(plan)
}

pub(super) fn verify(plan: &PhysicalPlan, logical: &LogicalPlan) -> PlanResult<()> {
    let Reindex { value, kind } = semantic_reindex(logical)?;
    let (buffers, views, ops) = descriptors(value, kind)?;
    if let Some(known_shape) = &logical.values[1].facts.shape {
        if *known_shape != views[1].shape {
            return Err(PhysicalPlanError::Invalid(
                "reindex view contradicts known A3 result shape",
            ));
        }
    }
    if plan.buffers != buffers || plan.views != views || plan.operations != ops {
        return Err(PhysicalPlanError::Invalid(
            "invalid M4 reindex buffer, view span, materialization or op order",
        ));
    }
    Ok(())
}

fn materialize(view: &PhysicalArray) -> PlanResult<Value> {
    // A read-only view may have negative or permuted strides. Iterate J's
    // logical row-major order rather than the physical backing order.
    let index = |mut flat: usize| -> PlanResult<usize> {
        let mut coords = vec![0usize; view.shape().len()];
        for axis in (0..coords.len()).rev() {
            let dim = view.shape()[axis];
            if dim == 0 {
                return Err(PhysicalPlanError::Invalid("index into zero-atom view"));
            }
            coords[axis] = flat % dim;
            flat /= dim;
        }
        view.backing_index(&coords).map_err(PhysicalPlanError::Storage)
    };
    let data = match view.buffer().backing_view() {
        CpuView::Bool(backing) => {
            let mut out = crate::value::buffer(view.len()).map_err(PhysicalPlanError::Storage)?;
            for i in 0..view.len() { out.push(backing[index(i)?]); }
            Data::Bool(CpuStorage::new(out))
        }
        CpuView::Int(backing) => {
            let mut out = crate::value::buffer(view.len()).map_err(PhysicalPlanError::Storage)?;
            for i in 0..view.len() { out.push(backing[index(i)?]); }
            Data::Int(CpuStorage::new(out))
        }
        CpuView::Float(backing) => {
            let mut out = crate::value::buffer(view.len()).map_err(PhysicalPlanError::Storage)?;
            for i in 0..view.len() { out.push(backing[index(i)?]); }
            Data::Float(CpuStorage::new(out))
        }
        CpuView::Char(backing) => {
            let mut out = crate::value::buffer(view.len()).map_err(PhysicalPlanError::Storage)?;
            for i in 0..view.len() { out.push(backing[index(i)?]); }
            Data::Char(CpuStorage::new(out))
        }
        _ => return Err(PhysicalPlanError::Unsupported("not a supported dense CPU backing")),
    };
    Value::new(view.shape().to_vec(), data).map_err(PhysicalPlanError::Storage)
}

pub(super) fn execute(plan: &PhysicalPlan, logical: &LogicalPlan) -> PlanResult<Value> {
    plan.verify(logical)?;
    let Reindex { value, .. } = semantic_reindex(logical)?;
    let mut registry = BufferRegistry::new().map_err(PhysicalPlanError::Storage)?;
    let input = PhysicalArray::from_value(&mut registry, value.clone().into_shared())
        .map_err(PhysicalPlanError::Storage)?;
    let view = &plan.views[1];
    let remapped = PhysicalArray::new(
        input.buffer().clone(),
        view.shape.clone(),
        view.strides.clone(),
        view.offset,
    )
    .map_err(PhysicalPlanError::Storage)?;
    if remapped.shape() != view.shape.as_slice()
        || remapped.strides() != view.strides.as_slice()
        || remapped.offset() != view.offset
        || remapped.encoding() != view.encoding
    {
        return Err(PhysicalPlanError::Invalid(
            "runtime affine view does not match the verified plan",
        ));
    }
    materialize(&remapped)
}

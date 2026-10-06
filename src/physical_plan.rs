//! Verified, plan-time physical identities for the first RustJ-native CPU slice.
//!
//! This is not the semantic A3 plan and not the runtime BufferRegistry.
//! M4 v0 intentionally executes only a single closed dense literal, or a
//! completely empty A3 plan. Other A3 operations fail closed. In particular,
//! SemanticCheck, name reads, writes and dynamic calls may not be skipped.
//!
//! A logical ValueId, a PlanBufferId, a PhysicalViewId and a runtime BufferId
//! are distinct identities. CPU execution and host memory are separate axes.

use crate::{
    Error, Value,
    logical_ir::{IrHeader, OpId, OpKind, Plan as LogicalPlan, ValueId},
    lowering::RealizationFamily,
    physical::{BufferRegistry, Encoding, PhysicalArray},
    value::Data,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlanBufferId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PhysicalViewId(pub usize);

/// Chosen execution device, independent of the backing memory space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionDevice {
    Cpu,
}

/// Memory placement is not implied by the execution device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemorySpace {
    Host,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferOwnership {
    Input,
    Temporary,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewAccess {
    ReadOnly,
    ExclusiveWrite,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferRequirement {
    pub memory: MemorySpace,
    pub encoding: Encoding,
    pub atoms: usize,
    pub alignment: usize,
    pub ownership: BufferOwnership,
}

/// Symbolic view specification; never a live BufferLease or physical address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedView {
    pub buffer: PlanBufferId,
    pub encoding: Encoding,
    pub shape: Vec<usize>,
    pub strides: Vec<isize>,
    pub offset: isize,
    pub access: ViewAccess,
}

/// Future operation forms are declared, but deliberately rejected by the
/// identity-slice verifier until their semantic and lifetime proofs exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhysicalOp {
    BindInput {
        source_op: OpId,
        logical_value: ValueId,
        buffer: PlanBufferId,
        view: PhysicalViewId,
    },
    Check {
        source_op: OpId,
    },
    View {
        input: PhysicalViewId,
        result: PhysicalViewId,
    },
    Materialize {
        input: PhysicalViewId,
        result: PhysicalViewId,
    },
    Kernel {
        source_op: OpId,
        inputs: Vec<PhysicalViewId>,
        output: PhysicalViewId,
        realization: RealizationFamily,
    },
    Return {
        logical_value: ValueId,
        view: PhysicalViewId,
    },
}

#[derive(Clone, Debug)]
pub struct PhysicalPlan {
    pub source_header: IrHeader,
    pub source_text: String,
    pub device: ExecutionDevice,
    pub buffers: Vec<BufferRequirement>,
    pub views: Vec<PlannedView>,
    pub operations: Vec<PhysicalOp>,
}

#[derive(Debug)]
pub enum PhysicalPlanError {
    /// Malformed plan, lost A3 checks or incompatible provenance.
    Invalid(&'static str),
    /// Valid J/Logical IR not supported by this deliberately narrow M4 route.
    Unsupported(&'static str),
    /// Executor storage/infrastructure failure, not a J semantic error.
    Storage(Error),
    /// Failure of the upstream A3 verifier.
    Logical(String),
}

impl std::fmt::Display for PhysicalPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(reason) => write!(f, "invalid M4 PhysicalPlan: {reason}"),
            Self::Unsupported(reason) => write!(f, "unsupported M4 PhysicalPlan route: {reason}"),
            Self::Storage(error) => write!(f, "M4 PhysicalPlan storage failure: {error}"),
            Self::Logical(error) => write!(f, "invalid upstream A3 plan: {error}"),
        }
    }
}
impl std::error::Error for PhysicalPlanError {}

type PlanResult<T> = std::result::Result<T, PhysicalPlanError>;

fn dense_encoding(value: &Value) -> Option<Encoding> {
    match value.data() {
        Data::Bool(_) => Some(Encoding::BoolByte),
        Data::Int(_) => Some(Encoding::Int64),
        Data::Float(_) => Some(Encoding::Float64),
        Data::Char(_) => Some(Encoding::Char8),
        Data::Sparse(_) | Data::Boxed(_) => None,
    }
}

fn row_major_strides(shape: &[usize]) -> PlanResult<Vec<isize>> {
    let mut strides = vec![0isize; shape.len()];
    if shape.contains(&0) {
        return Ok(strides);
    }
    let mut stride = 1isize;
    for axis in (0..shape.len()).rev() {
        strides[axis] = stride;
        let dim = isize::try_from(shape[axis])
            .map_err(|_| PhysicalPlanError::Unsupported("CPU stride extent exceeds isize"))?;
        stride = stride
            .checked_mul(dim)
            .ok_or(PhysicalPlanError::Unsupported("CPU stride extent overflow"))?;
    }
    Ok(strides)
}

fn checked_a3(logical: &LogicalPlan) -> PlanResult<()> {
    logical
        .verify()
        .map_err(|error| PhysicalPlanError::Logical(error.to_string()))
}

impl PhysicalPlan {
    /// Build an empty CPU plan for a genuinely empty, result-less A3 block.
    /// This is not the same as a J array with a zero-length axis.
    pub fn empty_from_a3(logical: &LogicalPlan) -> PlanResult<Self> {
        checked_a3(logical)?;
        if !logical.operations.is_empty()
            || !logical.values.is_empty()
            || logical.result.is_some()
            || logical.write.is_some()
        {
            return Err(PhysicalPlanError::Unsupported(
                "empty plan requires no A3 operations, values, result or write",
            ));
        }
        let plan = Self {
            source_header: logical.header.clone(),
            source_text: logical.source.clone(),
            device: ExecutionDevice::Cpu,
            buffers: Vec::new(),
            views: Vec::new(),
            operations: Vec::new(),
        };
        plan.verify(logical)?;
        Ok(plan)
    }

    /// Initial M4 compiler-native realization: one closed dense literal,
    /// no SemanticCheck, no dynamic namespace, and no other computation.
    /// Later planners must use their own legality-checked constructors.
    pub fn identity_literal(logical: &LogicalPlan) -> PlanResult<Self> {
        checked_a3(logical)?;
        if logical.write.is_some() || logical.operations.len() != 1 || logical.values.len() != 1 {
            return Err(PhysicalPlanError::Unsupported(
                "identity route requires exactly one closed literal with no write",
            ));
        }
        let result = logical.result.ok_or(PhysicalPlanError::Unsupported(
            "identity route requires one returned value",
        ))?;
        let source_op = logical.values[result.0].producer;
        if source_op != OpId(0) || logical.operations[0].results.as_slice() != [result] {
            return Err(PhysicalPlanError::Unsupported(
                "identity route requires the literal to produce the returned value",
            ));
        }
        let OpKind::Literal(value) = &logical.operations[0].kind else {
            return Err(PhysicalPlanError::Unsupported(
                "identity route cannot bypass J calls or semantic checks",
            ));
        };
        let encoding = dense_encoding(value).ok_or(PhysicalPlanError::Unsupported(
            "identity route supports dense CPU encodings only",
        ))?;
        let shape = value.shape().to_vec();
        let strides = row_major_strides(&shape)?;
        let plan = Self {
            source_header: logical.header.clone(),
            source_text: logical.source.clone(),
            device: ExecutionDevice::Cpu,
            buffers: vec![BufferRequirement {
                memory: MemorySpace::Host,
                encoding,
                atoms: value.len(),
                alignment: 1,
                ownership: BufferOwnership::Input,
            }],
            views: vec![PlannedView {
                buffer: PlanBufferId(0),
                encoding,
                shape,
                strides,
                offset: 0,
                access: ViewAccess::ReadOnly,
            }],
            operations: vec![
                PhysicalOp::BindInput {
                    source_op,
                    logical_value: result,
                    buffer: PlanBufferId(0),
                    view: PhysicalViewId(0),
                },
                PhysicalOp::Return {
                    logical_value: result,
                    view: PhysicalViewId(0),
                },
            ],
        };
        plan.verify(logical)?;
        Ok(plan)
    }

    /// Fail-closed verifier: v0 recognizes exactly the empty plan or the
    /// literal BindInput -> Return route. It cannot silently drop an A3 Check,
    /// effect, state mutation or unsupported kernel.
    pub fn verify(&self, logical: &LogicalPlan) -> PlanResult<()> {
        checked_a3(logical)?;
        if self.source_header != logical.header || self.source_text != logical.source {
            return Err(PhysicalPlanError::Invalid("stale A3 schema or source provenance"));
        }
        if self.device != ExecutionDevice::Cpu {
            return Err(PhysicalPlanError::Invalid("M4 v0 must execute on CPU"));
        }
        if logical.operations.is_empty() {
            if logical.values.is_empty()
                && logical.result.is_none()
                && logical.write.is_none()
                && self.buffers.is_empty()
                && self.views.is_empty()
                && self.operations.is_empty()
            {
                return Ok(());
            }
            return Err(PhysicalPlanError::Invalid(
                "empty A3 block requires an empty physical plan",
            ));
        }
        if logical.write.is_some() || logical.operations.len() != 1 || logical.values.len() != 1 {
            return Err(PhysicalPlanError::Unsupported(
                "M4 v0 cannot drop A3 operations, writes or SemanticChecks",
            ));
        }
        let result = logical.result.ok_or(PhysicalPlanError::Unsupported(
            "M4 v0 requires one returned literal",
        ))?;
        let source_op = logical.values[result.0].producer;
        if source_op != OpId(0) || logical.operations[0].results.as_slice() != [result] {
            return Err(PhysicalPlanError::Invalid(
                "result does not match its A3 literal producer",
            ));
        }
        let OpKind::Literal(value) = &logical.operations[0].kind else {
            return Err(PhysicalPlanError::Unsupported(
                "M4 v0 cannot execute a non-literal operation",
            ));
        };
        let encoding = dense_encoding(value).ok_or(PhysicalPlanError::Unsupported(
            "M4 v0 does not support boxed or sparse physical binding",
        ))?;

        if self.buffers.len() != 1 || self.views.len() != 1 || self.operations.len() != 2 {
            return Err(PhysicalPlanError::Invalid(
                "literal route must have exactly one buffer, one view, BindInput and Return",
            ));
        }
        let buffer = &self.buffers[0];
        if buffer.memory != MemorySpace::Host
            || buffer.encoding != encoding
            || buffer.atoms != value.len()
            || buffer.alignment != 1
            || buffer.ownership != BufferOwnership::Input
        {
            return Err(PhysicalPlanError::Invalid(
                "invalid host input buffer encoding, extent, alignment or ownership",
            ));
        }
        let view = &self.views[0];
        if view.buffer != PlanBufferId(0)
            || view.encoding != encoding
            || view.shape.as_slice() != value.shape()
            || view.strides != row_major_strides(value.shape())?
            || view.offset != 0
            || view.access != ViewAccess::ReadOnly
        {
            return Err(PhysicalPlanError::Invalid(
                "invalid read-only contiguous CPU view or backing extent",
            ));
        }
        if !matches!(
            &self.operations[0],
            PhysicalOp::BindInput {
                source_op: OpId(0),
                logical_value,
                buffer: PlanBufferId(0),
                view: PhysicalViewId(0)
            } if *logical_value == result
        ) {
            return Err(PhysicalPlanError::Invalid(
                "source input must be bound before its first use",
            ));
        }
        if !matches!(
            &self.operations[1],
            PhysicalOp::Return {
                logical_value,
                view: PhysicalViewId(0)
            } if *logical_value == result
        ) {
            return Err(PhysicalPlanError::Invalid(
                "Return must reference the live input view and A3 result",
            ));
        }
        Ok(())
    }

    /// Execute only an already verified identity route. The returned noun
    /// shares the validated CPU backing; no kernel, transfer or J name lookup
    /// is performed. Non-identity execution belongs to later M4 work.
    pub fn execute_identity(&self, logical: &LogicalPlan) -> PlanResult<Option<Value>> {
        self.verify(logical)?;
        if self.operations.is_empty() {
            return Ok(None);
        }
        let mut registry = BufferRegistry::new().map_err(PhysicalPlanError::Storage)?;
        let mut bound: Option<(Value, PhysicalArray)> = None;
        for op in &self.operations {
            match op {
                PhysicalOp::BindInput { source_op, .. } => {
                    let OpKind::Literal(value) = &logical.operations[source_op.0].kind else {
                        return Err(PhysicalPlanError::Invalid("BindInput source is not a literal"));
                    };
                    let shared = value.clone().into_shared();
                    let array = PhysicalArray::from_value(&mut registry, shared.clone())
                        .map_err(PhysicalPlanError::Storage)?;
                    let spec = &self.views[0];
                    if array.shape() != spec.shape.as_slice()
                        || array.strides() != spec.strides.as_slice()
                        || array.offset() != spec.offset
                        || array.encoding() != spec.encoding
                    {
                        return Err(PhysicalPlanError::Invalid(
                            "executor physical view differs from verified plan",
                        ));
                    }
                    bound = Some((shared, array));
                }
                PhysicalOp::Return { .. } => {
                    let (value, _lease) = bound
                        .as_ref()
                        .ok_or(PhysicalPlanError::Invalid("Return before BindInput"))?;
                    return Ok(Some(value.clone()));
                }
                _ => {
                    return Err(PhysicalPlanError::Unsupported(
                        "M4 v0 has no implementation for this PhysicalOp",
                    ));
                }
            }
        }
        Err(PhysicalPlanError::Invalid("nonempty plan is missing Return"))
    }
}

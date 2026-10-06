//! Checked, read-only CPU affine representations. This is the G1 foundation,
//! not a physical executor or a CUDA ABI. Buffer IDs never encode addresses.
use crate::{
    Data, Error, Result, Value,
    storage::{CpuStorage, CpuView, Shape},
    types::{DType, Scalar},
    value::count,
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

static NEXT_REGISTRY: AtomicU64 = AtomicU64::new(1);

/// Physical search-strategy inputs. These are runtime/target facts, never
/// canonical J values or A3 semantic identities. The index/key span may stay
/// unknown until the relevant input has been inspected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchWorkload {
    pub indexed_items: usize,
    pub query_items: usize,
    pub integer_span: Option<u128>,
    pub immutable_shared_index: bool,
    /// A prehash is a permissible candidate only when the caller already owns
    /// a compatible per-Engine cache. It is not requested by a J name alone.
    pub prehash_available: bool,
    /// Reverse hashing needs the actual query values available for indexing.
    pub allow_reverse: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchSelectionBasis {
    /// No semantics-changing optimization has been selected.
    Reference,
    /// Guarded at execution by exact Int/Bool scalar item/value checks.
    RuntimeExactScalarGuard,
    /// No legal route is registered for this target or J search form.
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchPhysicalChoice {
    pub algorithm: crate::lowering::SearchAlgorithm,
    pub basis: SearchSelectionBasis,
    /// Estimate of temporary dictionary/lookup entries, not bytes or a
    /// performance measurement; optional allocations may fail and fall back.
    pub estimated_table_entries: usize,
}

/// A small deterministic *physical* cost heuristic. Registered candidates are
/// checked against target and search output meaning first (MLIR-style dynamic
/// legality). Actual J comparison/rank/effect proof is NOT inferred from the
/// workload. The caller must have verified exact scalar Int/Bool input shape,
/// or the only available route is reference sequential execution.
///
/// Alternative algorithms are kept visible in the registry; this function
/// picks one conditional implementation without mutating the canonical IR.
/// Thresholds are provisional, not TVM-style measured tuning records.
#[cfg(test)]
thread_local! {
    // Same-thread test-only witness: reference execution must never invoke
    // physical search choice, even when the optimized evaluator does.
    static SEARCH_PLANNER_CALLS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

pub fn plan_search_algorithm(
    output: crate::logical_ir::SearchOutputKind,
    target: &crate::lowering::TargetCapabilities,
    workload: SearchWorkload,
    runtime_exact_scalar_guard: bool,
) -> SearchPhysicalChoice {
    #[cfg(test)]
    SEARCH_PLANNER_CALLS.with(|count| count.set(count.get() + 1));
    use crate::logical_ir::SearchComparison;
    use crate::lowering::{SearchAlgorithm as A, SearchAlgorithmReadiness as R};

    // One shared registry legality rule, without a per-lookup registry/vector
    // allocation. Compiler diagnostics can request the full report separately.
    let status = |algorithm| {
        Some(
            crate::lowering::LoweringRegistry::search_algorithm_readiness(
                output,
                SearchComparison::JEquality,
                true,
                algorithm,
                target,
            ),
        )
    };
    let fallback = SearchPhysicalChoice {
        algorithm: A::Sequential,
        basis: if status(A::Sequential) == Some(R::Baseline) {
            SearchSelectionBasis::Reference
        } else {
            SearchSelectionBasis::Unavailable
        },
        estimated_table_entries: 0,
    };
    if !runtime_exact_scalar_guard || fallback.basis == SearchSelectionBasis::Unavailable {
        return fallback;
    }
    if workload.indexed_items == 0
        || workload.query_items == 0
        || workload.indexed_items.saturating_mul(workload.query_items) <= 32
    {
        return fallback;
    }

    let choice = if workload.prehash_available
        && workload.immutable_shared_index
        && (64..=16_384).contains(&workload.indexed_items)
    {
        A::PreparedHash
    } else if workload.allow_reverse
        && workload.indexed_items >= 64
        && workload.indexed_items / 2 > workload.query_items
    {
        A::ReverseQueryHash
    } else if workload.integer_span.is_some_and(|span| {
        span <= 65_536
            && span
                <= (workload
                    .indexed_items
                    .saturating_add(workload.query_items)
                    .saturating_mul(4) as u128)
    }) {
        A::DirectAddress
    } else {
        A::IndexedHash
    };

    // Semantic proof and target availability trump the cost heuristic.
    if status(choice) != Some(R::RequiresExactScalarGuard) {
        return fallback;
    }
    SearchPhysicalChoice {
        algorithm: choice,
        basis: SearchSelectionBasis::RuntimeExactScalarGuard,
        estimated_table_entries: match choice {
            A::DirectAddress => workload.integer_span.unwrap_or(0) as usize,
            A::ReverseQueryHash => workload.query_items,
            A::IndexedHash | A::PreparedHash => workload.indexed_items,
            _ => 0,
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferId {
    registry: u64,
    slot: usize,
    generation: u64,
}

/// Fixed-width encodings supported by the first CPU affine representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    BoolByte,
    Int64,
    Float64,
    Char8,
}
impl Encoding {
    pub fn dtype(self) -> DType {
        match self {
            Self::BoolByte => DType::Bool,
            Self::Int64 => DType::Int,
            Self::Float64 => DType::Float,
            Self::Char8 => DType::Char,
        }
    }
    pub fn element_bytes(self) -> usize {
        match self {
            Self::BoolByte | Self::Char8 => 1,
            Self::Int64 | Self::Float64 => 8,
        }
    }
}

#[derive(Debug)]
enum Backing {
    Bool(CpuStorage<u8>),
    Int(CpuStorage<i64>),
    Float(CpuStorage<f64>),
    Char(CpuStorage<u8>),
}
impl Backing {
    fn view(&self) -> CpuView<'_> {
        match self {
            Self::Bool(v) => CpuView::Bool(v),
            Self::Int(v) => CpuView::Int(v),
            Self::Float(v) => CpuView::Float(v),
            Self::Char(v) => CpuView::Char(v),
        }
    }
    fn encoding(&self) -> Encoding {
        match self {
            Self::Bool(_) => Encoding::BoolByte,
            Self::Int(_) => Encoding::Int64,
            Self::Float(_) => Encoding::Float64,
            Self::Char(_) => Encoding::Char8,
        }
    }
    fn len(&self) -> usize {
        match self {
            Self::Bool(v) | Self::Char(v) => v.len(),
            Self::Int(v) => v.len(),
            Self::Float(v) => v.len(),
        }
    }
    fn retained_bytes(&self) -> usize {
        fn bytes<T>(v: &CpuStorage<T>) -> usize {
            let n = match v {
                CpuStorage::Inline(_) => 1,
                CpuStorage::Owned(v) => v.capacity(),
                CpuStorage::Shared(v) => v.capacity(),
            };
            n * std::mem::size_of::<T>()
        }
        match self {
            Self::Bool(v) | Self::Char(v) => bytes(v),
            Self::Int(v) => bytes(v),
            Self::Float(v) => bytes(v),
        }
    }
    fn address(&self) -> usize {
        match self {
            Self::Bool(v) | Self::Char(v) => v.as_ptr() as usize,
            Self::Int(v) => v.as_ptr() as usize,
            Self::Float(v) => v.as_ptr() as usize,
        }
    }
    fn shared_with(&self, rhs: &Self) -> bool {
        fn shared<T>(a: &CpuStorage<T>, b: &CpuStorage<T>) -> bool {
            matches!((a,b), (CpuStorage::Shared(a),CpuStorage::Shared(b)) if Arc::ptr_eq(a,b))
        }
        match (self, rhs) {
            (Self::Bool(a) | Self::Char(a), Self::Bool(b) | Self::Char(b)) => shared(a, b),
            (Self::Int(a), Self::Int(b)) => shared(a, b),
            (Self::Float(a), Self::Float(b)) => shared(a, b),
            _ => false,
        }
    }
}

/// Owning read lease. Removing an ID or dropping its registry does not destroy
/// outstanding leases. No mutable slice or ownership recovery is exposed.
#[derive(Clone, Debug)]
pub struct BufferLease {
    id: BufferId,
    backing: Arc<Backing>,
}
impl BufferLease {
    pub fn id(&self) -> BufferId {
        self.id
    }
    pub fn encoding(&self) -> Encoding {
        self.backing.encoding()
    }
    pub fn len(&self) -> usize {
        self.backing.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Retained payload capacity, excluding Arc/registry/descriptor metadata.
    /// Small views keep this entire allocation alive.
    pub fn retained_payload_bytes(&self) -> usize {
        self.backing.retained_bytes()
    }
    /// Entire initialized CPU backing, in physical memory order, not J order.
    pub fn backing_view(&self) -> CpuView<'_> {
        self.backing.view()
    }
    /// Actual address alignment; empty buffers have no element address.
    pub fn address_alignment(&self) -> Option<usize> {
        if self.is_empty() {
            None
        } else {
            Some(1usize << self.backing.address().trailing_zeros())
        }
    }
    /// Conservative allocation alias test. Different IDs may share a payload.
    pub fn shares_backing(&self, rhs: &Self) -> bool {
        !self.is_empty()
            && !rhs.is_empty()
            && (Arc::ptr_eq(&self.backing, &rhs.backing) || self.backing.shared_with(&rhs.backing))
    }
}

#[derive(Debug)]
struct Slot {
    generation: u64,
    lease: Option<BufferLease>,
}
/// IDs are scoped to a registry. Slot reuse increments a checked generation.
#[derive(Debug)]
pub struct BufferRegistry {
    identity: u64,
    slots: Vec<Slot>,
    free: Vec<usize>,
}
impl BufferRegistry {
    pub fn new() -> Result<Self> {
        #[allow(deprecated)] // try_update requires Rust 1.95; preserve the 1.85 MSRV.
        let identity = NEXT_REGISTRY
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| Error::Limit)?;
        Ok(Self {
            identity,
            slots: Vec::new(),
            free: Vec::new(),
        })
    }
    /// Moves existing storage, preserving shared payloads without a deep clone.
    pub fn register(&mut self, value: Value) -> Result<BufferLease> {
        let backing = match value.data {
            Data::Bool(v) => Backing::Bool(v.into_shared()),
            Data::Int(v) => Backing::Int(v.into_shared()),
            Data::Float(v) => Backing::Float(v.into_shared()),
            Data::Char(v) => Backing::Char(v.into_shared()),
            _ => return Err(Error::Unsupported("non-affine CPU encoding".into())),
        };
        let slot = if let Some(slot) = self.free.pop() {
            slot
        } else {
            self.slots.try_reserve(1).map_err(|_| Error::Limit)?;
            self.slots.push(Slot {
                generation: 0,
                lease: None,
            });
            self.slots.len() - 1
        };
        let lease = BufferLease {
            id: BufferId {
                registry: self.identity,
                slot,
                generation: self.slots[slot].generation,
            },
            backing: Arc::new(backing),
        };
        self.slots[slot].lease = Some(lease.clone());
        Ok(lease)
    }
    pub fn resolve(&self, id: BufferId) -> Result<BufferLease> {
        if id.registry != self.identity {
            return Err(Error::Index);
        }
        let slot = self.slots.get(id.slot).ok_or(Error::Index)?;
        if slot.generation != id.generation {
            return Err(Error::Index);
        }
        slot.lease.clone().ok_or(Error::Index)
    }
    pub fn remove(&mut self, id: BufferId) -> Result<()> {
        self.resolve(id)?;
        let generation = self.slots[id.slot]
            .generation
            .checked_add(1)
            .ok_or(Error::Limit)?;
        self.free.try_reserve(1).map_err(|_| Error::Limit)?;
        self.slots[id.slot].lease = None;
        self.slots[id.slot].generation = generation;
        self.free.push(id.slot);
        Ok(())
    }
}

/// Immutable affine mapping into a leased CPU allocation.
///
/// The descriptor's `shape` is the logical index domain needed to interpret
/// this particular physical mapping; it is not the authoritative semantic
/// identity of a J noun.  Strides, offset, encoding, and BufferId are physical
/// realization details and must remain downstream of logical/semantic analysis.
///
/// Logical dtype comes from its encoding, preventing caller-supplied
/// dtype/backing mismatches.
#[derive(Clone, Debug)]
pub struct PhysicalArray {
    buffer: BufferLease,
    shape: Shape,
    strides: Box<[isize]>,
    offset: isize,
    len: usize,
}
impl PhysicalArray {
    pub fn new(
        buffer: BufferLease,
        shape: impl Into<Shape>,
        strides: Vec<isize>,
        offset: isize,
    ) -> Result<Self> {
        let shape = shape.into();
        if shape.len() != strides.len() {
            return Err(Error::Rank);
        }
        let len = count(&shape)?;
        let mut strides = strides;
        let offset = if len == 0 {
            strides.fill(0);
            0
        } else {
            isize::try_from(len).map_err(|_| Error::Limit)?;
            let mut lo = offset;
            let mut hi = offset;
            for (&dim, stride) in shape.iter().zip(&mut strides) {
                if dim == 1 {
                    *stride = 0;
                    continue;
                }
                let delta = isize::try_from(dim - 1)
                    .map_err(|_| Error::Limit)?
                    .checked_mul(*stride)
                    .ok_or(Error::Limit)?;
                lo = lo.checked_add(delta.min(0)).ok_or(Error::Limit)?;
                hi = hi.checked_add(delta.max(0)).ok_or(Error::Limit)?;
            }
            let lo = usize::try_from(lo).map_err(|_| Error::Index)?;
            let hi = usize::try_from(hi).map_err(|_| Error::Index)?;
            if hi >= buffer.len() {
                return Err(Error::Index);
            }
            let bytes = buffer.encoding().element_bytes();
            let byte_end = hi
                .checked_add(1)
                .and_then(|n| n.checked_mul(bytes))
                .ok_or(Error::Limit)?;
            isize::try_from(byte_end).map_err(|_| Error::Limit)?;
            isize::try_from((hi - lo).checked_mul(bytes).ok_or(Error::Limit)?)
                .map_err(|_| Error::Limit)?;
            offset
        };
        Ok(Self {
            buffer,
            shape,
            strides: strides.into_boxed_slice(),
            offset,
            len,
        })
    }
    /// Adapt an existing dense Value without copying its payload.
    pub fn from_value(registry: &mut BufferRegistry, value: Value) -> Result<Self> {
        let shape = value.shape.clone();
        let n = count(&shape)?;
        let mut strides = Vec::new();
        strides
            .try_reserve_exact(shape.len())
            .map_err(|_| Error::Limit)?;
        strides.resize(shape.len(), 0);
        if n != 0 {
            let mut stride = 1isize;
            for axis in (0..shape.len()).rev() {
                strides[axis] = stride;
                stride = stride
                    .checked_mul(isize::try_from(shape[axis]).map_err(|_| Error::Limit)?)
                    .ok_or(Error::Limit)?;
            }
        }
        let buffer = registry.register(value)?;
        Self::new(buffer, shape, strides, 0)
    }
    pub fn buffer(&self) -> &BufferLease {
        &self.buffer
    }
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }
    pub fn strides(&self) -> &[isize] {
        &self.strides
    }
    pub fn offset(&self) -> isize {
        self.offset
    }
    pub fn encoding(&self) -> Encoding {
        self.buffer.encoding()
    }
    pub fn dtype(&self) -> DType {
        self.encoding().dtype()
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn backing_index(&self, index: &[usize]) -> Result<usize> {
        if index.len() != self.shape.len() {
            return Err(Error::Rank);
        }
        let mut offset = self.offset;
        for ((&i, &dim), &stride) in index.iter().zip(self.shape.iter()).zip(self.strides.iter()) {
            if i >= dim {
                return Err(Error::Index);
            }
            offset = offset
                .checked_add(
                    isize::try_from(i)
                        .map_err(|_| Error::Limit)?
                        .checked_mul(stride)
                        .ok_or(Error::Limit)?,
                )
                .ok_or(Error::Limit)?;
        }
        usize::try_from(offset).map_err(|_| Error::Index)
    }
    pub fn atom(&self, index: &[usize]) -> Result<Scalar> {
        let i = self.backing_index(index)?;
        match self.buffer.backing_view() {
            CpuView::Bool(v) => Ok(Scalar::Bool(v[i] != 0)),
            CpuView::Int(v) => Ok(Scalar::Int(v[i])),
            CpuView::Float(v) => Ok(Scalar::Float(v[i])),
            CpuView::Char(v) => Ok(Scalar::Char(v[i])),
            _ => unreachable!("closed affine encoding"),
        }
    }
    pub fn is_standard_layout(&self) -> bool {
        if self.is_empty() {
            return true;
        }
        let mut expected = 1isize;
        for (&dim, &stride) in self.shape.iter().zip(self.strides.iter()).rev() {
            if dim > 1 && stride != expected {
                return false;
            }
            let Ok(dim) = isize::try_from(dim) else {
                return false;
            };
            let Some(next) = expected.checked_mul(dim) else {
                return false;
            };
            expected = next;
        }
        true
    }
    /// Alignment at the view's first logical element, not the allocation base.
    pub fn address_alignment(&self) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        let bytes = usize::try_from(self.offset)
            .ok()?
            .checked_mul(self.encoding().element_bytes())?;
        let address = self.buffer.backing.address().checked_add(bytes)?;
        Some(1usize << address.trailing_zeros())
    }
    /// Slice in J logical order only. Memory-contiguous transposes/reversals
    /// deliberately return None instead of silently changing atom order.
    ///
    /// ```compile_fail
    /// use rustj::{Value, physical::{BufferRegistry, PhysicalArray}};
    /// let slice;
    /// {
    ///     let mut registry = BufferRegistry::new().unwrap();
    ///     let array = PhysicalArray::from_value(&mut registry, Value::scalar(7)).unwrap();
    ///     slice = array.logical_slice();
    /// }
    /// println!("{slice:?}"); // read lease has been dropped
    /// ```
    pub fn logical_slice(&self) -> Option<CpuView<'_>> {
        if !self.is_standard_layout() {
            return None;
        }
        let start = usize::try_from(self.offset).ok()?;
        let end = start.checked_add(self.len)?;
        match self.buffer.backing_view() {
            CpuView::Bool(v) => Some(CpuView::Bool(v.get(start..end)?)),
            CpuView::Int(v) => Some(CpuView::Int(v.get(start..end)?)),
            CpuView::Float(v) => Some(CpuView::Float(v.get(start..end)?)),
            CpuView::Char(v) => Some(CpuView::Char(v.get(start..end)?)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod search_reference_isolation_tests {
    use super::SEARCH_PLANNER_CALLS;
    use crate::Engine;

    #[test]
    fn fw02_semantic_reference_never_calls_physical_search_planner() {
        let mut engine = Engine::new();
        engine.eval("keys=:i.256").unwrap();
        SEARCH_PLANNER_CALLS.with(|count| count.set(0));

        let reference = engine
            .eval_semantic_reference("keys i. 17 255 999")
            .unwrap()
            .unwrap();
        engine
            .eval_semantic_reference("keys i: 17 255 999")
            .unwrap();
        engine
            .eval_semantic_reference("17 255 999 e. keys")
            .unwrap();
        assert_eq!(
            SEARCH_PLANNER_CALLS.with(|count| count.get()),
            0,
            "reference executor unexpectedly entered physical search planner"
        );

        // Positive control: an ordinary optimized path must exercise this
        // counter; otherwise a zero on the reference path proves nothing.
        let optimized = engine.eval("keys i. 17 255 999").unwrap().unwrap();
        assert_eq!(reference.json(), optimized.json());
        assert!(
            SEARCH_PLANNER_CALLS.with(|count| count.get()) > 0,
            "test instrumentation must observe an optimized planner call"
        );
    }
}

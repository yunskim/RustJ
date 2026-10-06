use crate::error::{Error, Result};
use crate::storage::{CpuStorage, Shape};
use std::sync::Arc;

// Intermediates own CPU buffers; name bindings explicitly freeze them for
// sharing. Device storage will not expose this CPU-only slice interface.
#[derive(Clone, Debug)]
pub enum Data {
    Bool(CpuStorage<u8>),
    Int(CpuStorage<i64>),
    Float(CpuStorage<f64>),
    Char(CpuStorage<u8>),
    Boxed(CpuStorage<Arc<Value>>),
    Sparse(Arc<crate::sparse::SparseArray>),
}

#[derive(Clone, Debug)]
/// Transitional runtime carrier for a logical J value.
///
/// `shape` and J-visible Data variants belong to logical semantics, while the
/// current dense payloads still use `CpuStorage` directly.  That CPU backing is
/// a migration artifact, not a license to add strides, offsets, BufferId,
/// device placement, tiling, or other physical-representation identity here.
///
/// The target architecture keeps logical array identity separate from
/// `physical::PhysicalArray`/BufferId and eventually treats CpuStorage as one
/// backend storage realization.
///
/// Cloning an owned value copies its payload. Call `into_shared` before cloning
/// to explicitly share storage without copying (as name bindings do).
pub struct Value {
    pub(crate) shape: Shape,
    pub(crate) data: Data,
}

pub fn buffer<T>(n: usize) -> Result<Vec<T>> {
    let mut v = Vec::new();
    v.try_reserve_exact(n).map_err(|_| Error::Limit)?;
    Ok(v)
}

// Allocate without zeroing or copying an input. Every element is written once.
// Copy excludes destructors: if f panics, dropping the len=0 Vec is still safe.
#[allow(unsafe_code)]
pub(crate) fn generate<T: Copy>(n: usize, mut f: impl FnMut(usize) -> T) -> Result<Vec<T>> {
    let mut out = buffer(n)?;
    for (i, slot) in out.spare_capacity_mut()[..n].iter_mut().enumerate() {
        slot.write(f(i));
    }
    // SAFETY: capacity >= n and all first n elements were initialized above.
    // If f panicked execution never reaches set_len. T has no destructor.
    unsafe {
        out.set_len(n);
    }
    Ok(out)
}

// The producer is internal and must initialize every slot before returning.
// Keep this unsafe contract inside the numeric module's audited call sites.
#[allow(unsafe_code)]
pub(crate) unsafe fn finish_initialized<T>(mut out: Vec<T>, n: usize) -> Vec<T> {
    // SAFETY: caller guarantees n initialized elements and sufficient capacity.
    unsafe {
        out.set_len(n);
    }
    out
}

pub fn count(shape: &[usize]) -> Result<usize> {
    if shape.contains(&0) {
        return Ok(0);
    }
    shape
        .iter()
        .try_fold(1usize, |n, &d| n.checked_mul(d).ok_or(Error::Limit))
}

impl Value {
    /// Freeze at an ownership-sharing boundary; intermediates remain unshared.
    pub fn into_shared(self) -> Self {
        let data = match self.data {
            Data::Bool(v) => Data::Bool(v.into_shared()),
            Data::Int(v) => Data::Int(v.into_shared()),
            Data::Float(v) => Data::Float(v.into_shared()),
            Data::Char(v) => Data::Char(v.into_shared()),
            Data::Boxed(v) => Data::Boxed(v.into_shared()),
            Data::Sparse(v) => Data::Sparse(v),
        };
        Self {
            shape: self.shape,
            data,
        }
    }
    /// Borrow a CPU array without copying data or incrementing a refcount.
    ///
    /// ```compile_fail
    /// use rustj::Value;
    /// let view;
    /// {
    ///     let owner = Value::ints([3], vec![1, 2, 3]).unwrap();
    ///     view = owner.view();
    /// }
    /// println!("{}", view.len()); // owner no longer exists
    /// ```
    pub fn view(&self) -> crate::storage::ArrayView<'_> {
        use crate::storage::{ArrayView, CpuView};
        let data = match &self.data {
            Data::Bool(v) => CpuView::Bool(v),
            Data::Int(v) => CpuView::Int(v),
            Data::Float(v) => CpuView::Float(v),
            Data::Char(v) => CpuView::Char(v),
            Data::Boxed(v) => CpuView::Boxed(v),
            Data::Sparse(v) => CpuView::Sparse(v),
        };
        ArrayView {
            shape: &self.shape,
            data,
        }
    }
    pub fn new(shape: impl Into<Shape>, data: Data) -> Result<Self> {
        let shape = shape.into();
        let n = match &data {
            Data::Bool(v) | Data::Char(v) => v.len(),
            Data::Int(v) => v.len(),
            Data::Float(v) => v.len(),
            Data::Boxed(v) => v.len(),
            Data::Sparse(v) => {
                if v.shape() != &*shape {
                    return Err(Error::Length);
                }
                count(v.shape())?
            }
        };
        if count(&shape)? != n {
            return Err(Error::Length);
        }
        if let Data::Bool(v) = &data {
            if v.iter().any(|&x| x > 1) {
                return Err(Error::Domain);
            }
        }
        Ok(Self { shape, data })
    }
    pub fn from_sparse(array: crate::sparse::SparseArray) -> Result<Self> {
        let shape = Shape::from(array.shape());
        Self::new(shape, Data::Sparse(Arc::new(array)))
    }
    pub fn is_sparse(&self) -> bool {
        matches!(self.data, Data::Sparse(_))
    }
    pub fn ints(shape: impl Into<Shape>, data: Vec<i64>) -> Result<Self> {
        Self::new(shape, Data::Int(CpuStorage::new(data)))
    }
    /// Box an entire noun, freezing buffers so subsequent opening can share them.
    pub fn boxed(value: Value) -> Self {
        Self {
            shape: Shape::from([]),
            data: Data::Boxed(CpuStorage::Inline(Arc::new(value.into_shared()))),
        }
    }
    pub fn scalar(n: i64) -> Self {
        Self {
            shape: Shape::from([]),
            data: Data::Int(CpuStorage::Inline(n)),
        }
    }
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }
    pub fn data(&self) -> &Data {
        &self.data
    }
    pub fn len(&self) -> usize {
        match &self.data {
            Data::Bool(v) | Data::Char(v) => v.len(),
            Data::Int(v) => v.len(),
            Data::Float(v) => v.len(),
            Data::Boxed(v) => v.len(),
            Data::Sparse(v) => count(v.shape()).expect("validated sparse shape"),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn type_code(&self) -> i32 {
        match self.data {
            Data::Bool(_) => 1,
            Data::Char(_) => 2,
            Data::Int(_) => 4,
            Data::Float(_) => 8,
            Data::Boxed(_) => 32,
            Data::Sparse(ref v) => v.fill().type_code() << 10,
        }
    }
    pub fn int_at(&self, i: usize) -> Result<i64> {
        match &self.data {
            Data::Bool(v) => Ok(v[i] as i64),
            Data::Int(v) => Ok(v[i]),
            Data::Float(v)
                if v[i].is_finite()
                    && v[i].fract() == 0.0
                    && v[i] >= i64::MIN as f64
                    && v[i] < -(i64::MIN as f64) =>
            {
                Ok(v[i] as i64)
            }
            _ => Err(Error::Domain),
        }
    }
    pub fn float_at(&self, i: usize) -> Result<f64> {
        match &self.data {
            Data::Bool(v) => Ok(v[i] as f64),
            Data::Int(v) => Ok(v[i] as f64),
            Data::Float(v) => Ok(v[i]),
            _ => Err(Error::Domain),
        }
    }
    /// Build one representative rank cell when a result frame has zero items.
    ///
    /// jsource cr.c's generic rank path evaluates a fill-cell to determine
    /// the type and shape of the empty result. An existing nonempty argument
    /// contributes its first real cell; an empty dense argument contributes
    /// a cell of type-correct fills. This intentionally does not interpret
    /// boxed or sparse J prototypes.
    pub(crate) fn rank_fill_cell(&self, rank: usize) -> Result<Self> {
        if rank > self.shape.len() {
            return Err(Error::Rank);
        }
        if self.is_sparse() {
            return Err(Error::Unsupported("sparse rank fill cell".into()));
        }
        if !self.is_empty() {
            return self.view().cell(rank, 0)?.to_owned();
        }
        let shape = Shape::from(&self.shape[self.shape.len() - rank..]);
        let atoms = count(&shape)?;
        let data = match &self.data {
            Data::Bool(_) => Data::Bool(CpuStorage::generate(atoms, |_| 0)?),
            Data::Int(_) => Data::Int(CpuStorage::generate(atoms, |_| 0)?),
            Data::Float(_) => Data::Float(CpuStorage::generate(atoms, |_| 0.0)?),
            Data::Char(_) => Data::Char(CpuStorage::generate(atoms, |_| b' ')?),
            Data::Boxed(_) => {
                return Err(Error::Unsupported("boxed rank fill cell".into()));
            }
            Data::Sparse(_) => unreachable!(),
        };
        Self::new(shape, data)
    }

    /// Rebuild a rank fill-cell at a J dense type chosen by cr.c's
    /// EVINHOMO retry. Deliberately discard input values: jtfiller builds
    /// default fillers of the target type, it does not cast the old data.
    /// Boxed/sparse retries remain unsupported until their J contract is
    /// separately witnessed.
    pub(crate) fn rank_refill_as(&self, target_type: i32) -> Result<Self> {
        if self.is_sparse() || matches!(self.data, Data::Boxed(_)) {
            return Err(Error::Unsupported("boxed/sparse rank refill".into()));
        }
        let atoms = count(&self.shape)?;
        let data = match target_type {
            1 => Data::Bool(CpuStorage::generate(atoms, |_| 0)?),
            2 => Data::Char(CpuStorage::generate(atoms, |_| b' ')?),
            4 => Data::Int(CpuStorage::generate(atoms, |_| 0)?),
            8 => Data::Float(CpuStorage::generate(atoms, |_| 0.0)?),
            _ => return Err(Error::Unsupported("rank refill target type".into())),
        };
        Self::new(self.shape.clone(), data)
    }

    /// Assemble a zero-frame result from the type and shape of its fill-cell.
    /// No atom is copied from that synthetic cell into the final result.
    pub(crate) fn empty_rank_result(&self, frame: &[usize]) -> Result<Self> {
        let mut shape = Shape::from(frame);
        shape.extend_from_slice(&self.shape);
        self.select(shape, std::iter::empty())
    }

    pub fn select(
        &self,
        shape: impl Into<Shape>,
        indices: impl IntoIterator<Item = usize>,
    ) -> Result<Self> {
        let shape = shape.into();
        let n = count(&shape)?;
        macro_rules! select {
            ($v:expr, $variant:ident) => {{
                let mut out = buffer(n)?;
                for i in indices {
                    out.push($v.get(i).ok_or(Error::Index)?.clone());
                }
                Data::$variant(CpuStorage::new(out))
            }};
        }
        let data = match &self.data {
            Data::Bool(v) => select!(v, Bool),
            Data::Int(v) => select!(v, Int),
            Data::Float(v) => select!(v, Float),
            Data::Char(v) => select!(v, Char),
            Data::Boxed(v) => select!(v, Boxed),
            Data::Sparse(_) => return Err(Error::Unsupported("sparse selection".into())),
        };
        Self::new(shape, data)
    }
    pub fn json(&self) -> String {
        if let Data::Sparse(v) = &self.data {
            return format!(
                "{{\"type\":{},\"shape\":{:?},\"sparse\":{{\"axes\":{:?},\"coordinates\":{:?},\"fill\":{},\"values\":{}}}}}",
                self.type_code(),
                self.shape(),
                v.sparse_axes(),
                v.coordinates(),
                v.fill().json(),
                v.values().json()
            );
        }
        let shape = self
            .shape
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let values: Vec<String> = match &self.data {
            Data::Bool(v) | Data::Char(v) => v.iter().map(u8::to_string).collect(),
            Data::Int(v) => v.iter().map(i64::to_string).collect(),
            Data::Boxed(v) => v.iter().map(|x| x.json()).collect(),
            Data::Sparse(_) => unreachable!(),
            Data::Float(v) => v
                .iter()
                .map(|x| {
                    if x.is_nan() {
                        "\"nan\"".into()
                    } else if *x == f64::INFINITY {
                        "\"inf\"".into()
                    } else if *x == f64::NEG_INFINITY {
                        "\"-inf\"".into()
                    } else {
                        x.to_string()
                    }
                })
                .collect(),
        };
        format!(
            "{{\"type\":{},\"shape\":[{}],\"data\":[{}]}}",
            self.type_code(),
            shape,
            values.join(",")
        )
    }
    pub fn display(&self) -> String {
        match &self.data {
            Data::Sparse(v) => format!(
                "sparse(shape={:?}, axes={:?}, stored_rows={})",
                self.shape(),
                v.sparse_axes(),
                v.stored_rows()
            ),
            Data::Char(v) => String::from_utf8_lossy(v).into_owned(),
            Data::Boxed(v) => v
                .iter()
                .map(|x| format!("<({})", x.display()))
                .collect::<Vec<_>>()
                .join(" "),
            _ => {
                let parts: Vec<String> = match &self.data {
                    Data::Bool(v) => v.iter().map(u8::to_string).collect(),
                    Data::Int(v) => v.iter().map(|x| x.to_string().replace('-', "_")).collect(),
                    Data::Float(v) => v
                        .iter()
                        .map(|x| {
                            if x.is_nan() {
                                "_.".into()
                            } else if *x == f64::INFINITY {
                                "_".into()
                            } else if *x == f64::NEG_INFINITY {
                                "__".into()
                            } else {
                                x.to_string().replace('-', "_")
                            }
                        })
                        .collect(),
                    _ => unreachable!(),
                };
                if self.shape.len() < 2 {
                    parts.join(" ")
                } else {
                    let width = *self.shape.last().unwrap();
                    if width == 0 {
                        String::new()
                    } else {
                        parts
                            .chunks(width)
                            .map(|r| r.join(" "))
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod rank_fill_cell_tests {
    use super::{Data, Value};
    use crate::storage::CpuStorage;

    #[test]
    fn dense_empty_rank_cells_have_j_type_fills_and_result_shape() {
        let integer = Value::ints([0, 3], vec![]).unwrap();
        let fill = integer.rank_fill_cell(1).unwrap();
        assert_eq!(fill.shape(), &[3]);
        assert_eq!(fill.type_code(), 4);
        for index in 0..3 {
            assert_eq!(fill.int_at(index).unwrap(), 0);
        }
        let empty_result = fill.empty_rank_result(&[0]).unwrap();
        assert_eq!(empty_result.shape(), &[0, 3]);
        assert_eq!(empty_result.type_code(), 4);
        assert_eq!(empty_result.len(), 0);

        let character = Value::new([0, 2], Data::Char(CpuStorage::new(vec![]))).unwrap();
        let char_fill = character.rank_fill_cell(1).unwrap();
        assert_eq!(char_fill.type_code(), 2);
        assert_eq!(char_fill.shape(), &[2]);
        assert_eq!(char_fill.display(), "  ");

        let floating = Value::new([0, 2], Data::Float(CpuStorage::new(vec![]))).unwrap();
        let float_fill = floating.rank_fill_cell(1).unwrap();
        assert_eq!(float_fill.type_code(), 8);
        assert_eq!(float_fill.float_at(0).unwrap(), 0.0);
        assert_eq!(float_fill.float_at(1).unwrap(), 0.0);
    }

    #[test]
    fn nonempty_rank_argument_reuses_actual_first_cell_not_fill() {
        let value = Value::ints([2, 3], vec![5, 6, 7, 8, 9, 10]).unwrap();
        let fill = value.rank_fill_cell(1).unwrap();
        assert_eq!(fill.shape(), &[3]);
        assert_eq!(fill.int_at(0).unwrap(), 5);
        assert_eq!(fill.int_at(1).unwrap(), 6);
        assert_eq!(fill.int_at(2).unwrap(), 7);
    }

    #[test]
    fn rank_refill_uses_target_type_default_not_original_values() {
        let source = Value::ints([3], vec![11, 22, 33]).unwrap();
        let ch = source.rank_refill_as(2).unwrap();
        assert_eq!(ch.type_code(), 2);
        assert_eq!(ch.shape(), &[3]);
        assert_eq!(ch.display(), "   ");
        let int = ch.rank_refill_as(4).unwrap();
        assert_eq!(int.type_code(), 4);
        assert_eq!(int.shape(), &[3]);
        for i in 0..3 {
            assert_eq!(int.int_at(i).unwrap(), 0);
        }
        let fl = ch.rank_refill_as(8).unwrap();
        assert_eq!(fl.type_code(), 8);
        assert_eq!(fl.float_at(2).unwrap(), 0.0);
        assert_eq!(
            ch.rank_refill_as(32).unwrap_err().kind(),
            "unsupported",
        );
        let empty = Value::ints([0, 3], vec![]).unwrap();
        assert_eq!(empty.rank_refill_as(2).unwrap().shape(), &[0, 3]);
    }

    #[test]
    fn unknown_boxed_fill_is_not_guessed() {
        let boxed = Value::new([0], Data::Boxed(CpuStorage::new(vec![]))).unwrap();
        assert_eq!(boxed.rank_fill_cell(0).unwrap_err().kind(), "unsupported");
    }
}

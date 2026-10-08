//! CPU storage. Device allocations must use a separate API, never Deref to a
//! host slice. CUDA storage will be introduced only with a real device backend.
use crate::error::{Error, Result};
use std::{ops::Deref, sync::Arc};

/// Owned clones copy; explicitly freeze with into_shared before sharing values.
/// The evaluator freezes name bindings, but keeps intermediate buffers owned.
#[derive(Clone, Debug)]
pub enum CpuStorage<T> {
    Inline(T),
    Owned(Vec<T>),
    Shared(Arc<Vec<T>>),
}
impl<T> CpuStorage<T> {
    pub fn as_slice(&self) -> &[T] {
        self
    }
    pub fn new(mut values: Vec<T>) -> Self {
        if values.len() == 1 {
            Self::Inline(values.pop().unwrap())
        } else {
            Self::Owned(values)
        }
    }
    pub fn into_shared(self) -> Self {
        match self {
            Self::Owned(v) if !v.is_empty() => Self::Shared(Arc::new(v)),
            other => other,
        }
    }
    pub fn try_into_vec(self) -> std::result::Result<Vec<T>, Self> {
        match self {
            Self::Owned(v) => Ok(v),
            Self::Shared(v) => Arc::try_unwrap(v).map_err(Self::Shared),
            other => Err(other),
        }
    }
}
impl<T: Copy> CpuStorage<T> {
    pub(crate) fn generate(n: usize, f: impl FnMut(usize) -> T) -> Result<Self> {
        let mut f = f;
        if n == 1 {
            Ok(Self::Inline(f(0)))
        } else {
            crate::value::generate(n, f).map(Self::new)
        }
    }
}
impl<T> Deref for CpuStorage<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        match self {
            Self::Inline(x) => std::slice::from_ref(x),
            Self::Owned(v) => v,
            Self::Shared(v) => v,
        }
    }
}

/// Up to four axes without heap allocation. Arbitrary higher ranks remain valid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shape(ShapeRepr);
#[derive(Clone, Debug, PartialEq, Eq)]
enum ShapeRepr {
    Inline { axes: [usize; 4], rank: u8 },
    Heap(Box<[usize]>),
}
impl From<&[usize]> for Shape {
    fn from(value: &[usize]) -> Self {
        if value.len() <= 4 {
            let mut axes = [0; 4];
            axes[..value.len()].copy_from_slice(value);
            Self(ShapeRepr::Inline {
                axes,
                rank: value.len() as u8,
            })
        } else {
            Self(ShapeRepr::Heap(value.into()))
        }
    }
}
impl From<Vec<usize>> for Shape {
    fn from(value: Vec<usize>) -> Self {
        if value.len() <= 4 {
            Self::from(value.as_slice())
        } else {
            Self(ShapeRepr::Heap(value.into_boxed_slice()))
        }
    }
}
impl<const N: usize> From<[usize; N]> for Shape {
    fn from(value: [usize; N]) -> Self {
        Self::from(value.as_slice())
    }
}
impl Deref for Shape {
    type Target = [usize];
    fn deref(&self) -> &[usize] {
        match &self.0 {
            ShapeRepr::Inline { axes, rank } => &axes[..*rank as usize],
            ShapeRepr::Heap(v) => v,
        }
    }
}
impl Shape {
    pub fn extend_from_slice(&mut self, suffix: &[usize]) {
        if self.len() + suffix.len() <= 4 {
            let mut axes = [0; 4];
            let n = self.len();
            axes[..n].copy_from_slice(self);
            axes[n..n + suffix.len()].copy_from_slice(suffix);
            *self = Self(ShapeRepr::Inline {
                axes,
                rank: (n + suffix.len()) as u8,
            });
        } else {
            let mut axes = self.to_vec();
            axes.extend_from_slice(suffix);
            *self = Self::from(axes);
        }
    }
}

/// A typed read-only CPU view cannot outlive its source. No owner or refcount.
#[derive(Clone, Copy, Debug)]
pub enum CpuView<'a> {
    Bool(&'a [u8]),
    Int(&'a [i64]),
    Float(&'a [f64]),
    Char(&'a [u8]),
    ExtendedInt(&'a [Arc<crate::types::BigInt>]),
    Boxed(&'a [Arc<crate::Value>]),
    Sparse(&'a crate::sparse::SparseArray),
}
#[derive(Clone, Copy, Debug)]
pub struct ArrayView<'a> {
    pub(crate) shape: &'a [usize],
    pub(crate) data: CpuView<'a>,
}
impl<'a> ArrayView<'a> {
    pub fn to_owned(self) -> Result<crate::value::Value> {
        use crate::value::{Data, Value};
        let data = match self.data {
            CpuView::Sparse(v) => return crate::Value::from_sparse(v.clone()),
            CpuView::Bool(v) => Data::Bool(CpuStorage::generate(v.len(), |i| v[i])?),
            CpuView::Int(v) => Data::Int(CpuStorage::generate(v.len(), |i| v[i])?),
            CpuView::Float(v) => Data::Float(CpuStorage::generate(v.len(), |i| v[i])?),
            CpuView::Char(v) => Data::Char(CpuStorage::generate(v.len(), |i| v[i])?),
            CpuView::ExtendedInt(v) => {
                let mut out = crate::value::buffer(v.len())?;
                out.extend_from_slice(v);
                Data::ExtendedInt(CpuStorage::new(out))
            }
            CpuView::Boxed(v) => {
                let mut out = crate::value::buffer(v.len())?;
                out.extend_from_slice(v);
                Data::Boxed(CpuStorage::new(out))
            }
        };
        Value::new(Shape::from(self.shape), data)
    }
    pub fn shape(self) -> &'a [usize] {
        self.shape
    }
    pub fn len(self) -> usize {
        match self.data {
            CpuView::Bool(v) | CpuView::Char(v) => v.len(),
            CpuView::Int(v) => v.len(),
            CpuView::Float(v) => v.len(),
            CpuView::ExtendedInt(v) => v.len(),
            CpuView::Boxed(v) => v.len(),
            CpuView::Sparse(v) => crate::value::count(v.shape()).expect("validated sparse view"),
        }
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
    pub fn data(self) -> CpuView<'a> {
        self.data
    }
    pub fn int_at(self, i: usize) -> Result<i64> {
        match self.data {
            CpuView::Bool(v) => Ok(*v.get(i).ok_or(Error::Index)? as i64),
            CpuView::Int(v) => v.get(i).copied().ok_or(Error::Index),
            CpuView::ExtendedInt(v) => i64::try_from(v.get(i).ok_or(Error::Index)?.as_ref())
                .map_err(|_| Error::Unsupported("extended integer machine conversion".into())),
            CpuView::Float(v) => {
                let x = *v.get(i).ok_or(Error::Index)?;
                if x.is_finite()
                    && x.fract() == 0.0
                    && x >= i64::MIN as f64
                    && x < -(i64::MIN as f64)
                {
                    Ok(x as i64)
                } else {
                    Err(Error::Domain)
                }
            }
            _ => Err(Error::Domain),
        }
    }
    pub fn float_at(self, i: usize) -> Result<f64> {
        match self.data {
            CpuView::Float(v) => v.get(i).copied().ok_or(Error::Index),
            _ => self.int_at(i).map(|x| x as f64),
        }
    }
    pub fn cell(self, rank: usize, index: usize) -> Result<Self> {
        if matches!(self.data, CpuView::Sparse(_)) {
            return Err(Error::Unsupported("sparse rank cells".into()));
        }
        if rank > self.shape.len() {
            return Err(Error::Rank);
        }
        let split = self.shape.len() - rank;
        let shape = &self.shape[split..];
        let frames = crate::value::count(&self.shape[..split])?;
        if index >= frames {
            return Err(Error::Index);
        }
        let size = crate::value::count(shape)?;
        let start = index.checked_mul(size).ok_or(Error::Limit)?;
        let end = start.checked_add(size).ok_or(Error::Limit)?;
        let data = match self.data {
            CpuView::Bool(v) => CpuView::Bool(v.get(start..end).ok_or(Error::Index)?),
            CpuView::Int(v) => CpuView::Int(v.get(start..end).ok_or(Error::Index)?),
            CpuView::Float(v) => CpuView::Float(v.get(start..end).ok_or(Error::Index)?),
            CpuView::Char(v) => CpuView::Char(v.get(start..end).ok_or(Error::Index)?),
            CpuView::Sparse(_) => unreachable!(),
            CpuView::ExtendedInt(v) => CpuView::ExtendedInt(v.get(start..end).ok_or(Error::Index)?),
            CpuView::Boxed(v) => CpuView::Boxed(v.get(start..end).ok_or(Error::Index)?),
        };
        Ok(Self { shape, data })
    }
}

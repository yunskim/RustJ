//! Axis-sparse storage foundation. Not yet connected to J's `$.` verbs.
//! Coordinates are sorted unique rows; omitted cells have the scalar fill value.
use crate::{
    Data, Error, Result, Value,
    storage::{CpuStorage, Shape},
    value::{buffer, count},
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct SparseArray {
    shape: Shape,
    axes: Arc<[usize]>,
    coordinates: Arc<[usize]>,
    fill: Value,
    values: Value,
}
impl SparseArray {
    /// `values.shape = [stored_rows] + dimensions of the non-sparse axes`.
    /// An empty sparse-axis list permits at most one coordinate row.
    pub fn new(
        shape: impl Into<Shape>,
        axes: Vec<usize>,
        coordinates: Vec<usize>,
        fill: Value,
        values: Value,
    ) -> Result<Self> {
        let shape = shape.into();
        if !fill.shape().is_empty() {
            return Err(Error::Rank);
        }
        if !matches!(
            fill.data(),
            Data::Bool(_) | Data::Int(_) | Data::Float(_) | Data::Char(_)
        ) {
            return Err(Error::Unsupported("sparse element type".into()));
        }
        if fill.type_code() != values.type_code() {
            return Err(Error::Domain);
        }
        if axes.iter().any(|&axis| axis >= shape.len())
            || axes.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(Error::Domain);
        }
        let rows = *values.shape().first().ok_or(Error::Rank)?;
        if coordinates.len() != rows.checked_mul(axes.len()).ok_or(Error::Limit)? {
            return Err(Error::Length);
        }
        let dense_shape: Vec<_> = shape
            .iter()
            .enumerate()
            .filter(|(axis, _)| axes.binary_search(axis).is_err())
            .map(|(_, d)| *d)
            .collect();
        if values.shape()[1..] != dense_shape {
            return Err(Error::Length);
        }
        if axes.is_empty() {
            if rows > 1 {
                return Err(Error::Domain);
            }
        } else {
            for row in coordinates.chunks_exact(axes.len()) {
                if row.iter().zip(&axes).any(|(&i, &axis)| i >= shape[axis]) {
                    return Err(Error::Index);
                }
            }
            for row in 1..rows {
                let a = &coordinates[(row - 1) * axes.len()..row * axes.len()];
                let b = &coordinates[row * axes.len()..(row + 1) * axes.len()];
                if a >= b {
                    return Err(Error::Domain);
                }
            }
        }
        Ok(Self {
            shape,
            axes: axes.into(),
            coordinates: coordinates.into(),
            fill: fill.into_shared(),
            values: values.into_shared(),
        })
    }
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }
    pub fn sparse_axes(&self) -> &[usize] {
        &self.axes
    }
    pub fn coordinates(&self) -> &[usize] {
        &self.coordinates
    }
    pub fn fill(&self) -> &Value {
        &self.fill
    }
    pub fn values(&self) -> &Value {
        &self.values
    }
    pub fn stored_rows(&self) -> usize {
        self.values.shape()[0]
    }

    /// Explicit bounded materialization. No implicit allocation of logical size.
    pub fn to_dense(&self, max_atoms: usize) -> Result<Value> {
        let n = if self.shape.contains(&0) {
            0
        } else {
            count(&self.shape)?
        };
        if n > max_atoms {
            return Err(Error::Limit);
        }
        macro_rules! expand {
            ($f:expr,$v:expr,$kind:ident) => {
                Data::$kind(CpuStorage::new(self.expand($f[0], $v, n)?))
            };
        }
        let data = match (self.fill.data(), self.values.data()) {
            (Data::Bool(f), Data::Bool(v)) => expand!(f, v, Bool),
            (Data::Int(f), Data::Int(v)) => expand!(f, v, Int),
            (Data::Float(f), Data::Float(v)) => expand!(f, v, Float),
            (Data::Char(f), Data::Char(v)) => expand!(f, v, Char),
            _ => return Err(Error::Domain),
        };
        Value::new(self.shape.clone(), data)
    }
    fn expand<T: Copy>(&self, fill: T, values: &[T], n: usize) -> Result<Vec<T>> {
        let mut out = buffer(n)?;
        out.resize(n, fill);
        if n == 0 {
            return Ok(out);
        }
        let mut strides = vec![1; self.shape.len()];
        let mut stride = 1usize;
        for axis in (0..self.shape.len()).rev() {
            strides[axis] = stride;
            stride = stride.checked_mul(self.shape[axis]).ok_or(Error::Limit)?;
        }
        let dense_axes: Vec<_> = (0..self.shape.len())
            .filter(|axis| self.axes.binary_search(axis).is_err())
            .collect();
        let cell = count(&self.values.shape()[1..])?;
        for row in 0..self.stored_rows() {
            let mut base = 0;
            for (column, &axis) in self.axes.iter().enumerate() {
                base += self.coordinates[row * self.axes.len() + column] * strides[axis];
            }
            for k in 0..cell {
                let mut rem = k;
                let mut index = base;
                for &axis in dense_axes.iter().rev() {
                    index += (rem % self.shape[axis]) * strides[axis];
                    rem /= self.shape[axis];
                }
                out[index] = values[row * cell + k];
            }
        }
        Ok(out)
    }
}

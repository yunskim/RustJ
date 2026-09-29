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
    /// Exact storage conversion: floating fill comparison preserves signed zero
    /// and NaN payload bits. This does not apply J's tolerant numeric equality.
    pub fn from_dense(dense: &Value, axes: Vec<usize>, fill: Value) -> Result<Self> {
        if !fill.shape().is_empty() {
            return Err(Error::Rank);
        }
        if dense.type_code() != fill.type_code() {
            return Err(Error::Domain);
        }
        if axes.iter().any(|&axis| axis >= dense.shape().len())
            || axes.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(Error::Domain);
        }
        let dense_axes: Vec<_> = (0..dense.shape().len())
            .filter(|axis| axes.binary_search(axis).is_err())
            .collect();
        let dense_shape: Vec<_> = dense_axes.iter().map(|&axis| dense.shape()[axis]).collect();
        macro_rules! pack {
            ($v:expr, $f:expr, $kind:ident, $same:expr) => {{
                let (coordinates, rows, data) =
                    pack_cells(dense.shape(), &axes, &dense_axes, $v, $f[0], $same)?;
                let mut value_shape = vec![rows];
                value_shape.extend_from_slice(&dense_shape);
                let values = Value::new(value_shape, Data::$kind(CpuStorage::new(data)))?;
                Self::new(Shape::from(dense.shape()), axes, coordinates, fill, values)
            }};
        }
        match (dense.data(), fill.data()) {
            (Data::Bool(v), Data::Bool(f)) => pack!(v, f, Bool, |a, b| a == b),
            (Data::Int(v), Data::Int(f)) => pack!(v, f, Int, |a, b| a == b),
            (Data::Char(v), Data::Char(f)) => pack!(v, f, Char, |a, b| a == b),
            (Data::Float(v), Data::Float(f)) => {
                pack!(v, f, Float, |a: f64, b: f64| a.to_bits() == b.to_bits())
            }
            _ => Err(Error::Unsupported("sparse element type".into())),
        }
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

// Scan one dense cell at a time; allocate only retained rows, never a full
// array of mapped indices or a dense-sized output scratch buffer.
fn pack_cells<T: Copy>(
    shape: &[usize],
    axes: &[usize],
    dense_axes: &[usize],
    input: &[T],
    fill: T,
    same: impl Fn(T, T) -> bool,
) -> Result<(Vec<usize>, usize, Vec<T>)> {
    let mut coordinates = Vec::new();
    let mut output = Vec::new();
    if input.is_empty() {
        return Ok((coordinates, 0, output));
    }
    let mut strides = vec![1; shape.len()];
    let mut stride = 1usize;
    for axis in (0..shape.len()).rev() {
        strides[axis] = stride;
        stride = stride.checked_mul(shape[axis]).ok_or(Error::Limit)?;
    }
    let rows = axes.iter().try_fold(1usize, |n, &axis| {
        n.checked_mul(shape[axis]).ok_or(Error::Limit)
    })?;
    let cell = input.len() / rows;
    let mut row_coordinates = vec![0; axes.len()];
    let mut stored = 0;
    for row in 0..rows {
        let mut rem = row;
        let mut base = 0;
        for (column, &axis) in axes.iter().enumerate().rev() {
            row_coordinates[column] = rem % shape[axis];
            rem /= shape[axis];
            base += row_coordinates[column] * strides[axis];
        }
        let index = |mut k: usize| {
            let mut index = base;
            for &axis in dense_axes.iter().rev() {
                index += (k % shape[axis]) * strides[axis];
                k /= shape[axis];
            }
            index
        };
        if (0..cell).any(|k| !same(input[index(k)], fill)) {
            coordinates
                .try_reserve(axes.len())
                .map_err(|_| Error::Limit)?;
            output.try_reserve(cell).map_err(|_| Error::Limit)?;
            coordinates.extend_from_slice(&row_coordinates);
            for k in 0..cell {
                output.push(input[index(k)]);
            }
            stored += 1;
        }
    }
    Ok((coordinates, stored, output))
}

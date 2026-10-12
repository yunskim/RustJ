//! Coordinate-only access algebra for fully witnessed static axis permutations.
//!
//! This is an analysis projection, not an executable J rewrite or an Exact-J
//! equivalence witness. In particular, it says nothing about error/effect order,
//! NAME resolution, fill/prototype, or whether an apparent permutation is a
//! legal transformation of a specific J operation.

/// A structural proof failure: never silently replace missing evidence by a
/// guessed identity permutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisAccessError {
    RankMismatch,
    NotPermutation,
    IntermediateShapeMismatch,
    CoordinateOutOfBounds,
}

/// A bijective logical output-axis -> input-axis access relation.
///
/// For every output axis j, output_shape[j] = input_shape[axes[j]], and
/// input_coordinate[axes[j]] = output_coordinate[j].
/// The shape belongs to logical J atoms; no layout, stride, or device is implied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticAxisAccess {
    input_shape: Vec<usize>,
    axes: Vec<usize>,
}

impl StaticAxisAccess {
    pub fn new(input_shape: Vec<usize>, axes: Vec<usize>) -> Result<Self, AxisAccessError> {
        let rank = input_shape.len();
        if rank != axes.len() {
            return Err(AxisAccessError::RankMismatch);
        }
        let mut seen = vec![false; rank];
        for &axis in &axes {
            if axis >= rank || seen[axis] {
                return Err(AxisAccessError::NotPermutation);
            }
            seen[axis] = true;
        }
        Ok(Self { input_shape, axes })
    }

    pub fn input_shape(&self) -> &[usize] {
        &self.input_shape
    }

    pub fn output_shape(&self) -> Vec<usize> {
        self.axes
            .iter()
            .map(|&axis| self.input_shape[axis])
            .collect()
    }

    /// Compose self (input -> middle) followed by next (middle -> output).
    ///
    /// Both relations are independently shape-witnessed axis permutations.
    /// This proves only equality of logical coordinate lookups; it does not
    /// license dropping a J-visible intermediate operation or its checks.
    pub fn compose_after(&self, next: &Self) -> Result<Self, AxisAccessError> {
        if self.output_shape() != next.input_shape {
            return Err(AxisAccessError::IntermediateShapeMismatch);
        }
        let axes = next.axes.iter().map(|&axis| self.axes[axis]).collect();
        Self::new(self.input_shape.clone(), axes)
    }

    /// Pull one requested output coordinate back to its logical input coordinate.
    pub fn pullback_point(&self, output: &[usize]) -> Result<Vec<usize>, AxisAccessError> {
        if output.len() != self.axes.len()
            || output
                .iter()
                .zip(self.axes.iter())
                .any(|(&index, &axis)| index >= self.input_shape[axis])
        {
            return Err(AxisAccessError::CoordinateOutOfBounds);
        }
        let mut input = vec![0; self.axes.len()];
        for (output_axis, &input_axis) in self.axes.iter().enumerate() {
            input[input_axis] = output[output_axis];
        }
        Ok(input)
    }
}

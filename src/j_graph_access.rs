//! Coordinate-only access algebra for fully witnessed static axis permutations.
//!
//! This is an analysis projection, not an executable J rewrite or an Exact-J
//! equivalence witness. In particular, it says nothing about error/effect order,
//! NAME resolution, fill/prototype, or whether an apparent permutation is a
//! legal transformation of a specific J operation.

/// A structural proof failure: never silently replace missing evidence by a
/// guessed identity permutation.
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisAccessError {
    RankMismatch,
    NotPermutation,
    IntermediateShapeMismatch,
    CoordinateOutOfBounds,
    RegionOutOfBounds,
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

    /// Exact logical rectangular value-demand pullback under this bijection.
    ///
    /// This does not waive J prototype, error, NAME or effect obligations, and
    /// it is not a physical byte or storage chunk request.
    pub fn pullback_region(
        &self,
        output_region: &[Range<usize>],
    ) -> Result<Vec<Range<usize>>, AxisAccessError> {
        if output_region.len() != self.axes.len() {
            return Err(AxisAccessError::RankMismatch);
        }
        let mut input_region = vec![0..0; self.axes.len()];
        for (output_axis, interval) in output_region.iter().enumerate() {
            let input_axis = self.axes[output_axis];
            let extent = self.input_shape[input_axis];
            if interval.start > interval.end || interval.end > extent {
                return Err(AxisAccessError::RegionOutOfBounds);
            }
            input_region[input_axis] = interval.clone();
        }
        Ok(input_region)
    }
}

use crate::{
    contracts::Valence,
    j_graph_ir::{AccessContract, GraphBasisKind, GraphForm, NodeKind, Plan, ValueId},
    primitive::PrimitiveId,
    semantic::FunctionHead,
};

/// A graph-derived coordinate projection, retaining the source value identity.
///
/// This is *not* an Exact-J equivalence, materialization-elision or executable
/// region-demand witness. NAME reads, checks and effects stay in the original
/// J Graph and execution route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphTransposeAccess {
    pub input: ValueId,
    pub output: ValueId,
    pub access: StaticAxisAccess,
}

impl Plan {
    /// Extract the axis relation of an atomic monadic J `|:` application.
    ///
    /// Require the primitive identity, exact input/output shape and rank,
    /// and the original graph-basis/access contract. Other reindex operations,
    /// dynamic facts, ranked transposes and dyadic transpose remain unknown.
    /// A complete valid J graph is required; the projection never mutates it.
    pub fn static_transpose_access(&self, output: ValueId) -> Option<GraphTransposeAccess> {
        self.verify().ok()?;
        let node = self.nodes.get(output.0)?;
        let NodeKind::Apply {
            function,
            form,
            basis,
            contract,
            valence,
            left,
            right,
            ..
        } = &node.kind
        else {
            return None;
        };
        if !matches!(&function.head, FunctionHead::PrimitiveVerb(PrimitiveId::Cant))
            || !function.operands.is_empty()
            || !matches!(form, GraphForm::Atomic)
            || *valence != Valence::Monad
            || left.is_some()
            || basis.layers.as_slice() != [GraphBasisKind::StaticReindex]
            || contract.access != AccessContract::IndexTransform
        {
            return None;
        }

        let input_node = self.nodes.get(right.0)?;
        let input_shape = input_node.facts.shape.as_ref()?;
        let output_shape = node.facts.shape.as_ref()?;
        if input_node.facts.rank != Some(input_shape.len())
            || node.facts.rank != Some(output_shape.len())
        {
            return None;
        }
        let axes: Vec<usize> = (0..input_shape.len()).rev().collect();
        let access = StaticAxisAccess::new(input_shape.clone(), axes).ok()?;
        if access.output_shape() != *output_shape {
            return None;
        }
        Some(GraphTransposeAccess {
            input: *right,
            output,
            access,
        })
    }
}

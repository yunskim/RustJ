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
        if !matches!(
            &function.head,
            FunctionHead::PrimitiveVerb(PrimitiveId::Cant)
        ) || !function.operands.is_empty()
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

/// The two independently recognized families sharing the same logical
/// output-to-input coordinate projection and box-demand pullback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphValueAccessKind {
    UnaryMap,
    MonadicTranspose,
}

/// A single-edge *value-coordinate* demand projection for an original J Graph
/// occurrence. Neither family permits skipping J-visible errors, prototype/fill
/// evaluation, NAME/effects or any original intermediate call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphValueAccess {
    pub kind: GraphValueAccessKind,
    pub input: ValueId,
    pub output: ValueId,
    pub access: StaticAxisAccess,
}

impl Plan {
    /// Project an exactly witnessed atomic unary Map or monadic transpose
    /// onto the *same* static coordinate and rectangular-demand algebra.
    ///
    /// Eligibility is structural and shape-only. An identity Map access does
    /// not prove that evaluating unrequested elements can be skipped: a J Map
    /// may error there, and empty-frame prototype work is independently visible.
    pub fn static_value_access(&self, output: ValueId) -> Option<GraphValueAccess> {
        if let Some(transpose) = self.static_transpose_access(output) {
            return Some(GraphValueAccess {
                kind: GraphValueAccessKind::MonadicTranspose,
                input: transpose.input,
                output: transpose.output,
                access: transpose.access,
            });
        }

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
        let FunctionHead::PrimitiveVerb(primitive_id) = &function.head else {
            return None;
        };
        let primitive = crate::contracts::for_primitive(*primitive_id, Valence::Monad);
        if !function.operands.is_empty()
            || !matches!(form, GraphForm::Atomic)
            || *valence != Valence::Monad
            || left.is_some()
            || basis.layers.as_slice() != [GraphBasisKind::Elementwise]
            || contract.iteration != crate::j_graph_ir::IterationContract::Elementwise
            || contract.access != AccessContract::CorrespondingElements
            || primitive.class != crate::contracts::OperationClass::Map
            || primitive.shape_rule != crate::contracts::ShapeRule::PreserveRight
            || primitive.effect != crate::contracts::Effect::Pure
        {
            return None;
        }

        let input_node = self.nodes.get(right.0)?;
        let input_shape = input_node.facts.shape.as_ref()?;
        let output_shape = node.facts.shape.as_ref()?;
        if input_node.facts.rank != Some(input_shape.len())
            || node.facts.rank != Some(output_shape.len())
            || input_shape != output_shape
        {
            return None;
        }
        let axes: Vec<usize> = (0..input_shape.len()).collect();
        let access = StaticAxisAccess::new(input_shape.clone(), axes).ok()?;
        Some(GraphValueAccess {
            kind: GraphValueAccessKind::UnaryMap,
            input: *right,
            output,
            access,
        })
    }
}

/// Why a backward logical value-demand trace cannot be extended further.
///
/// An unproved operation remains a barrier even when its result box is known.
/// Neither variant permits skipping execution of any original J graph node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphDemandStop {
    /// An original input, literal or versioned NAME read was reached.
    Source,
    /// The last value is the *output* of an unproved operation; its input
    /// requirements are entirely unknown, not inferred from this box.
    UnprovedOperation,
}

/// A precisely known rectangular *logical value* demand at one original
/// J Graph ValueId. No storage byte, layout or execution implication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphDemandAtValue {
    pub value: ValueId,
    pub region: Vec<Range<usize>>,
}

/// Backward demand across a chain of individually witnessed unary edges.
/// Steps are ordered output first, ending at a source or a blocked output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphValueDemandTrace {
    pub requested_output: ValueId,
    pub steps: Vec<GraphDemandAtValue>,
    pub stop: GraphDemandStop,
}

impl Plan {
    /// Follow witnessed Map/Transpose *value-coordinate* edges backwards.
    ///
    /// This is an observationally inert analysis projection. It deliberately
    /// stops before unsupported operations instead of guessing their input
    /// region. In particular, an empty demanded box cannot erase J error,
    /// fill/prototype, ordered NAME, rank/cell/frame or effect obligations.
    pub fn value_demand_trace(
        &self,
        output: ValueId,
        output_region: &[Range<usize>],
    ) -> Option<GraphValueDemandTrace> {
        self.verify().ok()?;
        let node = self.nodes.get(output.0)?;
        let shape = node.facts.shape.as_ref()?;
        if shape.len() != output_region.len()
            || shape
                .iter()
                .zip(output_region)
                .any(|(extent, range)| range.start > range.end || range.end > *extent)
        {
            return None;
        }

        let mut current = output;
        let mut region = output_region.to_vec();
        let mut steps = vec![GraphDemandAtValue {
            value: current,
            region: region.clone(),
        }];
        loop {
            let Some(edge) = self.static_value_access(current) else {
                let stop = match &self.nodes[current.0].kind {
                    NodeKind::Input { .. } | NodeKind::Literal(_) | NodeKind::ReadNoun { .. } => {
                        GraphDemandStop::Source
                    }
                    _ => GraphDemandStop::UnprovedOperation,
                };
                return Some(GraphValueDemandTrace {
                    requested_output: output,
                    steps,
                    stop,
                });
            };
            // J Graph verification requires producer-before-consumer ordering;
            // reject any inconsistent/cyclic relation rather than looping.
            if edge.output != current || edge.input.0 >= current.0 {
                return None;
            }
            region = edge.access.pullback_region(&region).ok()?;
            current = edge.input;
            steps.push(GraphDemandAtValue {
                value: current,
                region: region.clone(),
            });
        }
    }
}

/// Exact union of separately witnessed rectangular *logical value* demands
/// for one original Graph ValueId. A disjoint union is not silently replaced
/// by a bounding box; duplicate or overlapping boxes are retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphValueDemandUnion {
    pub value: ValueId,
    pub boxes: Vec<Vec<Range<usize>>>,
}

impl GraphValueDemandUnion {
    /// Membership in the exact union, not in a potentially larger hull.
    pub fn contains(&self, coordinate: &[usize]) -> bool {
        self.boxes.iter().any(|region| {
            region.len() == coordinate.len()
                && region
                    .iter()
                    .zip(coordinate)
                    .all(|(range, index)| range.contains(index))
        })
    }
}

impl Plan {
    /// Pull back several original output demands to a common Graph ValueId.
    ///
    /// Each request is validated *within this Plan*, with its own backward
    /// R0.4 witness chain. Every request must reach target; an unsupported
    /// branch/join is a barrier. This does not model executable partial
    /// evaluation, branch pruning or error/effect/prototype elision.
    pub fn witnessed_value_demand_union(
        &self,
        target: ValueId,
        requests: &[(ValueId, Vec<Range<usize>>)],
    ) -> Option<GraphValueDemandUnion> {
        self.verify().ok()?;
        let target_shape = self.nodes.get(target.0)?.facts.shape.as_ref()?;
        if requests.is_empty() {
            return None;
        }
        let mut boxes = Vec::with_capacity(requests.len());
        for (output, region) in requests {
            let trace = self.value_demand_trace(*output, region)?;
            let at_target = trace.steps.iter().find(|step| step.value == target)?;
            if at_target.region.len() != target_shape.len()
                || at_target
                    .region
                    .iter()
                    .zip(target_shape)
                    .any(|(range, extent)| range.start > range.end || range.end > *extent)
            {
                return None;
            }
            boxes.push(at_target.region.clone());
        }
        Some(GraphValueDemandUnion {
            value: target,
            boxes,
        })
    }
}

//! Facts about successful results, not permission to eliminate errors or guards.
use crate::primitive::PrimitiveId::*;
pub use crate::types::DType;
use crate::{Value, contracts::ShapeRule, primitive::PrimitiveId};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TypeFact {
    #[default]
    Unknown,
    Exact(DType),
    IntOrFloat,
}
/// Physical representation is independent of the logical atom type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutFact {
    #[default]
    Unknown,
    Dense,
    AxisSparse,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub dtype: TypeFact,
    pub layout: LayoutFact,
    pub shape: Option<Vec<usize>>,
    pub rank: Option<usize>,
}
impl Facts {
    pub fn of(value: &Value) -> Self {
        if let crate::Data::Sparse(v) = value.data() {
            return Self {
                dtype: Self::of(v.fill()).dtype,
                layout: LayoutFact::AxisSparse,
                shape: Some(value.shape().to_vec()),
                rank: Some(value.shape().len()),
            };
        }
        let dtype = match value.data() {
            crate::Data::Bool(_) => DType::Bool,
            crate::Data::Int(_) => DType::Int,
            crate::Data::Float(_) => DType::Float,
            crate::Data::Char(_) => DType::Char,
            crate::Data::Boxed(_) => DType::Boxed,
            crate::Data::Sparse(_) => unreachable!(),
        };
        Self {
            dtype: TypeFact::Exact(dtype),
            layout: LayoutFact::Dense,
            shape: Some(value.shape().to_vec()),
            rank: Some(value.shape().len()),
        }
    }
}
/// J matches the shorter shape against the leading axes, never trailing axes.
fn agreement(left: &[usize], right: &[usize]) -> Option<Vec<usize>> {
    let (short, long) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    long.starts_with(short).then(|| long.to_vec())
}
pub(crate) fn infer(
    id: PrimitiveId,
    rule: ShapeRule,
    left: Option<&Facts>,
    right: &Facts,
) -> Facts {
    let shape = match rule {
        ShapeRule::PreserveRight => right.shape.clone(),
        ShapeRule::PrefixAgreement => left
            .and_then(|x| x.shape.as_ref())
            .zip(right.shape.as_ref())
            .and_then(|(x, y)| agreement(x, y)),
        ShapeRule::Ravel => right.shape.as_ref().and_then(|s| {
            if s.contains(&0) {
                Some(vec![0])
            } else {
                s.iter()
                    .try_fold(1usize, |n, d| n.checked_mul(*d))
                    .map(|n| vec![n])
            }
        }),
        ShapeRule::ReverseAxes => right
            .shape
            .as_ref()
            .map(|s| s.iter().copied().rev().collect()),
        ShapeRule::ShapeOf => right.rank.map(|r| vec![r]),
        ShapeRule::Tally | ShapeRule::Scalar => Some(vec![]),
        ShapeRule::Unknown => None,
    };
    let rank = shape.as_ref().map(Vec::len).or(match rule {
        ShapeRule::PreserveRight | ShapeRule::ReverseAxes => right.rank,
        ShapeRule::Ravel | ShapeRule::ShapeOf => Some(1),
        ShapeRule::Tally | ShapeRule::Scalar => Some(0),
        _ => None,
    });
    let dtype = match (id, left) {
        (Less, None) => TypeFact::Exact(DType::Boxed),
        (Equal | Less | Greater, Some(_)) => TypeFact::Exact(DType::Bool),
        (Shape | Tally | Multiply, None) => TypeFact::Exact(DType::Int),
        (Ravel | Reverse | Transpose | Add | Sparse, None) => right.dtype,
        (Add | Subtract | Multiply, Some(x))
            if x.dtype == TypeFact::Exact(DType::Int)
                && right.dtype == TypeFact::Exact(DType::Int) =>
        {
            TypeFact::IntOrFloat
        }
        _ => TypeFact::Unknown,
    };
    let layout = match (id, left, right.rank) {
        (Sparse, None, Some(0)) => right.layout,
        (Sparse, None, Some(_)) => LayoutFact::AxisSparse,
        (Shape | Tally, None, _) => LayoutFact::Dense,
        _ => LayoutFact::Unknown,
    };
    Facts {
        dtype,
        layout,
        shape,
        rank,
    }
}

/// One semantic cell-application boundary. Layers are ordered outermost to
/// innermost; explicit rank conjunction boundaries remain distinct from the
/// callable's innate rank boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellApplyBoundary {
    Explicit,
    Innate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepeatedSide {
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellApplyLayer {
    pub boundary: CellApplyBoundary,
    pub requested: crate::contracts::RankContract,
    pub effective_monad_rank: Option<usize>,
    pub effective_left_rank: Option<usize>,
    pub effective_right_rank: Option<usize>,
    pub left_frame: Option<Vec<usize>>,
    pub left_cell: Option<Vec<usize>>,
    pub right_frame: Vec<usize>,
    pub right_cell: Vec<usize>,
    pub common_frame_prefix: Option<Vec<usize>>,
    pub left_residual_frame: Vec<usize>,
    pub right_residual_frame: Vec<usize>,
    pub repeated_side: Option<RepeatedSide>,
    /// None means the known dyadic frames fail J prefix agreement.
    pub result_frame: Option<Vec<usize>>,
    pub requires_empty_frame_prototype: bool,
}

/// Logical iteration semantics only. This is deliberately not a physical loop
/// or a GPU mapping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellApplicationPlan {
    pub layers: Vec<CellApplyLayer>,
}

fn split_effective(shape: &[usize], cell_rank: usize) -> (Vec<usize>, Vec<usize>) {
    let cell_rank = cell_rank.min(shape.len());
    let frame_rank = shape.len() - cell_rank;
    (shape[..frame_rank].to_vec(), shape[frame_rank..].to_vec())
}

fn cell(input: &Facts, shape: Vec<usize>) -> Facts {
    Facts {
        dtype: input.dtype,
        layout: input.layout,
        rank: Some(shape.len()),
        shape: Some(shape),
    }
}

fn reduction(id: PrimitiveId, input: &Facts) -> Facts {
    if !matches!(id, Add | Subtract | Multiply | Divide) {
        return Facts::default();
    }
    let shape = input
        .shape
        .as_ref()
        .map(|s| s.get(1..).unwrap_or(&[]).to_vec());
    let dtype = match input.shape.as_deref() {
        Some([]) => input.dtype,
        Some([1, ..]) => input.dtype,
        Some([0, ..]) if matches!(id, Add | Multiply) => TypeFact::Exact(DType::Bool),
        Some(_) if id != Divide && input.dtype == TypeFact::Exact(DType::Int) => {
            TypeFact::IntOrFloat
        }
        _ => TypeFact::Unknown,
    };
    Facts {
        dtype,
        layout: LayoutFact::Unknown,
        shape,
        rank: input.rank.map(|r| r.saturating_sub(1)),
    }
}

fn dyad_frame_details(
    left: &[usize],
    right: &[usize],
) -> (
    Option<Vec<usize>>,
    Vec<usize>,
    Vec<usize>,
    Option<RepeatedSide>,
    Option<Vec<usize>>,
) {
    if left.len() <= right.len() {
        if !right.starts_with(left) {
            return (None, Vec::new(), Vec::new(), None, None);
        }
        let right_residual = right[left.len()..].to_vec();
        let repeated = (!right_residual.is_empty()).then_some(RepeatedSide::Left);
        (
            Some(left.to_vec()),
            Vec::new(),
            right_residual,
            repeated,
            Some(right.to_vec()),
        )
    } else {
        if !left.starts_with(right) {
            return (None, Vec::new(), Vec::new(), None, None);
        }
        let left_residual = left[right.len()..].to_vec();
        let repeated = (!left_residual.is_empty()).then_some(RepeatedSide::Right);
        (
            Some(right.to_vec()),
            left_residual,
            Vec::new(),
            repeated,
            Some(left.to_vec()),
        )
    }
}

fn plan_layer(
    boundary: CellApplyBoundary,
    requested: crate::contracts::RankContract,
    left: Option<&Facts>,
    right: &Facts,
) -> Option<(CellApplyLayer, Option<Facts>, Facts)> {
    let right_shape = right.shape.as_ref()?;
    if let Some(left) = left {
        let left_shape = left.shape.as_ref()?;
        let effective_left = requested.left.resolve(left_shape.len());
        let effective_right = requested.right.resolve(right_shape.len());
        let (left_frame, left_cell) = split_effective(left_shape, effective_left);
        let (right_frame, right_cell) = split_effective(right_shape, effective_right);
        let (common, left_residual, right_residual, repeated_side, result_frame) =
            dyad_frame_details(&left_frame, &right_frame);
        let empty = result_frame.as_ref().is_some_and(|frame| frame.contains(&0));
        let layer = CellApplyLayer {
            boundary,
            requested,
            effective_monad_rank: None,
            effective_left_rank: Some(effective_left),
            effective_right_rank: Some(effective_right),
            left_frame: Some(left_frame),
            left_cell: Some(left_cell.clone()),
            right_frame,
            right_cell: right_cell.clone(),
            common_frame_prefix: common,
            left_residual_frame: left_residual,
            right_residual_frame: right_residual,
            repeated_side,
            result_frame,
            requires_empty_frame_prototype: empty,
        };
        Some((layer, Some(cell(left, left_cell)), cell(right, right_cell)))
    } else {
        let effective = requested.monad.resolve(right_shape.len());
        let (right_frame, right_cell) = split_effective(right_shape, effective);
        let empty = right_frame.contains(&0);
        let layer = CellApplyLayer {
            boundary,
            requested,
            effective_monad_rank: Some(effective),
            effective_left_rank: None,
            effective_right_rank: None,
            left_frame: None,
            left_cell: None,
            right_frame: right_frame.clone(),
            right_cell: right_cell.clone(),
            common_frame_prefix: None,
            left_residual_frame: Vec::new(),
            right_residual_frame: Vec::new(),
            repeated_side: None,
            result_frame: Some(right_frame),
            requires_empty_frame_prototype: empty,
        };
        Some((layer, None, cell(right, right_cell)))
    }
}

pub(crate) fn infer_call(
    id: PrimitiveId,
    reduce: bool,
    explicit_ranks: &[crate::contracts::RankContract],
    innate_rank: Option<crate::contracts::RankContract>,
    left: Option<&Facts>,
    right: &Facts,
) -> (Facts, Option<CellApplicationPlan>) {
    use crate::contracts::{self, Valence};
    let base = |x: Option<&Facts>, y: &Facts| {
        if reduce {
            if x.is_some() {
                Facts::default()
            } else {
                reduction(id, y)
            }
        } else {
            let valence = if x.is_some() {
                Valence::Dyad
            } else {
                Valence::Monad
            };
            infer(id, contracts::for_primitive(id, valence).shape_rule, x, y)
        }
    };

    if explicit_ranks.is_empty() && innate_rank.is_none() {
        return (base(left, right), None);
    }

    let mut current_left = left.cloned();
    let mut current_right = right.clone();
    let mut layers = Vec::with_capacity(explicit_ranks.len() + usize::from(innate_rank.is_some()));

    for requested in explicit_ranks.iter().copied() {
        let Some((layer, next_left, next_right)) = plan_layer(
            CellApplyBoundary::Explicit,
            requested,
            current_left.as_ref(),
            &current_right,
        ) else {
            return (
                if explicit_ranks.is_empty() {
                    base(left, right)
                } else {
                    Facts::default()
                },
                None,
            );
        };
        let agreed = layer.result_frame.is_some();
        layers.push(layer);
        if !agreed {
            return (Facts::default(), Some(CellApplicationPlan { layers }));
        }
        current_left = next_left;
        current_right = next_right;
    }

    if let Some(requested) = innate_rank {
        let Some((layer, next_left, next_right)) = plan_layer(
            CellApplyBoundary::Innate,
            requested,
            current_left.as_ref(),
            &current_right,
        ) else {
            return (
                if explicit_ranks.is_empty() {
                    base(left, right)
                } else {
                    Facts::default()
                },
                None,
            );
        };
        let agreed = layer.result_frame.is_some();
        layers.push(layer);
        if !agreed {
            return (Facts::default(), Some(CellApplicationPlan { layers }));
        }
        current_left = next_left;
        current_right = next_right;
    }

    let has_empty_prototype = layers
        .iter()
        .any(|layer| layer.requires_empty_frame_prototype);
    let plan = CellApplicationPlan { layers };
    if has_empty_prototype {
        // IL4 will replace this conservative unknown with J fill-cell/prototype
        // abstract evaluation. The obligation is explicit in the plan now.
        return (Facts::default(), Some(plan));
    }

    let mut result = base(current_left.as_ref(), &current_right);
    for layer in plan.layers.iter().rev() {
        let Some(frame) = layer.result_frame.as_ref() else {
            return (Facts::default(), Some(plan));
        };
        if !frame.is_empty() {
            result.layout = LayoutFact::Unknown;
        }
        if let Some(shape) = result.shape.take() {
            let mut assembled = frame.clone();
            assembled.extend(shape);
            result.shape = Some(assembled);
        }
        result.rank = result
            .rank
            .and_then(|rank| rank.checked_add(frame.len()));
    }
    (result, Some(plan))
}

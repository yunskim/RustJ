//! Facts about successful results, not permission to eliminate errors or guards.
use crate::primitive::PrimitiveId::*;
use crate::{Value, contracts::ShapeRule, primitive::PrimitiveId};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DType {
    Bool,
    Int,
    Float,
    Char,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TypeFact {
    #[default]
    Unknown,
    Exact(DType),
    IntOrFloat,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub dtype: TypeFact,
    pub shape: Option<Vec<usize>>,
    pub rank: Option<usize>,
}
impl Facts {
    pub fn of(value: &Value) -> Self {
        let dtype = match value.data() {
            crate::Data::Bool(_) => DType::Bool,
            crate::Data::Int(_) => DType::Int,
            crate::Data::Float(_) => DType::Float,
            crate::Data::Char(_) => DType::Char,
        };
        Self {
            dtype: TypeFact::Exact(dtype),
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
        ShapeRule::Tally => Some(vec![]),
        ShapeRule::Unknown => None,
    };
    let rank = shape.as_ref().map(Vec::len).or(match rule {
        ShapeRule::PreserveRight | ShapeRule::ReverseAxes => right.rank,
        ShapeRule::Ravel | ShapeRule::ShapeOf => Some(1),
        ShapeRule::Tally => Some(0),
        _ => None,
    });
    let dtype = match (id, left) {
        (Equal | Less | Greater, Some(_)) => TypeFact::Exact(DType::Bool),
        (Shape | Tally | Multiply, None) => TypeFact::Exact(DType::Int),
        (Ravel | Reverse | Transpose | Add, None) => right.dtype,
        (Add | Subtract | Multiply, Some(x))
            if x.dtype == TypeFact::Exact(DType::Int)
                && right.dtype == TypeFact::Exact(DType::Int) =>
        {
            TypeFact::IntOrFloat
        }
        _ => TypeFact::Unknown,
    };
    Facts { dtype, shape, rank }
}

/// Cell/frame decomposition, independent of physical layout and worker count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankPlan {
    pub left_frame: Option<Vec<usize>>,
    pub left_cell: Option<Vec<usize>>,
    pub right_frame: Vec<usize>,
    pub right_cell: Vec<usize>,
    pub result_frame: Option<Vec<usize>>,
    pub requires_empty_frame_prototype: bool,
}
fn split(shape: &[usize], requested: i64) -> (Vec<usize>, Vec<usize>) {
    let cell_rank = if requested < 0 {
        shape
            .len()
            .saturating_sub(usize::try_from(requested.unsigned_abs()).unwrap_or(usize::MAX))
    } else {
        shape
            .len()
            .min(usize::try_from(requested).unwrap_or(usize::MAX))
    };
    let frame_rank = shape.len() - cell_rank;
    (shape[..frame_rank].to_vec(), shape[frame_rank..].to_vec())
}
fn cell(input: &Facts, shape: Vec<usize>) -> Facts {
    Facts {
        dtype: input.dtype,
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
        shape,
        rank: input.rank.map(|r| r.saturating_sub(1)),
    }
}

pub(crate) fn infer_call(
    id: PrimitiveId,
    reduce: bool,
    ranks: Option<[i64; 3]>,
    left: Option<&Facts>,
    right: &Facts,
) -> (Facts, Option<RankPlan>) {
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
    let Some(ranks) = ranks else {
        return (base(left, right), None);
    };
    let Some(right_shape) = &right.shape else {
        return (Facts::default(), None);
    };
    let (rf, rc) = split(
        right_shape,
        if left.is_some() { ranks[2] } else { ranks[0] },
    );
    let (lf, lc) = if let Some(left) = left {
        let Some(shape) = &left.shape else {
            return (Facts::default(), None);
        };
        let (frame, cell) = split(shape, ranks[1]);
        (Some(frame), Some(cell))
    } else {
        (None, None)
    };
    let result_frame = match &lf {
        Some(lf) => agreement(lf, &rf),
        None => Some(rf.clone()),
    };
    let empty = result_frame.as_ref().is_some_and(|f| f.contains(&0));
    let plan = RankPlan {
        left_frame: lf,
        left_cell: lc.clone(),
        right_frame: rf,
        right_cell: rc.clone(),
        result_frame: result_frame.clone(),
        requires_empty_frame_prototype: empty,
    };
    let Some(mut frame) = result_frame else {
        return (Facts::default(), Some(plan));
    };
    if empty {
        return (Facts::default(), Some(plan));
    }
    let right_cell = cell(right, rc);
    let left_cell = left.zip(lc).map(|(x, s)| cell(x, s));
    let result = base(left_cell.as_ref(), &right_cell);
    let shape = result.shape.map(|s| {
        frame.extend(s);
        frame
    });
    let frame_rank = plan.result_frame.as_ref().unwrap().len();
    let rank = result.rank.and_then(|r| r.checked_add(frame_rank));
    (
        Facts {
            dtype: result.dtype,
            shape,
            rank,
        },
        Some(plan),
    )
}

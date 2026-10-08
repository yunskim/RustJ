//! Facts about successful results, not permission to eliminate errors or guards.
use crate::primitive::PrimitiveId::*;
pub use crate::types::DType;
use crate::{
    Value,
    contracts::{ShapeRule, Valence},
    primitive::PrimitiveId,
    semantic::{FunctionEntity, FunctionHead, FunctionOperand},
};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TypeFact {
    #[default]
    Unknown,
    Exact(DType),
    IntOrFloat,
}
/// J-visible representation-class fact used by semantic/execution analysis.
///
/// J-visible representation class, not a physical layout.
/// `Dense` vs `AxisSparse` is observable through J sparse semantics; stride,
/// tile, device, buffer and other physical realization details belong to
/// representation/physical planning and must not be added here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RepresentationClassFact {
    #[default]
    Unknown,
    Dense,
    AxisSparse,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueRole {
    ShapeVector,
    AxisVector,
    AxisPermutation,
    RankSpecifier,
    IndexVector,
    CountVector,
    WindowSpec,
    StrideSpec,
    DilationSpec,
    PaddingSpec,
    SegmentDescriptor,
    Permutation,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValueRoleFacts {
    roles: Vec<ValueRole>,
}

impl ValueRoleFacts {
    pub fn contains(&self, role: ValueRole) -> bool {
        self.roles.contains(&role)
    }

    pub fn insert(&mut self, role: ValueRole) {
        if !self.contains(role) {
            self.roles.push(role);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = ValueRole> + '_ {
        self.roles.iter().copied()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub dtype: TypeFact,
    pub representation_class: RepresentationClassFact,
    pub shape: Option<Vec<usize>>,
    pub rank: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SemanticFacts {
    pub dtype: TypeFact,
    pub shape: Option<Vec<usize>>,
    pub rank: Option<usize>,
}

impl SemanticFacts {
    pub(crate) fn of(value: &Value) -> Self {
        if let crate::Data::Sparse(v) = value.data() {
            return Self {
                dtype: Self::of(v.fill()).dtype,
                shape: Some(value.shape().to_vec()),
                rank: Some(value.shape().len()),
            };
        }
        let dtype = match value.data() {
            crate::Data::Bool(_) => DType::Bool,
            crate::Data::Int(_) => DType::Int,
            crate::Data::Float(_) => DType::Float,
            crate::Data::Char(_) => DType::Char,
            crate::Data::Rational(_) => DType::Rational,
            crate::Data::ExtendedInt(_) => DType::ExtendedInt,
            crate::Data::Boxed(_) => DType::Boxed,
            crate::Data::Sparse(_) => unreachable!(),
        };
        Self {
            dtype: TypeFact::Exact(dtype),
            shape: Some(value.shape().to_vec()),
            rank: Some(value.shape().len()),
        }
    }
}

impl std::convert::From<&Facts> for SemanticFacts {
    fn from(facts: &Facts) -> Self {
        Self {
            dtype: facts.dtype,
            shape: facts.shape.clone(),
            rank: facts.rank,
        }
    }
}

impl Facts {
    pub fn of(value: &Value) -> Self {
        if let crate::Data::Sparse(v) = value.data() {
            return Self {
                dtype: Self::of(v.fill()).dtype,
                representation_class: RepresentationClassFact::AxisSparse,
                shape: Some(value.shape().to_vec()),
                rank: Some(value.shape().len()),
            };
        }
        let dtype = match value.data() {
            crate::Data::Bool(_) => DType::Bool,
            crate::Data::Int(_) => DType::Int,
            crate::Data::Float(_) => DType::Float,
            crate::Data::Char(_) => DType::Char,
            crate::Data::Rational(_) => DType::Rational,
            crate::Data::ExtendedInt(_) => DType::ExtendedInt,
            crate::Data::Boxed(_) => DType::Boxed,
            crate::Data::Sparse(_) => unreachable!(),
        };
        Self {
            dtype: TypeFact::Exact(dtype),
            representation_class: RepresentationClassFact::Dense,
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
fn infer_semantic_primitive(
    id: PrimitiveId,
    rule: ShapeRule,
    left: Option<&SemanticFacts>,
    right: &SemanticFacts,
) -> SemanticFacts {
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
        (Equal | Less | Greater | Find, Some(_)) => TypeFact::Exact(DType::Bool),
        (Shape | Tally, None) if right.dtype == TypeFact::Exact(DType::Rational) => {
            TypeFact::Exact(DType::ExtendedInt)
        }
        (Shape | Tally | Multiply | Subtract, None)
            if right.dtype == TypeFact::Exact(DType::ExtendedInt) =>
        {
            right.dtype
        }
        (Shape | Tally | Multiply, None)
            if matches!(
                right.dtype,
                TypeFact::Exact(
                    DType::Bool | DType::Int | DType::Float | DType::Char | DType::Boxed
                ) | TypeFact::IntOrFloat
            ) =>
        {
            TypeFact::Exact(DType::Int)
        }
        (Ravel | Reverse | Transpose | Add | Sparse, None) => right.dtype,
        (Add | Subtract | Multiply, Some(x))
            if x.dtype == TypeFact::Exact(DType::Int)
                && right.dtype == TypeFact::Exact(DType::Int) =>
        {
            TypeFact::IntOrFloat
        }
        _ => TypeFact::Unknown,
    };
    SemanticFacts { dtype, shape, rank }
}

pub(crate) fn infer(
    id: PrimitiveId,
    rule: ShapeRule,
    left: Option<&Facts>,
    right: &Facts,
) -> Facts {
    let left_semantic = left.map(SemanticFacts::from);
    let right_semantic = SemanticFacts::from(right);
    let semantic = infer_semantic_primitive(id, rule, left_semantic.as_ref(), &right_semantic);
    let representation_class = match (id, left, right.rank) {
        (Sparse, None, Some(0)) => right.representation_class,
        (Sparse, None, Some(_)) => RepresentationClassFact::AxisSparse,
        (Shape | Tally, None, _) => RepresentationClassFact::Dense,
        _ => RepresentationClassFact::Unknown,
    };
    Facts {
        dtype: semantic.dtype,
        representation_class,
        shape: semantic.shape,
        rank: semantic.rank,
    }
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

/// Semantic rank-cell cardinality; this never authorizes skipping a call.
///
/// A zero *frame* requires J fill-cell evaluation even though it produces no
/// ordinary result cells. A positive frame can contain empty *cells*; those
/// cells still have to be evaluated (for example +/ over each empty row).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RankFrameExecution {
    IncompatibleFrames,
    ZeroFrameNeedsFill,
    CellsPresent,
}

impl RankPlan {
    pub fn frame_execution(&self) -> RankFrameExecution {
        match &self.result_frame {
            None => RankFrameExecution::IncompatibleFrames,
            Some(frame) if frame.contains(&0) => RankFrameExecution::ZeroFrameNeedsFill,
            Some(_) => RankFrameExecution::CellsPresent,
        }
    }

    /// Structural fact only: an empty cell is not an empty result frame.
    pub fn has_empty_input_cell(&self) -> bool {
        self.left_cell
            .as_deref()
            .is_some_and(|cell| cell.contains(&0))
            || self.right_cell.contains(&0)
    }
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
/// Split known J shapes into Rank frame/cell geometry without evaluating
/// the operand or inferring the *result* type. Used identically by the
/// J-grammar graph view and A3 execution-basis analysis.
pub fn rank_plan_for_shapes(
    ranks: [i64; 3],
    left_shape: Option<&[usize]>,
    right_shape: &[usize],
) -> RankPlan {
    let (right_frame, right_cell) = split(
        right_shape,
        if left_shape.is_some() {
            ranks[2]
        } else {
            ranks[0]
        },
    );
    let (left_frame, left_cell) = match left_shape {
        Some(shape) => {
            let (frame, cell) = split(shape, ranks[1]);
            (Some(frame), Some(cell))
        }
        None => (None, None),
    };
    let result_frame = match &left_frame {
        Some(frame) => agreement(frame, &right_frame),
        None => Some(right_frame.clone()),
    };
    let requires_empty_frame_prototype = result_frame
        .as_ref()
        .is_some_and(|frame| frame.contains(&0));
    RankPlan {
        left_frame,
        left_cell,
        right_frame,
        right_cell,
        result_frame,
        requires_empty_frame_prototype,
    }
}

fn cell(input: &Facts, shape: Vec<usize>) -> Facts {
    Facts {
        dtype: input.dtype,
        representation_class: input.representation_class,
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
        Some(_)
            if matches!(
                input.dtype,
                TypeFact::Exact(DType::ExtendedInt | DType::Rational)
            ) =>
        {
            TypeFact::Unknown
        }
        Some([0, ..]) if matches!(id, Add | Multiply) => TypeFact::Exact(DType::Bool),
        Some(_) if id != Divide && input.dtype == TypeFact::Exact(DType::Int) => {
            TypeFact::IntOrFloat
        }
        _ => TypeFact::Unknown,
    };
    Facts {
        dtype,
        representation_class: RepresentationClassFact::Unknown,
        shape,
        rank: input.rank.map(|r| r.saturating_sub(1)),
    }
}

fn semantic_rank_triplet(function: &FunctionEntity) -> Option<[i64; 3]> {
    function_operand(function)?;
    function.requested_ranks()
}

fn function_operand(function: &FunctionEntity) -> Option<&Arc<FunctionEntity>> {
    match function.operands.first()? {
        FunctionOperand::Function(function) => Some(function),
        FunctionOperand::Noun { .. } => None,
    }
}

fn infer_derived_reduction(function: &Arc<FunctionEntity>, input: &Facts) -> Facts {
    if let FunctionHead::PrimitiveVerb(id) = function.head {
        return reduction(id, input);
    }

    let Some(shape) = input.shape.as_ref() else {
        return Facts::default();
    };
    if shape.is_empty() {
        return input.clone();
    }

    let items = shape[0];
    if items == 0 {
        return Facts::default();
    }

    let item_shape = shape[1..].to_vec();
    let item = cell(input, item_shape.clone());
    if items == 1 {
        return item;
    }

    let (step, _) = infer_semantic_call(function, Some(&item), &item);
    if items == 2 {
        return step;
    }

    // For a longer fold, one-step inference is stable only when feeding the
    // result back into the reducer preserves the logical cell shape/rank.
    if step.shape.as_deref() == Some(item_shape.as_slice()) && step.rank == Some(item_shape.len()) {
        step
    } else {
        Facts::default()
    }
}

fn infer_ranked_semantic_call(
    function: &Arc<FunctionEntity>,
    ranks: [i64; 3],
    left: Option<&Facts>,
    right: &Facts,
) -> (Facts, Option<RankPlan>) {
    let Some(right_shape) = &right.shape else {
        return (Facts::default(), None);
    };
    let left_shape = left.and_then(|facts| facts.shape.as_deref());
    if left.is_some() && left_shape.is_none() {
        return (Facts::default(), None);
    }
    let plan = rank_plan_for_shapes(ranks, left_shape, right_shape);
    let result_frame = plan.result_frame.clone();
    let empty = plan.requires_empty_frame_prototype;
    let rc = plan.right_cell.clone();
    let lc = plan.left_cell.clone();
    let Some(mut frame) = result_frame else {
        return (Facts::default(), Some(plan));
    };
    if empty {
        return (Facts::default(), Some(plan));
    }

    let right_cell = cell(right, rc);
    let left_cell = left.zip(lc).map(|(x, s)| cell(x, s));
    let (result, _) = infer_semantic_call(function, left_cell.as_ref(), &right_cell);
    let shape = result.shape.map(|s| {
        frame.extend(s);
        frame
    });
    let frame_rank = plan.result_frame.as_ref().unwrap().len();
    let rank = result.rank.and_then(|r| r.checked_add(frame_rank));
    (
        Facts {
            dtype: result.dtype,
            representation_class: RepresentationClassFact::Unknown,
            shape,
            rank,
        },
        Some(plan),
    )
}

/// Infer result facts from the semantic FunctionEntity rather than flattened
/// reduce/rank flags. Modifier nesting is semantically significant:
/// Rank(Insert(u)) and Insert(Rank(u)) must not share an inference path.
pub(crate) fn infer_semantic_call(
    function: &Arc<FunctionEntity>,
    left: Option<&Facts>,
    right: &Facts,
) -> (Facts, Option<RankPlan>) {
    match &function.head {
        FunctionHead::PrimitiveVerb(id) => {
            let valence = if left.is_some() {
                Valence::Dyad
            } else {
                Valence::Monad
            };
            (
                infer(
                    *id,
                    crate::contracts::for_primitive(*id, valence).shape_rule,
                    left,
                    right,
                ),
                None,
            )
        }
        FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert) => {
            if left.is_some() {
                return (Facts::default(), None);
            }
            let Some(operand) = function_operand(function) else {
                return (Facts::default(), None);
            };
            (infer_derived_reduction(operand, right), None)
        }
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank) => {
            let Some(operand) = function_operand(function) else {
                return (Facts::default(), None);
            };
            let Some(ranks) = semantic_rank_triplet(function) else {
                return (Facts::default(), None);
            };
            infer_ranked_semantic_call(operand, ranks, left, right)
        }
        FunctionHead::VocabularyPrimitive(_)
        | FunctionHead::TakeName { .. }
        | FunctionHead::NameRef(_)
        | FunctionHead::PrimitiveAdverb(_)
        | FunctionHead::PrimitiveConjunction(_)
        | FunctionHead::DefinitionConstructor(_)
        | FunctionHead::ExplicitDefinition(_)
        | FunctionHead::ModifierTrain
        | FunctionHead::Hook
        | FunctionHead::Fork => (Facts::default(), None),
    }
}

fn semantic_cell(input: &SemanticFacts, shape: Vec<usize>) -> SemanticFacts {
    SemanticFacts {
        dtype: input.dtype,
        rank: Some(shape.len()),
        shape: Some(shape),
    }
}

fn semantic_reduction(id: PrimitiveId, input: &SemanticFacts) -> SemanticFacts {
    if !matches!(id, Add | Subtract | Multiply | Divide) {
        return SemanticFacts::default();
    }
    let shape = input
        .shape
        .as_ref()
        .map(|s| s.get(1..).unwrap_or(&[]).to_vec());
    let dtype = match input.shape.as_deref() {
        Some([]) => input.dtype,
        Some([1, ..]) => input.dtype,
        Some(_)
            if matches!(
                input.dtype,
                TypeFact::Exact(DType::ExtendedInt | DType::Rational)
            ) =>
        {
            TypeFact::Unknown
        }
        Some([0, ..]) if matches!(id, Add | Multiply) => TypeFact::Exact(DType::Bool),
        Some(_) if id != Divide && input.dtype == TypeFact::Exact(DType::Int) => {
            TypeFact::IntOrFloat
        }
        _ => TypeFact::Unknown,
    };
    SemanticFacts {
        dtype,
        shape,
        rank: input.rank.map(|r| r.saturating_sub(1)),
    }
}

fn infer_derived_reduction_semantic(
    function: &Arc<FunctionEntity>,
    input: &SemanticFacts,
) -> SemanticFacts {
    if let FunctionHead::PrimitiveVerb(id) = function.head {
        return semantic_reduction(id, input);
    }

    let Some(shape) = input.shape.as_ref() else {
        return SemanticFacts::default();
    };
    if shape.is_empty() {
        return input.clone();
    }

    let items = shape[0];
    if items == 0 {
        return SemanticFacts::default();
    }

    let item_shape = shape[1..].to_vec();
    let item = semantic_cell(input, item_shape.clone());
    if items == 1 {
        return item;
    }

    let step = infer_semantic_projection(function, Some(&item), &item);
    if items == 2 {
        return step;
    }

    if step.shape.as_deref() == Some(item_shape.as_slice()) && step.rank == Some(item_shape.len()) {
        step
    } else {
        SemanticFacts::default()
    }
}

fn infer_ranked_semantic_projection(
    function: &Arc<FunctionEntity>,
    ranks: [i64; 3],
    left: Option<&SemanticFacts>,
    right: &SemanticFacts,
) -> SemanticFacts {
    let Some(right_shape) = &right.shape else {
        return SemanticFacts::default();
    };
    let left_shape = left.and_then(|facts| facts.shape.as_deref());
    if left.is_some() && left_shape.is_none() {
        return SemanticFacts::default();
    }
    let plan = rank_plan_for_shapes(ranks, left_shape, right_shape);
    let result_frame = plan.result_frame.clone();
    let rc = plan.right_cell.clone();
    let lc = plan.left_cell.clone();
    let Some(mut frame) = result_frame else {
        return SemanticFacts::default();
    };
    if frame.contains(&0) {
        // J requires fill/prototype execution to determine the cell result.
        return SemanticFacts::default();
    }

    let right_cell = semantic_cell(right, rc);
    let left_cell = left.zip(lc).map(|(x, s)| semantic_cell(x, s));
    let result = infer_semantic_projection(function, left_cell.as_ref(), &right_cell);
    let frame_rank = frame.len();
    let shape = result.shape.map(|s| {
        frame.extend(s);
        frame
    });
    let rank = result.rank.and_then(|r| r.checked_add(frame_rank));
    SemanticFacts {
        dtype: result.dtype,
        shape,
        rank,
    }
}

/// Graph-facing semantic fact projection. J Graph IR consumes this domain
/// directly; representation/layout facts remain owned by execution analysis.
pub(crate) fn infer_semantic_projection(
    function: &Arc<FunctionEntity>,
    left: Option<&SemanticFacts>,
    right: &SemanticFacts,
) -> SemanticFacts {
    match &function.head {
        FunctionHead::PrimitiveVerb(id) => {
            let valence = if left.is_some() {
                Valence::Dyad
            } else {
                Valence::Monad
            };
            infer_semantic_primitive(
                *id,
                crate::contracts::for_primitive(*id, valence).shape_rule,
                left,
                right,
            )
        }
        FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert) => {
            if left.is_some() {
                return SemanticFacts::default();
            }
            let Some(operand) = function_operand(function) else {
                return SemanticFacts::default();
            };
            infer_derived_reduction_semantic(operand, right)
        }
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank) => {
            let Some(operand) = function_operand(function) else {
                return SemanticFacts::default();
            };
            let Some(ranks) = semantic_rank_triplet(function) else {
                return SemanticFacts::default();
            };
            infer_ranked_semantic_projection(operand, ranks, left, right)
        }
        FunctionHead::VocabularyPrimitive(_)
        | FunctionHead::TakeName { .. }
        | FunctionHead::NameRef(_)
        | FunctionHead::PrimitiveAdverb(_)
        | FunctionHead::PrimitiveConjunction(_)
        | FunctionHead::DefinitionConstructor(_)
        | FunctionHead::ExplicitDefinition(_)
        | FunctionHead::ModifierTrain
        | FunctionHead::Hook
        | FunctionHead::Fork => SemanticFacts::default(),
    }
}

#[cfg(test)]
mod noun_rank_tests {
    use super::*;

    #[test]
    fn noun_left_rank_does_not_infer_from_right_verb_or_left_constant() {
        let program = crate::semantic::parse("3\"+").unwrap();
        let crate::semantic::ExprKind::VerbValue(verb) = program.expression.unwrap().kind else {
            panic!();
        };
        assert!(function_operand(&verb.entity).is_none());
        assert!(semantic_rank_triplet(&verb.entity).is_none());
        let input = SemanticFacts {
            dtype: TypeFact::Exact(DType::Int),
            shape: Some(vec![4]),
            rank: Some(1),
        };
        let result = infer_semantic_projection(&verb.entity, None, &input);
        assert!(result.shape.is_none());
        assert_eq!(result.dtype, TypeFact::Unknown);
    }
}

#[cfg(test)]
mod extended_type_tests {
    use super::*;
    #[test]
    fn extended_structural_result_facts_do_not_claim_machine_int() {
        let input = SemanticFacts::of(
            &crate::types::Scalar::ExtendedInt(std::sync::Arc::new(crate::types::BigInt::from(1)))
                .into_value()
                .unwrap(),
        );
        for (id, rule) in [
            (Shape, ShapeRule::ShapeOf),
            (Tally, ShapeRule::Tally),
            (Multiply, ShapeRule::PreserveRight),
        ] {
            let result = infer_semantic_primitive(id, rule, None, &input);
            assert_eq!(result.dtype, TypeFact::Exact(DType::ExtendedInt));
            let unknown = infer_semantic_primitive(id, rule, None, &SemanticFacts::default());
            assert_eq!(unknown.dtype, TypeFact::Unknown);
        }
        let empty = SemanticFacts {
            dtype: TypeFact::Exact(DType::ExtendedInt),
            shape: Some(vec![0]),
            rank: Some(1),
        };
        assert_eq!(semantic_reduction(Add, &empty).dtype, TypeFact::Unknown);
    }
}

#[cfg(test)]
mod rational_type_tests {
    use super::*;
    #[test]
    fn rational_facts_preserve_dtype_and_extended_counts_without_invented_arithmetic() {
        let input = SemanticFacts::of(
            &crate::types::Scalar::Rational(std::sync::Arc::new(
                crate::types::Rational::new(2.into(), 3.into()).unwrap(),
            ))
            .into_value()
            .unwrap(),
        );
        assert_eq!(input.dtype, TypeFact::Exact(DType::Rational));
        for (id, rule) in [(Shape, ShapeRule::ShapeOf), (Tally, ShapeRule::Tally)] {
            assert_eq!(
                infer_semantic_primitive(id, rule, None, &input).dtype,
                TypeFact::Exact(DType::ExtendedInt)
            );
            assert_eq!(
                infer_semantic_primitive(id, rule, None, &SemanticFacts::default()).dtype,
                TypeFact::Unknown
            );
        }
        assert_eq!(
            infer_semantic_primitive(Add, ShapeRule::PrefixAgreement, Some(&input), &input).dtype,
            TypeFact::Unknown
        );
        let empty = SemanticFacts {
            shape: Some(vec![0]),
            rank: Some(1),
            ..input
        };
        assert_eq!(semantic_reduction(Add, &empty).dtype, TypeFact::Unknown);
    }
}

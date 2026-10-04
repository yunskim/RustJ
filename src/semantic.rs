//! Target-independent semantic objects and binding/version model.
//! Parser construction lives in `parser`; execution and lowering are separate.
use crate::{Error, Result, Value};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FunctionPartOfSpeech {
    Verb,
    Adverb,
    Conjunction,
}

/// Concrete semantic RHS transport at parser/host boundaries.
/// Binding versions and occurrence provenance belong to the caller. This does
/// not merge noun storage with function identity or introduce function arrays.
/// Deliberately not Clone: cloning an owned Value can copy its entire payload.
#[derive(Debug)]
pub enum JEntity {
    Noun(Value),
    Function(Arc<FunctionEntity>),
}

/// Borrowed inspection without payload copies or reference-count updates.
#[derive(Clone, Copy, Debug)]
pub enum JEntityRef<'a> {
    Noun(&'a Value),
    Function(&'a FunctionEntity),
}

impl JEntity {
    pub fn as_ref(&self) -> JEntityRef<'_> {
        match self {
            Self::Noun(value) => JEntityRef::Noun(value),
            Self::Function(function) => JEntityRef::Function(function),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FunctionHead {
    PrimitiveVerb(crate::primitive::PrimitiveId),
    PrimitiveAdverb(crate::primitive::AdverbId),
    PrimitiveConjunction(crate::primitive::ConjunctionId),
    NameRef(String),
    ExplicitDefinition(Arc<crate::definition_code::DefinitionCode>),
    DefinitionConstructor(Arc<crate::definition_code::DefinitionSource>),
    /// Parser-production identities with no source operator token.
    Hook,
    Fork,
    /// cf.c CADVF: non-executing bident/trident returning a modifier.
    ModifierTrain,
}

#[derive(Debug)]
pub enum FunctionOperand {
    Function(Arc<FunctionEntity>),
    Noun {
        value: Value,
        span: std::ops::Range<usize>,
    },
}

/// Immutable semantic function object. Operands are shared references so large
/// trains/derived functions form DAGs rather than recursively copied Rust values.
/// This mirrors the structural role of jsource's common V block + f/g/h links,
/// not its execution-function-pointer layout.
#[derive(Debug)]
pub struct FunctionEntity {
    pub span: std::ops::Range<usize>,
    pub result_pos: FunctionPartOfSpeech,
    pub head: FunctionHead,
    pub operands: Vec<FunctionOperand>,
    /// Constructor-decoded gerund functions. Not source semantic operand edges.
    /// Retains intrinsic noun snapshots independently of the original boxed AR.
    pub decoded_gerund: Option<Vec<Arc<FunctionEntity>>>,
}
impl FunctionEntity {
    /// p.c stacks primitive modifiers and cf.c trains of primitive ACVs/nouns
    /// by value. This is a lookup policy, not an optimizer purity guarantee.
    pub(crate) fn is_nameless_modifier(&self) -> bool {
        self.is_primitive_modifier()
            || (matches!(self.head, FunctionHead::ModifierTrain)
                && self.operands.iter().all(|operand| match operand {
                    FunctionOperand::Noun { .. } => true,
                    FunctionOperand::Function(function) => {
                        function.operands.is_empty()
                            && matches!(
                                function.head,
                                FunctionHead::PrimitiveVerb(_)
                                    | FunctionHead::PrimitiveAdverb(_)
                                    | FunctionHead::PrimitiveConjunction(_)
                            )
                    }
                }))
    }
    pub(crate) fn with_decoded_gerund(
        mut entity: Arc<Self>,
        decoded: Option<Vec<Arc<FunctionEntity>>>,
    ) -> Arc<Self> {
        Arc::get_mut(&mut entity)
            .expect("fresh constructor entity")
            .decoded_gerund = decoded;
        entity
    }
    /// Registered, operand-free core construction semantics known without a call.
    pub(crate) fn is_primitive_modifier(&self) -> bool {
        self.operands.is_empty()
            && matches!(
                (&self.head, self.result_pos),
                (
                    FunctionHead::PrimitiveAdverb(_),
                    FunctionPartOfSpeech::Adverb
                ) | (
                    FunctionHead::PrimitiveConjunction(_),
                    FunctionPartOfSpeech::Conjunction
                )
            )
    }
    /// Construction identity can be observed without applying the modifier.
    /// A known train identity does not imply its application is implemented.
    pub(crate) fn is_known_modifier(&self) -> bool {
        self.is_primitive_modifier()
            || (matches!(self.head, FunctionHead::ModifierTrain)
                && matches!(
                    self.result_pos,
                    FunctionPartOfSpeech::Adverb | FunctionPartOfSpeech::Conjunction
                )
                && matches!(self.operands.len(), 2 | 3))
    }
    pub(crate) fn primitive(
        id: crate::primitive::PrimitiveId,
        span: std::ops::Range<usize>,
    ) -> Arc<Self> {
        Arc::new(Self {
            span,
            result_pos: FunctionPartOfSpeech::Verb,
            head: FunctionHead::PrimitiveVerb(id),
            operands: Vec::new(),
            decoded_gerund: None,
        })
    }

    pub(crate) fn name_ref(
        name: String,
        result_pos: FunctionPartOfSpeech,
        span: std::ops::Range<usize>,
    ) -> Arc<Self> {
        Arc::new(Self {
            span,
            result_pos,
            head: FunctionHead::NameRef(name),
            operands: Vec::new(),
            decoded_gerund: None,
        })
    }

    pub(crate) fn primitive_adverb(
        id: crate::primitive::AdverbId,
        span: std::ops::Range<usize>,
    ) -> Arc<Self> {
        Arc::new(Self {
            span,
            result_pos: FunctionPartOfSpeech::Adverb,
            head: FunctionHead::PrimitiveAdverb(id),
            operands: Vec::new(),
            decoded_gerund: None,
        })
    }

    pub(crate) fn primitive_conjunction(
        id: crate::primitive::ConjunctionId,
        span: std::ops::Range<usize>,
    ) -> Arc<Self> {
        Arc::new(Self {
            span,
            result_pos: FunctionPartOfSpeech::Conjunction,
            head: FunctionHead::PrimitiveConjunction(id),
            operands: Vec::new(),
            decoded_gerund: None,
        })
    }

    pub(crate) fn derived(
        head: FunctionHead,
        result_pos: FunctionPartOfSpeech,
        span: std::ops::Range<usize>,
        operands: Vec<FunctionOperand>,
    ) -> Arc<Self> {
        Arc::new(Self {
            span,
            result_pos,
            head,
            operands,
            decoded_gerund: None,
        })
    }
}

/// J rank-conjunction noun construction (cr.c::jtqq): rank, then length,
/// then numeric audit. Keep the original noun in FunctionEntity operands;
/// the requested triple is intrinsic and does not use actual argument ranks.
pub(crate) fn rank_noun_contract(value: &Value) -> Result<[i64; 3]> {
    if value.shape.len() > 1 {
        return Err(Error::Rank);
    }
    if !(1..=3).contains(&value.len()) {
        return Err(Error::Length);
    }
    let at = |index: usize| -> Result<i64> {
        let rank = match &value.data {
            crate::value::Data::Float(values) => {
                let x = values[index];
                if x.abs() < -(i64::MIN as f64) {
                    let rounded = x.round();
                    // u.c::jtvib uses fixed fuzz against the integer, even
                    // when the caller's comparison tolerance differs.
                    if x != rounded && (x - rounded).abs() > 2f64.powi(-44) * rounded.abs() {
                        return Err(Error::Domain);
                    }
                    rounded as i64
                } else if x > 0.0 {
                    i64::MAX
                } else {
                    // Matches vib for negative infinity and J's _. rank.
                    -i64::MAX
                }
            }
            _ => value.int_at(index)?,
        };
        // J's maximum array rank is 63. Retain the source noun unchanged.
        Ok(rank.clamp(-63, 63))
    };
    Ok(match value.len() {
        1 => {
            let r = at(0)?;
            [r, r, r]
        }
        2 => [at(1)?, at(0)?, at(1)?],
        _ => [at(0)?, at(1)?, at(2)?],
    })
}

#[derive(Clone, Debug)]
pub struct Verb {
    pub span: std::ops::Range<usize>,
    pub target: VerbTarget,
    /// Shared semantic identity/provenance graph.
    pub entity: Arc<FunctionEntity>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerbTarget {
    Primitive(crate::primitive::PrimitiveId),
    Named(String),
    /// Migration marker for a function whose executable identity is carried
    /// by the shared FunctionEntity graph (hook/fork and later open forms).
    Derived,
}

impl Verb {
    /// Compatibility adapter for the remaining runtime symbol carrier.
    /// The shared entity is authoritative; this never copies its DAG.
    pub(crate) fn from_entity(entity: Arc<FunctionEntity>) -> Result<Self> {
        if entity.result_pos != FunctionPartOfSpeech::Verb {
            return Err(Error::Domain);
        }
        let target = match &entity.head {
            FunctionHead::PrimitiveVerb(id) => VerbTarget::Primitive(*id),
            FunctionHead::NameRef(name) => VerbTarget::Named(name.clone()),
            _ => VerbTarget::Derived,
        };
        Ok(Self {
            span: entity.span.clone(),
            target,
            entity,
        })
    }
}

#[cfg(test)]
mod entity_adapter_tests {
    use super::*;
    #[test]
    fn verb_adapter_preserves_identity_and_rejects_modifier_pos() {
        let primitive = FunctionEntity::primitive(crate::primitive::PrimitiveId::Add, 3..4);
        let adapted = Verb::from_entity(primitive.clone()).unwrap();
        assert!(Arc::ptr_eq(&primitive, &adapted.entity));
        assert_eq!(
            adapted.target,
            VerbTarget::Primitive(crate::primitive::PrimitiveId::Add)
        );
        assert_eq!(adapted.span, 3..4);
        let named = FunctionEntity::name_ref("current".into(), FunctionPartOfSpeech::Verb, 9..16);
        let adapted = Verb::from_entity(named.clone()).unwrap();
        assert!(Arc::ptr_eq(&named, &adapted.entity));
        assert_eq!(adapted.target, VerbTarget::Named("current".into()));
        assert_eq!(adapted.span, 9..16);
        for function in [
            FunctionEntity::primitive_adverb(crate::primitive::AdverbId::Insert, 0..1),
            FunctionEntity::primitive_conjunction(crate::primitive::ConjunctionId::Rank, 0..1),
        ] {
            assert_eq!(
                Verb::from_entity(function).unwrap_err().kind(),
                "domain error"
            );
        }
    }
}
#[derive(Clone, Debug)]
pub struct Expr {
    pub span: std::ops::Range<usize>,
    pub kind: ExprKind,
}
#[derive(Clone, Debug)]
pub enum ExprKind {
    Group(Box<Expr>),
    VerbValue(Verb),
    /// First-class adverb/conjunction result, distinct from a noun or verb.
    ModifierValue(Arc<FunctionEntity>),
    Literal(Value),
    ReadName(String),
    Monad {
        verb: Verb,
        argument: Box<Expr>,
    },
    Dyad {
        verb: Verb,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}
#[derive(Clone, Debug)]
pub struct Program {
    pub source: String,
    pub assignment: Option<String>,
    pub assignment_span: Option<std::ops::Range<usize>>,
    pub expression: Option<Expr>,
    /// Parser-row provenance, separate from semantic operation payloads.
    pub reductions: Vec<crate::parser::ParseReduction>,
    pub assignment_source: Option<crate::parser::AssignmentSource>,
    /// Read-only analysis dependencies, outside intrinsic function identity.
    pub modifier_snapshots: Vec<ModifierSnapshot>,
}
/// Maximum number of edges from a parsed root to a leaf.
pub const MAX_EXPR_DEPTH: usize = 128;

impl From<crate::primitive::PrimitivePartOfSpeech> for FunctionPartOfSpeech {
    fn from(pos: crate::primitive::PrimitivePartOfSpeech) -> Self {
        match pos {
            crate::primitive::PrimitivePartOfSpeech::Verb => Self::Verb,
            crate::primitive::PrimitivePartOfSpeech::Adverb => Self::Adverb,
            crate::primitive::PrimitivePartOfSpeech::Conjunction => Self::Conjunction,
        }
    }
}

/// Versions identify successful writes within one Engine, not physical buffers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameVersion(pub u64);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameUse {
    pub name: String,
    pub version: NameVersion,
    pub span: std::ops::Range<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingWrite {
    pub name: String,
    pub previous: Option<NameVersion>,
    pub proposed: NameVersion,
    pub span: std::ops::Range<usize>,
}
/// Known construction identity used by a non-executing frontend observation.
/// Versions are local to its catalog/Engine; no executable guard is implied.
#[derive(Clone, Debug)]
pub struct ModifierSnapshot {
    pub name: String,
    pub version: NameVersion,
    pub expected: FunctionPartOfSpeech,
    pub function: Arc<FunctionEntity>,
    pub span: std::ops::Range<usize>,
}

/// Analysis snapshot only. It cannot be executed later as a cached plan.
#[derive(Clone, Debug)]
pub struct BoundProgram {
    pub program: Program,
    pub reads: Vec<NameUse>,
    /// Dynamic calls are not pinned to a noun assignment version.
    pub verb_references: Vec<(String, std::ops::Range<usize>)>,
    pub write: Option<PendingWrite>,
}

pub(crate) fn bind(
    program: Program,
    lookup: impl Fn(&str) -> Option<NameVersion>,
) -> Result<BoundProgram> {
    let mut pending = Vec::new();
    let mut verb_references = Vec::new();
    let mut stack = Vec::new();
    if let Some(expr) = &program.expression {
        stack.push(expr);
    }
    while let Some(expr) = stack.pop() {
        let function_root = match &expr.kind {
            ExprKind::VerbValue(v)
            | ExprKind::Monad { verb: v, .. }
            | ExprKind::Dyad { verb: v, .. } => Some(v.entity.as_ref()),
            ExprKind::ModifierValue(f) => Some(f.as_ref()),
            _ => None,
        };
        if let Some(function) = function_root {
            let mut functions = vec![function];
            while let Some(function) = functions.pop() {
                if let FunctionHead::NameRef(name) = &function.head {
                    verb_references.push((name.clone(), function.span.clone()));
                }
                for operand in function.operands.iter().rev() {
                    if let FunctionOperand::Function(child) = operand {
                        functions.push(child.as_ref());
                    }
                }
            }
        }
        match &expr.kind {
            ExprKind::ReadName(name) => pending.push((name.clone(), expr.span.clone())),
            ExprKind::Group(inner) => stack.push(inner),
            ExprKind::Monad { argument, .. } => stack.push(argument),
            ExprKind::Dyad { left, right, .. } => {
                stack.push(left);
                stack.push(right);
            }
            ExprKind::Literal(_) | ExprKind::VerbValue(_) | ExprKind::ModifierValue(_) => {}
        }
    }
    pending.sort_by_key(|(_, span)| span.start);
    let reads = pending
        .into_iter()
        .map(|(name, span)| {
            let version = lookup(&name).ok_or_else(|| Error::Value(name.clone()))?;
            Ok(NameUse {
                name,
                version,
                span,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let write = if let Some(name) = &program.assignment {
        let previous = lookup(name);
        let proposed = NameVersion(
            previous
                .map_or(0, |v| v.0)
                .checked_add(1)
                .ok_or(Error::Limit)?,
        );
        Some(PendingWrite {
            name: name.clone(),
            previous,
            proposed,
            span: program.assignment_span.clone().expect("assignment span"),
        })
    } else {
        None
    };
    Ok(BoundProgram {
        program,
        reads,
        verb_references,
        write,
    })
}

// Compatibility exports for existing consumers. The implementations live in parser.rs.
pub use crate::parser::{ParseClass, ParseRow, match_parse_row, parse, parse_diagnostic};

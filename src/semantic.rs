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

impl FunctionOperand {
    /// Inspect the concrete RHS without copying a noun or retaining a function.
    /// Operand provenance remains available separately through `span`.
    pub fn as_entity_ref(&self) -> JEntityRef<'_> {
        match self {
            Self::Noun { value, .. } => JEntityRef::Noun(value),
            Self::Function(function) => JEntityRef::Function(function),
        }
    }

    /// Original operand provenance, not the span of a later application.
    pub fn span(&self) -> &std::ops::Range<usize> {
        match self {
            Self::Noun { span, .. } => span,
            Self::Function(function) => &function.span,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForkSemantics {
    Ordinary,
    Capped,
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
    /// Constructor-fixed meaning for a Fork. Original source operands survive;
    /// this is neither a call-site fact nor a late binding/purity proof.
    pub fork_semantics: Option<ForkSemantics>,
    /// Header copied when a NAME enters the parser stack (sc.c::namerefacv).
    /// Does not fix the eventual executable binding or imply purity.
    pub name_ranks: Option<[i64; 3]>,
}
impl FunctionEntity {
    pub(crate) fn with_name_ranks(mut entity: Arc<Self>, ranks: Option<[i64; 3]>) -> Arc<Self> {
        debug_assert!(matches!(entity.head, FunctionHead::NameRef(_)));
        Arc::get_mut(&mut entity).expect("fresh nameref").name_ranks = ranks;
        entity
    }
    /// Intrinsic header ranks, separate from call-site rank/frame facts.
    pub fn innate_ranks(&self) -> Option<[i64; 3]> {
        self.innate_ranks_at(0)
    }
    fn innate_ranks_at(&self, depth: usize) -> Option<[i64; 3]> {
        if depth > MAX_EXPR_DEPTH || self.result_pos != FunctionPartOfSpeech::Verb {
            return None;
        }
        match self.head {
            FunctionHead::PrimitiveVerb(id) => Some(id.innate_ranks()),
            FunctionHead::NameRef(_) => self.name_ranks,
            FunctionHead::ExplicitDefinition(_) | FunctionHead::Hook | FunctionHead::Fork => {
                Some([63; 3])
            }
            FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert)
            | FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Atop) => {
                Some([63; 3])
            }
            FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::PrefixInfix) => {
                Some([63, 0, 63])
            }
            FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank) => {
                if self.decoded_gerund.is_some() {
                    return Some([63; 3]);
                }
                self.requested_ranks_at(depth + 1)
                    .map(|ranks| ranks.map(|rank| if rank < 0 { 63 } else { rank }))
            }
            _ => None,
        }
    }
    /// cr.c::jtqq request: noun triple or the right verb's fixed header.
    /// Never execute or resolve the right operand here.
    pub(crate) fn requested_ranks(&self) -> Option<[i64; 3]> {
        self.requested_ranks_at(0)
    }
    fn requested_ranks_at(&self, depth: usize) -> Option<[i64; 3]> {
        if depth > MAX_EXPR_DEPTH
            || !matches!(
                self.head,
                FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank)
            )
        {
            return None;
        }
        match self.operands.get(1)? {
            FunctionOperand::Noun { value, .. } => rank_noun_contract(value).ok(),
            FunctionOperand::Function(function) => function.innate_ranks_at(depth + 1),
        }
    }

    /// p.c stacks primitive modifiers and cf.c trains of primitive ACVs/nouns
    /// by value. This is a lookup policy, not an optimizer purity guarantee.
    pub(crate) fn is_nameless_modifier(&self) -> bool {
        self.is_primitive_modifier()
            || (matches!(self.head, FunctionHead::ModifierTrain)
                && self
                    .operands
                    .iter()
                    .all(|operand| match operand.as_entity_ref() {
                        JEntityRef::Noun(_) => true,
                        JEntityRef::Function(function) => {
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
            fork_semantics: None,
            name_ranks: None,
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
            fork_semantics: None,
            name_ranks: None,
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
            fork_semantics: None,
            name_ranks: None,
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
            fork_semantics: None,
            name_ranks: None,
        })
    }

    pub(crate) fn derived(
        head: FunctionHead,
        result_pos: FunctionPartOfSpeech,
        span: std::ops::Range<usize>,
        operands: Vec<FunctionOperand>,
    ) -> Arc<Self> {
        let fork_semantics = matches!(head, FunctionHead::Fork).then(|| {
            if matches!(operands.first(), Some(FunctionOperand::Function(first))
                if matches!(first.head, FunctionHead::PrimitiveVerb(crate::primitive::PrimitiveId::Cap)))
            {
                ForkSemantics::Capped
            } else {
                ForkSemantics::Ordinary
            }
        });
        Arc::new(Self {
            span,
            result_pos,
            head,
            operands,
            decoded_gerund: None,
            fork_semantics,
            name_ranks: None,
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
    /// Constructor-time single-name cap inspections, not executable namerefs.
    pub fork_name_reads: Vec<NameUse>,
    pub name_rank_snapshots: Vec<NameRankSnapshot>,
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

/// Parser-time header observation; version/absence belongs to this sidecar,
/// not immutable function identity. This is not an executable binding guard.
#[derive(Clone, Debug)]
pub struct NameRankSnapshot {
    pub name: String,
    pub version: Option<NameVersion>,
    pub ranks: Option<[i64; 3]>,
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
                for (index, operand) in function.operands.iter().enumerate().rev() {
                    if (index == 0 && function.fork_semantics == Some(ForkSemantics::Capped))
                        || (index == 1
                            && matches!(
                                function.head,
                                FunctionHead::PrimitiveConjunction(
                                    crate::primitive::ConjunctionId::Rank
                                )
                            ))
                    {
                        continue;
                    }
                    if let JEntityRef::Function(child) = operand.as_entity_ref() {
                        functions.push(child);
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
    let mut reads = pending
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
    reads.extend(program.fork_name_reads.iter().cloned());
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

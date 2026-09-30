//! C1 prototype: execution-free parsing into a backend-neutral semantic tree.
//! Nodes retain byte spans; binding and execution remain separate phases.
use crate::{
    Error, Result, Value,
    syntax::{Token, lex_spanned},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FunctionFormId(pub u32);
impl FunctionFormId {
    /// Built-in insert adverb application. Stable only inside the current
    /// semantic-IR schema; it is not the jsource C id byte.
    pub const INSERT: Self = Self(1);
    /// Built-in rank conjunction application.
    pub const RANK: Self = Self(2);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FunctionPartOfSpeech {
    Verb,
    Adverb,
    Conjunction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FunctionHead {
    Primitive(crate::primitive::PrimitiveId),
    NameRef(String),
    /// Open-form derived identity. New J/extension forms should be registered
    /// by form id rather than growing a closed DerivedVerb enum.
    Derived(FunctionFormId),
}

#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub struct FunctionEntity {
    pub span: std::ops::Range<usize>,
    pub result_pos: FunctionPartOfSpeech,
    pub head: FunctionHead,
    pub operands: Vec<FunctionOperand>,
}
impl FunctionEntity {
    fn primitive(id: crate::primitive::PrimitiveId, span: std::ops::Range<usize>) -> Arc<Self> {
        Arc::new(Self {
            span,
            result_pos: FunctionPartOfSpeech::Verb,
            head: FunctionHead::Primitive(id),
            operands: Vec::new(),
        })
    }

    fn name_ref(name: String, span: std::ops::Range<usize>) -> Arc<Self> {
        Arc::new(Self {
            span,
            result_pos: FunctionPartOfSpeech::Verb,
            head: FunctionHead::NameRef(name),
            operands: Vec::new(),
        })
    }

    fn derived(
        form: FunctionFormId,
        result_pos: FunctionPartOfSpeech,
        span: std::ops::Range<usize>,
        operands: Vec<FunctionOperand>,
    ) -> Arc<Self> {
        Arc::new(Self {
            span,
            result_pos,
            head: FunctionHead::Derived(form),
            operands,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Verb {
    pub span: std::ops::Range<usize>,
    pub target: VerbTarget,
    /// Shared semantic identity/provenance graph.
    pub entity: Arc<FunctionEntity>,
    /// Legacy runtime compatibility fields. Do not add more modifier kinds here;
    /// migrate runtime/analyzer consumers to the shared entity graph instead.
    pub reduce: bool,
    pub rank: Option<[i64; 3]>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerbTarget {
    Primitive(crate::primitive::PrimitiveId),
    Named(String),
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
}
/// Maximum number of edges from a parsed root to a leaf.
pub const MAX_EXPR_DEPTH: usize = 128;

enum Item {
    Noun(Expr, usize),
    Verb(Verb),
}

/// Parse without reading bindings, changing state, or invoking any kernels.
pub fn parse(source: &str) -> Result<Program> {
    parse_with(source, None, false)
}

pub(crate) fn parse_runtime(source: &str, noun: &dyn Fn(&str) -> Option<Value>) -> Result<Program> {
    parse_with(source, Some(noun), true)
}

pub(crate) fn parse_analysis(
    source: &str,
    noun: &dyn Fn(&str) -> Option<Value>,
) -> Result<Program> {
    parse_with(source, Some(noun), false)
}

type NounLookup<'a> = Option<&'a dyn Fn(&str) -> Option<Value>>;
fn parse_with(source: &str, noun: NounLookup<'_>, snapshot: bool) -> Result<Program> {
    let spanned = lex_spanned(source)?;
    let spans: Vec<_> = spanned.iter().map(|t| t.span.clone()).collect();
    let mut tokens: Vec<_> = spanned.into_iter().map(|t| t.token).collect();
    let mut assignment_span = None;
    let mut assignment = None;
    let expression = if tokens.is_empty() {
        None
    } else {
        let (expr, expr_spans) = if tokens.len() > 1 && matches!(tokens[1], Token::Assign) {
            let Token::Name(name) = &tokens[0] else {
                return Err(Error::Syntax("assignment target".into()));
            };
            assignment = Some((*name).to_owned());
            assignment_span = Some(spans[0].clone());
            (&mut tokens[2..], &spans[2..])
        } else {
            (tokens.as_mut_slice(), spans.as_slice())
        };
        let mut pos = 0;
        let (result, _) = expression(expr, expr_spans, &mut pos, false, 0, noun, snapshot)?;
        if pos != expr.len() {
            return Err(Error::Syntax("trailing tokens".into()));
        }
        Some(result)
    };
    Ok(Program {
        source: source.to_owned(),
        assignment,
        assignment_span,
        expression,
    })
}
fn expression(
    tokens: &mut [Token<'_>],
    spans: &[std::ops::Range<usize>],
    pos: &mut usize,
    nested: bool,
    depth: usize,
    noun: NounLookup<'_>,
    snapshot: bool,
) -> Result<(Expr, usize)> {
    if depth > MAX_EXPR_DEPTH {
        return Err(Error::Limit);
    }
    let mut items = Vec::new();
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Close => {
                if nested {
                    break;
                } else {
                    return Err(Error::Syntax("unexpected )".into()));
                }
            }
            Token::Open => {
                let start = spans[*pos].start;
                *pos += 1;
                let (v, height) = expression(tokens, spans, pos, true, depth + 1, noun, snapshot)?;
                let height = checked_height(height)?;
                if !matches!(tokens.get(*pos), Some(Token::Close)) {
                    return Err(Error::Syntax("missing )".into()));
                }
                let v = Expr {
                    span: start..spans[*pos].end,
                    kind: ExprKind::Group(Box::new(v)),
                };
                *pos += 1;
                if let ExprKind::Group(inner) = &v.kind {
                    if let ExprKind::VerbValue(verb) = &inner.kind {
                        let mut verb = verb.clone();
                        verb.span = v.span;
                        items.push(Item::Verb(verb));
                        continue;
                    }
                }
                items.push(Item::Noun(v, height));
            }
            Token::Scalar(v) => {
                items.push(Item::Noun(
                    Expr {
                        span: spans[*pos].clone(),
                        kind: ExprKind::Literal(v.clone().into_value()?),
                    },
                    0,
                ));
                *pos += 1;
            }
            Token::Noun(_) => {
                let Token::Noun(v) = std::mem::replace(&mut tokens[*pos], Token::Open) else {
                    unreachable!()
                };
                items.push(Item::Noun(
                    Expr {
                        span: spans[*pos].clone(),
                        kind: ExprKind::Literal(*v),
                    },
                    0,
                ));
                *pos += 1;
            }
            Token::Verb(_) | Token::Name(_) => {
                if let Token::Name(n) = &tokens[*pos] {
                    let kind = match noun {
                        None => Some(ExprKind::ReadName((*n).to_owned())),
                        Some(lookup) => lookup(n).map(|v| {
                            if snapshot {
                                ExprKind::Literal(v)
                            } else {
                                ExprKind::ReadName((*n).to_owned())
                            }
                        }),
                    };
                    if let Some(kind) = kind {
                        items.push(Item::Noun(
                            Expr {
                                span: spans[*pos].clone(),
                                kind,
                            },
                            0,
                        ));
                        *pos += 1;
                        continue;
                    }
                }
                let target = match &tokens[*pos] {
                    Token::Verb(id) => VerbTarget::Primitive(*id),
                    Token::Name(n) => VerbTarget::Named((*n).to_owned()),
                    _ => unreachable!(),
                };
                let verb_span = spans[*pos].clone();
                let entity = match &target {
                    VerbTarget::Primitive(id) => FunctionEntity::primitive(*id, verb_span.clone()),
                    VerbTarget::Named(name) => {
                        FunctionEntity::name_ref(name.clone(), verb_span.clone())
                    }
                };
                let mut verb = Verb {
                    span: verb_span,
                    target,
                    entity,
                    reduce: false,
                    rank: None,
                };
                *pos += 1;
                if matches!(tokens.get(*pos), Some(Token::Slash)) {
                    let slash_span = spans[*pos].clone();
                    let derived_span = verb.span.start..slash_span.end;
                    verb.entity = FunctionEntity::derived(
                        FunctionFormId::INSERT,
                        FunctionPartOfSpeech::Verb,
                        derived_span.clone(),
                        vec![FunctionOperand::Function(verb.entity.clone())],
                    );
                    verb.span = derived_span;
                    verb.reduce = true;
                    *pos += 1;
                }
                if matches!(tokens.get(*pos), Some(Token::Rank)) {
                    *pos += 1;
                    let v = match tokens.get(*pos) {
                        Some(Token::Scalar(v)) => v.clone().into_value()?,
                        Some(Token::Noun(v)) => (**v).clone(),
                        _ => return Err(Error::Syntax("rank needs a numeric literal".into())),
                    };
                    if v.is_empty() || v.len() > 3 {
                        return Err(Error::Length);
                    }
                    let at = |i| v.int_at(i);
                    let ranks = match v.len() {
                        1 => [at(0)?, at(0)?, at(0)?],
                        2 => [at(1)?, at(0)?, at(1)?],
                        _ => [at(0)?, at(1)?, at(2)?],
                    };
                    let rank_span = spans[*pos].clone();
                    let derived_span = verb.span.start..rank_span.end;
                    verb.entity = FunctionEntity::derived(
                        FunctionFormId::RANK,
                        FunctionPartOfSpeech::Verb,
                        derived_span.clone(),
                        vec![
                            FunctionOperand::Function(verb.entity.clone()),
                            FunctionOperand::Noun {
                                value: v,
                                span: rank_span,
                            },
                        ],
                    );
                    verb.span = derived_span;
                    verb.rank = Some(ranks);
                    *pos += 1;
                }
                verb.span.end = spans[*pos - 1].end;
                items.push(Item::Verb(verb));
            }
            _ => {
                return Err(Error::Unsupported(
                    "assignment/modifier in expression".into(),
                ));
            }
        }
    }
    if items.len() == 1 && matches!(items.first(), Some(Item::Verb(_))) {
        let Some(Item::Verb(verb)) = items.pop() else {
            unreachable!()
        };
        return Ok((
            Expr {
                span: verb.span.clone(),
                kind: ExprKind::VerbValue(verb),
            },
            0,
        ));
    }
    let Some(Item::Noun(mut rhs, mut height)) = items.pop() else {
        return Err(Error::Syntax("expected right argument".into()));
    };
    while let Some(item) = items.pop() {
        let Item::Verb(v) = item else {
            return Err(Error::Syntax("adjacent nouns".into()));
        };
        if matches!(items.last(), Some(Item::Noun(_, _))) {
            let Some(Item::Noun(lhs, left_height)) = items.pop() else {
                unreachable!()
            };
            if v.reduce {
                return Err(Error::Unsupported("dyadic derived verb".into()));
            }
            height = checked_height(height.max(left_height))?;
            rhs = Expr {
                span: lhs.span.start..rhs.span.end,
                kind: ExprKind::Dyad {
                    verb: v,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
            };
        } else {
            height = checked_height(height)?;
            rhs = Expr {
                span: v.span.start..rhs.span.end,
                kind: ExprKind::Monad {
                    verb: v,
                    argument: Box::new(rhs),
                },
            };
        }
    }

    Ok((rhs, height))
}

fn checked_height(child_height: usize) -> Result<usize> {
    let height = child_height + 1;
    if height > MAX_EXPR_DEPTH {
        Err(Error::Limit)
    } else {
        Ok(height)
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
        let verb = match &expr.kind {
            ExprKind::VerbValue(v)
            | ExprKind::Monad { verb: v, .. }
            | ExprKind::Dyad { verb: v, .. } => Some(v),
            _ => None,
        };
        if let Some(Verb {
            target: VerbTarget::Named(name),
            span,
            ..
        }) = verb
        {
            verb_references.push((name.clone(), span.clone()));
        }
        match &expr.kind {
            ExprKind::ReadName(name) => pending.push((name.clone(), expr.span.clone())),
            ExprKind::Group(inner) => stack.push(inner),
            ExprKind::Monad { argument, .. } => stack.push(argument),
            ExprKind::Dyad { left, right, .. } => {
                stack.push(left);
                stack.push(right);
            }
            ExprKind::Literal(_) | ExprKind::VerbValue(_) => {}
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

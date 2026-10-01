//! C1 prototype: execution-free parsing into a backend-neutral semantic tree.
//! Nodes retain byte spans; binding and execution remain separate phases.
use crate::{
    Error, Result, Value,
    enqueuer::{EnqueueClass, EnqueuedPayload, EnqueuedWord, enqueue},
    error::{DiagnosticPhase, ErrorContext},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FunctionPartOfSpeech {
    Verb,
    Adverb,
    Conjunction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FunctionHead {
    PrimitiveVerb(crate::primitive::PrimitiveId),
    PrimitiveAdverb(crate::primitive::AdverbId),
    PrimitiveConjunction(crate::primitive::ConjunctionId),
    NameRef(String),
    /// Parser-production identities with no source operator token.
    Hook,
    Fork,
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
}
impl FunctionEntity {
    fn primitive(id: crate::primitive::PrimitiveId, span: std::ops::Range<usize>) -> Arc<Self> {
        Arc::new(Self {
            span,
            result_pos: FunctionPartOfSpeech::Verb,
            head: FunctionHead::PrimitiveVerb(id),
            operands: Vec::new(),
        })
    }

    fn name_ref(
        name: String,
        result_pos: FunctionPartOfSpeech,
        span: std::ops::Range<usize>,
    ) -> Arc<Self> {
        Arc::new(Self {
            span,
            result_pos,
            head: FunctionHead::NameRef(name),
            operands: Vec::new(),
        })
    }

    fn primitive_adverb(
        id: crate::primitive::AdverbId,
        span: std::ops::Range<usize>,
    ) -> Arc<Self> {
        Arc::new(Self {
            span,
            result_pos: FunctionPartOfSpeech::Adverb,
            head: FunctionHead::PrimitiveAdverb(id),
            operands: Vec::new(),
        })
    }

    fn primitive_conjunction(
        id: crate::primitive::ConjunctionId,
        span: std::ops::Range<usize>,
    ) -> Arc<Self> {
        Arc::new(Self {
            span,
            result_pos: FunctionPartOfSpeech::Conjunction,
            head: FunctionHead::PrimitiveConjunction(id),
            operands: Vec::new(),
        })
    }

    fn derived(
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
        })
    }
}

fn train_hook(f: Verb, g: Verb) -> Verb {
    let span = f.span.start..g.span.end;
    Verb {
        span: span.clone(),
        target: VerbTarget::Derived,
        entity: FunctionEntity::derived(
            FunctionHead::Hook,
            FunctionPartOfSpeech::Verb,
            span,
            vec![
                FunctionOperand::Function(f.entity),
                FunctionOperand::Function(g.entity),
            ],
        ),
    }
}

fn train_fork(f: Verb, g: Verb, h: Verb) -> Verb {
    let span = f.span.start..h.span.end;
    Verb {
        span: span.clone(),
        target: VerbTarget::Derived,
        entity: FunctionEntity::derived(
            FunctionHead::Fork,
            FunctionPartOfSpeech::Verb,
            span,
            vec![
                FunctionOperand::Function(f.entity),
                FunctionOperand::Function(g.entity),
                FunctionOperand::Function(h.entity),
            ],
        ),
    }
}

/// Collapse one contiguous verb train using J's right-to-left hook/fork
/// construction. The build is iterative so large trains do not recurse while
/// being constructed, and each derived node only holds shared operand handles.
fn make_verb_train(mut verbs: Vec<Verb>) -> Result<Verb> {
    match verbs.len() {
        0 => return Err(Error::Syntax("empty verb train".into())),
        1 => return Ok(verbs.pop().expect("one verb")),
        2 => {
            let g = verbs.pop().expect("right hook verb");
            let f = verbs.pop().expect("left hook verb");
            return Ok(train_hook(f, g));
        }
        _ => {}
    }

    let h = verbs.pop().expect("fork h");
    let g = verbs.pop().expect("fork g");
    let f = verbs.pop().expect("fork f");
    let mut tail = train_fork(f, g, h);
    while verbs.len() >= 2 {
        let g = verbs.pop().expect("train g");
        let f = verbs.pop().expect("train f");
        tail = train_fork(f, g, tail);
    }
    if let Some(f) = verbs.pop() {
        tail = train_hook(f, tail);
    }
    Ok(tail)
}

fn apply_adverb(left: Verb, operator: Arc<FunctionEntity>) -> Result<Verb> {
    debug_assert_eq!(operator.result_pos, FunctionPartOfSpeech::Adverb);
    let span = left.span.start..operator.span.end;
    Ok(Verb {
        span: span.clone(),
        target: VerbTarget::Derived,
        entity: FunctionEntity::derived(
            operator.head.clone(),
            FunctionPartOfSpeech::Verb,
            span,
            vec![FunctionOperand::Function(left.entity)],
        ),
    })
}

fn apply_conjunction(
    left: Verb,
    operator: Arc<FunctionEntity>,
    right: Item,
) -> Result<Verb> {
    debug_assert_eq!(operator.result_pos, FunctionPartOfSpeech::Conjunction);
    let primitive_id = match &operator.head {
        FunctionHead::PrimitiveConjunction(id) => Some(*id),
        _ => None,
    };
    let mut operands = vec![FunctionOperand::Function(left.entity)];
    let right_end;
    let Item { class, value: right } = right;
    match (class, right) {
        (ParseClass::Noun, ParseValue::Noun(expr, _)) => {
            if matches!(primitive_id, Some(crate::primitive::ConjunctionId::Atop)) {
                return Err(Error::Syntax("atop requires a verb right operand".into()));
            }
            right_end = expr.span.end;
            let value = match expr.kind {
                ExprKind::Literal(value) => value,
                ExprKind::Group(inner) => match inner.kind {
                    ExprKind::Literal(value) => value,
                    _ => {
                        return Err(Error::Unsupported(
                            "non-literal conjunction noun operand".into(),
                        ))
                    }
                },
                _ => {
                    return Err(Error::Unsupported(
                        "non-literal conjunction noun operand".into(),
                    ))
                }
            };
            if matches!(primitive_id, Some(crate::primitive::ConjunctionId::Rank)) {
                if value.is_empty() || value.len() > 3 {
                    return Err(Error::Length);
                }
                for i in 0..value.len() {
                    value.int_at(i)?;
                }
            }
            operands.push(FunctionOperand::Noun {
                span: expr.span,
                value,
            });
        }
        (ParseClass::Verb, ParseValue::Verb(verb)) => {
            right_end = verb.span.end;
            operands.push(FunctionOperand::Function(verb.entity));
        }
        _ => return Err(Error::Syntax("invalid conjunction right operand".into())),
    }
    let span = left.span.start..right_end;
    Ok(Verb {
        span: span.clone(),
        target: VerbTarget::Derived,
        entity: FunctionEntity::derived(
            operator.head.clone(),
            FunctionPartOfSpeech::Verb,
            span,
            operands,
        ),
    })
}

/// Apply the jsource parser's function-construction rows for the subset currently
/// represented by this frontend: AVN ADV (row 3) and AVN CONJ AVN (row 4).
/// We select the rightmost reducible phrase to match the parser's right-to-left
/// queue/stack discipline. Hook/fork reduction is performed separately below.
fn reduce_modifier_applications(mut items: Vec<Item>) -> Result<Vec<Item>> {
    loop {
        let mut reduced = false;

        if items.len() >= 2 {
            for i in (0..items.len() - 1).rev() {
                let edge = if i == 0 {
                    ParseClass::Mark
                } else {
                    items[i - 1].class
                };
                let after = items
                    .get(i + 2)
                    .map_or(ParseClass::Mark, |item| item.class);
                if items[i].class == ParseClass::Verb
                    && match_parse_row([
                        edge,
                        items[i].class,
                        items[i + 1].class,
                        after,
                    ]) == Some(ParseRow::Adverb)
                {
                    let pair: Vec<_> = items.drain(i..i + 2).collect();
                    let mut pair = pair.into_iter();
                    let left = pair.next().unwrap().into_verb().expect("verb class");
                    let operator = pair
                        .next()
                        .unwrap()
                        .into_function()
                        .expect("adverb class");
                    let span = left.span.start..operator.span.end;
                    items.insert(
                        i,
                        Item::verb(
                            apply_adverb(left, operator).map_err(|error| error.at(span))?,
                        ),
                    );
                    reduced = true;
                    break;
                }
            }
        }
        if reduced {
            continue;
        }

        if items.len() >= 3 {
            for i in (0..items.len() - 2).rev() {
                let edge = if i == 0 {
                    ParseClass::Mark
                } else {
                    items[i - 1].class
                };
                if items[i].class == ParseClass::Verb
                    && match_parse_row([
                        edge,
                        items[i].class,
                        items[i + 1].class,
                        items[i + 2].class,
                    ]) == Some(ParseRow::Conjunction)
                {
                    let triple: Vec<_> = items.drain(i..i + 3).collect();
                    let mut triple = triple.into_iter();
                    let left = triple.next().unwrap().into_verb().expect("verb class");
                    let operator = triple
                        .next()
                        .unwrap()
                        .into_function()
                        .expect("conjunction class");
                    let right = triple.next().unwrap();
                    let span = left.span.start..right.span().end;
                    items.insert(
                        i,
                        Item::verb(
                            apply_conjunction(left, operator, right)
                                .map_err(|error| error.at(span))?,
                        ),
                    );
                    reduced = true;
                    break;
                }
            }
        }

        if !reduced {
            return Ok(items);
        }
    }
}

fn collapse_verb_trains(items: Vec<Item>) -> Result<Vec<Item>> {
    // A pure function phrase (for example `+/ % #` inside parentheses or on
    // an assignment RHS) is a train. Do not collapse verb runs embedded in a
    // mixed noun sentence yet: jsource's parse table may execute a V N / N V N
    // fragment before hook/fork construction (e.g. `1 + - 2`).
    if items.len() > 1 && items.iter().all(|item| item.class == ParseClass::Verb) {
        let verbs = items
            .into_iter()
            .map(|item| item.into_verb().expect("verb class"))
            .collect();
        Ok(vec![Item::verb(make_verb_train(verbs)?)])
    } else {
        Ok(items)
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParseClass {
    Noun,
    Verb,
    Adverb,
    Conjunction,
    Name,
    Assignment,
    LParen,
    RParen,
    Mark,
}

impl From<FunctionPartOfSpeech> for ParseClass {
    fn from(pos: FunctionPartOfSpeech) -> Self {
        match pos {
            FunctionPartOfSpeech::Verb => Self::Verb,
            FunctionPartOfSpeech::Adverb => Self::Adverb,
            FunctionPartOfSpeech::Conjunction => Self::Conjunction,
        }
    }
}

impl From<crate::primitive::PrimitivePartOfSpeech> for FunctionPartOfSpeech {
    fn from(pos: crate::primitive::PrimitivePartOfSpeech) -> Self {
        match pos {
            crate::primitive::PrimitivePartOfSpeech::Verb => Self::Verb,
            crate::primitive::PrimitivePartOfSpeech::Adverb => Self::Adverb,
            crate::primitive::PrimitivePartOfSpeech::Conjunction => Self::Conjunction,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ParseRow {
    MonadEdge = 0,
    MonadVVN = 1,
    DyadNVN = 2,
    Adverb = 3,
    Conjunction = 4,
    Fork = 5,
    Hook = 6,
    Assignment = 7,
    Parenthesis = 8,
}

fn is_avn(class: ParseClass) -> bool {
    matches!(class, ParseClass::Adverb | ParseClass::Verb | ParseClass::Noun)
}

fn is_cavn(class: ParseClass) -> bool {
    matches!(
        class,
        ParseClass::Conjunction | ParseClass::Adverb | ParseClass::Verb | ParseClass::Noun
    )
}

fn is_edge(class: ParseClass) -> bool {
    matches!(class, ParseClass::Mark | ParseClass::Assignment | ParseClass::LParen)
}

fn is_edge_or_avn(class: ParseClass) -> bool {
    is_edge(class) || is_avn(class)
}

/// Declarative equivalent of the pinned jsource p.c cases table.
///
/// The first matching row is the parser precedence. Actions are migrated onto
/// this table incrementally; the table itself is already the single source for
/// row eligibility.
fn match_parse_row(classes: [ParseClass; 4]) -> Option<ParseRow> {
    use ParseClass::*;
    let [a, b, c, d] = classes;
    [
        (
            ParseRow::MonadEdge,
            is_edge(a) && b == Verb && c == Noun,
        ),
        (
            ParseRow::MonadVVN,
            is_edge_or_avn(a) && b == Verb && c == Verb && d == Noun,
        ),
        (
            ParseRow::DyadNVN,
            is_edge_or_avn(a) && b == Noun && c == Verb && d == Noun,
        ),
        (
            ParseRow::Adverb,
            is_edge_or_avn(a) && matches!(b, Verb | Noun) && c == Adverb,
        ),
        (
            ParseRow::Conjunction,
            is_edge_or_avn(a)
                && matches!(b, Verb | Noun)
                && c == Conjunction
                && matches!(d, Verb | Noun),
        ),
        (
            ParseRow::Fork,
            is_edge_or_avn(a) && matches!(b, Verb | Noun) && c == Verb && d == Verb,
        ),
        (
            ParseRow::Hook,
            is_edge(a) && is_cavn(b) && is_cavn(c),
        ),
        (
            ParseRow::Assignment,
            matches!(a, Name | Noun) && b == Assignment && is_cavn(c),
        ),
        (
            ParseRow::Parenthesis,
            a == LParen && is_cavn(b) && c == RParen,
        ),
    ]
    .into_iter()
    .find_map(|(row, matched)| matched.then_some(row))
}

enum ParseValue {
    Noun(Expr, usize),
    Verb(Verb),
    Function(Arc<FunctionEntity>),
}

struct Item {
    class: ParseClass,
    value: ParseValue,
}

impl Item {
    fn span(&self) -> std::ops::Range<usize> {
        match &self.value {
            ParseValue::Noun(expr, _) => expr.span.clone(),
            ParseValue::Verb(verb) => verb.span.clone(),
            ParseValue::Function(entity) => entity.span.clone(),
        }
    }

    fn noun(expr: Expr, height: usize) -> Self {
        Self {
            class: ParseClass::Noun,
            value: ParseValue::Noun(expr, height),
        }
    }

    fn verb(verb: Verb) -> Self {
        debug_assert_eq!(verb.entity.result_pos, FunctionPartOfSpeech::Verb);
        Self {
            class: ParseClass::Verb,
            value: ParseValue::Verb(verb),
        }
    }

    fn function(entity: Arc<FunctionEntity>) -> Self {
        Self {
            class: entity.result_pos.into(),
            value: ParseValue::Function(entity),
        }
    }

    fn into_noun(self) -> Option<(Expr, usize)> {
        match self.value {
            ParseValue::Noun(expr, height) if self.class == ParseClass::Noun => {
                Some((expr, height))
            }
            _ => None,
        }
    }

    fn into_verb(self) -> Option<Verb> {
        match self.value {
            ParseValue::Verb(verb) if self.class == ParseClass::Verb => Some(verb),
            _ => None,
        }
    }

    fn into_function(self) -> Option<Arc<FunctionEntity>> {
        match self.value {
            ParseValue::Function(entity)
                if matches!(self.class, ParseClass::Adverb | ParseClass::Conjunction) =>
            {
                Some(entity)
            }
            _ => None,
        }
    }
}

/// Parse without reading bindings, changing state, or invoking any kernels.
pub fn parse(source: &str) -> Result<Program> {
    parse_with(source, None, false).map_err(Error::into_unlocated)
}

/// Diagnostic frontend entry point. It uses the same parser semantics as
/// `parse` but retains source spans on errors for compiler/interpreter/JIT UI.
pub fn parse_diagnostic(source: &str) -> Result<Program> {
    parse_with(source, None, false)
}

#[derive(Clone, Debug)]
pub(crate) enum ParserNameBinding {
    Noun(Value),
    Function(FunctionPartOfSpeech),
}

pub(crate) fn parse_runtime(
    source: &str,
    lookup: &dyn Fn(&str) -> Option<ParserNameBinding>,
) -> Result<Program> {
    parse_with(source, Some(lookup), true)
}

pub(crate) fn parse_analysis(
    source: &str,
    lookup: &dyn Fn(&str) -> Option<ParserNameBinding>,
) -> Result<Program> {
    parse_with(source, Some(lookup), false)
}

type NameLookup<'a> = Option<&'a dyn Fn(&str) -> Option<ParserNameBinding>>;
fn parse_with(source: &str, lookup: NameLookup<'_>, snapshot: bool) -> Result<Program> {
    let mut queue = enqueue(source)?;
    let mut assignment_span = None;
    let mut assignment = None;
    let expression = if queue.is_empty() {
        None
    } else {
        let expr = if queue.len() > 1 && queue[1].class == EnqueueClass::Assignment {
            if queue[0].class != EnqueueClass::Name || queue[0].flags.lookup_name {
                return Err(
                    Error::Syntax("assignment target".into()).with_context(
                        ErrorContext::phase(DiagnosticPhase::Parse)
                            .with_span(queue[0].span.clone())
                            .with_blame_word(queue[0].word_index),
                    ),
                );
            }
            let EnqueuedPayload::Name(name) = &queue[0].payload else {
                return Err(Error::Syntax("assignment target".into()));
            };
            assignment = Some((*name).to_owned());
            assignment_span = Some(queue[0].span.clone());
            &mut queue[2..]
        } else {
            queue.as_mut_slice()
        };
        let mut pos = 0;
        let (result, _) = expression(expr, &mut pos, false, 0, lookup, snapshot)
            .map_err(|error| {
                let fallback = expr
                    .get(pos)
                    .or_else(|| expr.last());
                let fallback_span = fallback
                    .map(|word| word.span.clone())
                    .unwrap_or(source.len()..source.len());
                let mut context =
                    ErrorContext::phase(DiagnosticPhase::Parse).with_span(fallback_span);
                if let Some(word) = fallback {
                    context = context.with_blame_word(word.word_index);
                }
                error.with_context(context)
            })?;
        if pos != expr.len() {
            let fallback = expr.get(pos).or_else(|| expr.last());
            let span = fallback
                .map(|word| word.span.clone())
                .unwrap_or(source.len()..source.len());
            let mut context = ErrorContext::phase(DiagnosticPhase::Parse).with_span(span);
            if let Some(word) = fallback {
                context = context.with_blame_word(word.word_index);
            }
            return Err(Error::Syntax("trailing tokens".into()).with_context(context));
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
    tokens: &mut [EnqueuedWord<'_>],
    pos: &mut usize,
    nested: bool,
    depth: usize,
    lookup: NameLookup<'_>,
    snapshot: bool,
) -> Result<(Expr, usize)> {
    if depth > MAX_EXPR_DEPTH {
        return Err(Error::Limit);
    }
    let mut items = Vec::new();
    while *pos < tokens.len() {
        match &tokens[*pos].payload {
            EnqueuedPayload::Close => {
                if nested {
                    break;
                } else {
                    return Err(Error::Syntax("unexpected )".into()).at(tokens[*pos].span.clone()));
                }
            }
            EnqueuedPayload::Open => {
                let start = tokens[*pos].span.start;
                *pos += 1;
                let (v, height) = expression(tokens, pos, true, depth + 1, lookup, snapshot)?;
                let height = checked_height(height)?;
                if !tokens.get(*pos).is_some_and(|word| matches!(word.payload, EnqueuedPayload::Close)) {
                    return Err(Error::Syntax("missing )".into()).at(start..start + 1));
                }
                let v = Expr {
                    span: start..tokens[*pos].span.end,
                    kind: ExprKind::Group(Box::new(v)),
                };
                *pos += 1;
                if let ExprKind::Group(inner) = &v.kind {
                    if let ExprKind::VerbValue(verb) = &inner.kind {
                        let mut verb = verb.clone();
                        verb.span = v.span;
                        items.push(Item::verb(verb));
                        continue;
                    }
                }
                items.push(Item::noun(v, height));
            }
            EnqueuedPayload::Scalar(v) => {
                items.push(Item::noun(
                    Expr {
                        span: tokens[*pos].span.clone(),
                        kind: ExprKind::Literal(
                        v.clone()
                            .into_value()
                            .map_err(|error| error.at(tokens[*pos].span.clone()))?,
                    ),
                    },
                    0,
                ));
                *pos += 1;
            }
            EnqueuedPayload::Noun(_) => {
                let EnqueuedPayload::Noun(v) = std::mem::replace(&mut tokens[*pos].payload, EnqueuedPayload::Open) else {
                    unreachable!()
                };
                items.push(Item::noun(
                    Expr {
                        span: tokens[*pos].span.clone(),
                        kind: ExprKind::Literal(*v),
                    },
                    0,
                ));
                *pos += 1;
            }
            EnqueuedPayload::Verb(_) | EnqueuedPayload::Name(_) => {
                if let EnqueuedPayload::Name(n) = &tokens[*pos].payload {
                    if !tokens[*pos].flags.lookup_name {
                        return Err(
                            Error::Syntax("assignment-target name in expression".into())
                                .at(tokens[*pos].span.clone()),
                        );
                    }
                    match lookup.and_then(|lookup| lookup(n)) {
                        Some(ParserNameBinding::Noun(value)) => {
                            let kind = if snapshot {
                                ExprKind::Literal(value)
                            } else {
                                ExprKind::ReadName((*n).to_owned())
                            };
                            items.push(Item::noun(
                                Expr {
                                    span: tokens[*pos].span.clone(),
                                    kind,
                                },
                                0,
                            ));
                            *pos += 1;
                            continue;
                        }
                        Some(ParserNameBinding::Function(result_pos)) => {
                            let span = tokens[*pos].span.clone();
                            let entity =
                                FunctionEntity::name_ref((*n).to_owned(), result_pos, span.clone());
                            *pos += 1;
                            if result_pos == FunctionPartOfSpeech::Verb {
                                items.push(Item::verb(Verb {
                                    span,
                                    target: VerbTarget::Named((*n).to_owned()),
                                    entity,
                                }));
                            } else {
                                items.push(Item::function(entity));
                            }
                            continue;
                        }
                        None if lookup.is_none() => {
                            items.push(Item::noun(
                                Expr {
                                    span: tokens[*pos].span.clone(),
                                    kind: ExprKind::ReadName((*n).to_owned()),
                                },
                                0,
                            ));
                            *pos += 1;
                            continue;
                        }
                        None => {
                            // jsource creates a late verb reference for an
                            // unresolved ordinary lookup name.
                        }
                    }
                }
                let target = match &tokens[*pos].payload {
                    EnqueuedPayload::Verb(id) => VerbTarget::Primitive(*id),
                    EnqueuedPayload::Name(n) => VerbTarget::Named((*n).to_owned()),
                    _ => unreachable!(),
                };
                let verb_span = tokens[*pos].span.clone();
                let entity = match &target {
                    VerbTarget::Primitive(id) => FunctionEntity::primitive(*id, verb_span.clone()),
                    VerbTarget::Named(name) => FunctionEntity::name_ref(
                        name.clone(),
                        FunctionPartOfSpeech::Verb,
                        verb_span.clone(),
                    ),
                    VerbTarget::Derived => unreachable!("source token is not a derived target"),
                };
                let verb = Verb {
                    span: verb_span,
                    target,
                    entity,
                };
                *pos += 1;
                items.push(Item::verb(verb));
            }
            EnqueuedPayload::Adverb(id) => {
                items.push(Item::function(FunctionEntity::primitive_adverb(
                    *id,
                    tokens[*pos].span.clone(),
                )));
                *pos += 1;
            }
            EnqueuedPayload::Conjunction(id) => {
                items.push(Item::function(FunctionEntity::primitive_conjunction(
                    *id,
                    tokens[*pos].span.clone(),
                )));
                *pos += 1;
            }
            _ => {
                return Err(
                    Error::Unsupported("assignment/modifier in expression".into())
                        .at(tokens[*pos].span.clone()),
                );
            }
        }
    }
    let items = reduce_modifier_applications(items)?;
    let mut items = collapse_verb_trains(items)?;

    if items.len() == 1
        && items
            .first()
            .is_some_and(|item| item.class == ParseClass::Verb)
    {
        let verb = items
            .pop()
            .and_then(Item::into_verb)
            .expect("verb class");
        return Ok((
            Expr {
                span: verb.span.clone(),
                kind: ExprKind::VerbValue(verb),
            },
            0,
        ));
    }
    let rhs_error_span = items
        .last()
        .map(Item::span)
        .unwrap_or_else(|| tokens.last().map(|word| word.span.clone()).unwrap_or(0..0));
    let Some((mut rhs, mut height)) = items.pop().and_then(Item::into_noun) else {
        return Err(Error::Syntax("expected right argument".into()).at(rhs_error_span));
    };
    while let Some(item) = items.pop() {
        let item_span = item.span();
        let Some(v) = item.into_verb() else {
            return Err(
                Error::Syntax("unreduced function modifier or adjacent nouns".into())
                    .at(item_span),
            );
        };
        if items
            .last()
            .is_some_and(|item| item.class == ParseClass::Noun)
        {
            let (lhs, left_height) = items
                .pop()
                .and_then(Item::into_noun)
                .expect("noun class");
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
        if let Some(verb) = verb {
            let mut functions = vec![verb.entity.as_ref()];
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


#[cfg(test)]
mod parser_table_tests {
    use super::{match_parse_row, ParseClass::*, ParseRow};

    #[test]
    fn pinned_jsource_rows_and_precedence_are_exact() {
        let cases = [
            ([Mark, Verb, Noun, Noun], ParseRow::MonadEdge),
            ([Mark, Verb, Verb, Noun], ParseRow::MonadVVN),
            ([Mark, Noun, Verb, Noun], ParseRow::DyadNVN),
            ([Mark, Verb, Adverb, Noun], ParseRow::Adverb),
            ([Mark, Verb, Conjunction, Noun], ParseRow::Conjunction),
            ([Mark, Verb, Verb, Verb], ParseRow::Fork),
            ([Mark, Verb, Noun, Verb], ParseRow::Hook),
            ([Name, Assignment, Verb, Noun], ParseRow::Assignment),
            ([LParen, Verb, RParen, Noun], ParseRow::Parenthesis),
        ];
        for (classes, expected) in cases {
            assert_eq!(match_parse_row(classes), Some(expected), "{classes:?}");
        }
    }

    #[test]
    fn fourth_stack_class_is_part_of_row_eligibility() {
        assert_ne!(
            match_parse_row([Mark, Verb, Verb, Verb]),
            Some(ParseRow::MonadVVN)
        );
        assert_ne!(
            match_parse_row([Mark, Noun, Verb, Verb]),
            Some(ParseRow::DyadNVN)
        );
        assert_ne!(
            match_parse_row([Mark, Verb, Conjunction, Adverb]),
            Some(ParseRow::Conjunction)
        );
    }
}

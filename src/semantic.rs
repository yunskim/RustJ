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
    let right_span = right.span();
    let right_end = right_span.end;
    let Item { class, value: right, .. } = right;
    match (class, right) {
        (ParseClass::Noun, ParseValue::Noun(expr, _)) => {
            if matches!(primitive_id, Some(crate::primitive::ConjunctionId::Atop)) {
                return Err(Error::Syntax("atop requires a verb right operand".into()));
            }
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
#[derive(Clone, Debug)]
struct PendingAssignment {
    name: String,
    span: std::ops::Range<usize>,
}

fn reduce_parse_stack_subset(
    mut queue: Vec<Item>,
) -> Result<(Vec<Item>, Option<PendingAssignment>)> {
    let mut stack = Vec::<Item>::new();
    let mut assignment = None;

    while let Some(item) = queue.pop() {
        stack.insert(0, item);
        reduce_stack_prefix(&mut stack, &mut assignment, queue.is_empty())?;
    }

    // jsource realizes the virtual FRONT MARK only after the queue is empty.
    stack.insert(0, Item::mark(0));
    reduce_stack_prefix(&mut stack, &mut assignment, true)?;

    if stack.first().is_some_and(|item| item.class == ParseClass::Mark) {
        stack.remove(0);
    }
    Ok((stack, assignment))
}

fn stack_prefix_classes(stack: &[Item]) -> [ParseClass; 4] {
    let class = |index: usize| {
        stack
            .get(index)
            .map_or(ParseClass::Mark, |item| item.class)
    };
    [class(0), class(1), class(2), class(3)]
}

fn reduce_stack_prefix(
    stack: &mut Vec<Item>,
    assignment: &mut Option<PendingAssignment>,
    queue_exhausted: bool,
) -> Result<()> {
    loop {
        let Some(row) = match_parse_row(stack_prefix_classes(stack)) else {
            return Ok(());
        };

        let reduced = match row {
            ParseRow::MonadEdge => {
                let mut phrase: Vec<_> = stack.drain(1..3).collect();
                let verb = phrase.remove(0).into_verb().expect("row 0 verb");
                let (argument, height) = phrase.remove(0).into_noun().expect("row 0 noun");
                let expr = Expr {
                    span: verb.span.start..argument.span.end,
                    kind: ExprKind::Monad {
                        verb,
                        argument: Box::new(argument),
                    },
                };
                stack.insert(1, Item::noun(expr, checked_height(height)?));
                true
            }
            ParseRow::MonadVVN => {
                let mut phrase: Vec<_> = stack.drain(2..4).collect();
                let verb = phrase.remove(0).into_verb().expect("row 1 verb");
                let (argument, height) = phrase.remove(0).into_noun().expect("row 1 noun");
                let expr = Expr {
                    span: verb.span.start..argument.span.end,
                    kind: ExprKind::Monad {
                        verb,
                        argument: Box::new(argument),
                    },
                };
                stack.insert(2, Item::noun(expr, checked_height(height)?));
                true
            }
            ParseRow::DyadNVN => {
                let mut phrase: Vec<_> = stack.drain(1..4).collect();
                let (left, left_height) = phrase.remove(0).into_noun().expect("row 2 left noun");
                let verb = phrase.remove(0).into_verb().expect("row 2 verb");
                let (right, right_height) = phrase.remove(0).into_noun().expect("row 2 right noun");
                let expr = Expr {
                    span: left.span.start..right.span.end,
                    kind: ExprKind::Dyad {
                        verb,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                };
                stack.insert(
                    1,
                    Item::noun(expr, checked_height(left_height.max(right_height))?),
                );
                true
            }
            ParseRow::Adverb => {
                if stack.get(1).is_some_and(|item| item.class == ParseClass::Verb) {
                    let mut phrase: Vec<_> = stack.drain(1..3).collect();
                    let left = phrase.remove(0).into_verb().expect("row 3 verb");
                    let operator = phrase
                        .remove(0)
                        .into_function()
                        .expect("row 3 adverb");
                    let span = left.span.start..operator.span.end;
                    stack.insert(
                        1,
                        Item::verb(
                            apply_adverb(left, operator)
                                .map_err(|error| error.at(span))?,
                        ),
                    );
                    true
                } else {
                    false
                }
            }
            ParseRow::Conjunction => {
                if stack.get(1).is_some_and(|item| item.class == ParseClass::Verb) {
                    let mut phrase: Vec<_> = stack.drain(1..4).collect();
                    let left = phrase.remove(0).into_verb().expect("row 4 left verb");
                    let operator = phrase
                        .remove(0)
                        .into_function()
                        .expect("row 4 conjunction");
                    let right = phrase.remove(0);
                    let span = left.span.start..right.span().end;
                    stack.insert(
                        1,
                        Item::verb(
                            apply_conjunction(left, operator, right)
                                .map_err(|error| error.at(span))?,
                        ),
                    );
                    true
                } else {
                    false
                }
            }
            ParseRow::Fork => {
                if stack.get(1).is_some_and(|item| item.class == ParseClass::Verb)
                    && stack.get(2).is_some_and(|item| item.class == ParseClass::Verb)
                    && stack.get(3).is_some_and(|item| item.class == ParseClass::Verb)
                {
                    let mut phrase: Vec<_> = stack.drain(1..4).collect();
                    let f = phrase.remove(0).into_verb().expect("row 5 f");
                    let g = phrase.remove(0).into_verb().expect("row 5 g");
                    let h = phrase.remove(0).into_verb().expect("row 5 h");
                    stack.insert(1, Item::verb(train_fork(f, g, h)));
                    true
                } else {
                    false
                }
            }
            ParseRow::Hook => {
                if stack.get(1).is_some_and(|item| item.class == ParseClass::Verb)
                    && stack.get(2).is_some_and(|item| item.class == ParseClass::Verb)
                {
                    let mut phrase: Vec<_> = stack.drain(1..3).collect();
                    let f = phrase.remove(0).into_verb().expect("row 6 f");
                    let g = phrase.remove(0).into_verb().expect("row 6 g");
                    stack.insert(1, Item::verb(train_hook(f, g)));
                    true
                } else {
                    false
                }
            }
            ParseRow::Assignment => {
                if !queue_exhausted {
                    return Err(Error::Unsupported(
                        "non-final assignment requires runtime semantic parsing".into(),
                    ));
                }
                if assignment.is_some() {
                    return Err(Error::Unsupported(
                        "multiple assignments in one sentence".into(),
                    ));
                }
                if stack.first().is_some_and(|item| item.class == ParseClass::Noun) {
                    return Err(Error::Unsupported(
                        "noun/multiple assignment target".into(),
                    ));
                }

                let mut phrase: Vec<_> = stack.drain(0..3).collect();
                let target = phrase.remove(0);
                let _copula = phrase.remove(0);
                let value = phrase.remove(0);
                let ParseValue::NameTarget { name, span } = target.value else {
                    return Err(Error::Syntax("row 7 requires a name target".into()));
                };
                if matches!(value.class, ParseClass::Adverb | ParseClass::Conjunction) {
                    return Err(Error::Unsupported(
                        "modifier assignment is not yet executable".into(),
                    ));
                }
                *assignment = Some(PendingAssignment { name, span });
                stack.insert(0, value);
                true
            },
            ParseRow::Parenthesis => {
                let mut phrase: Vec<_> = stack.drain(0..3).collect();
                let left = phrase.remove(0);
                let value = phrase.remove(0);
                let right = phrase.remove(0);
                let group_span = left.span().start..right.span().end;

                let grouped = match value.value {
                    ParseValue::Noun(expr, height) => Item::noun(
                        Expr {
                            span: group_span.clone(),
                            kind: ExprKind::Group(Box::new(expr)),
                        },
                        checked_height(height)?,
                    )
                    .with_span(group_span),
                    ParseValue::Verb(mut verb) => {
                        verb.span = group_span.clone();
                        Item::verb(verb).with_span(group_span)
                    }
                    ParseValue::Function(entity) => {
                        Item::function(entity).with_span(group_span)
                    }
                    ParseValue::NameTarget { .. } | ParseValue::Control { .. } => {
                        return Err(
                            Error::Syntax("invalid parenthesized parser control".into())
                                .at(group_span),
                        );
                    }
                };
                stack.insert(0, grouped);
                true
            },
        };

        // The table matched a jsource row whose semantic action has not yet
        // migrated into this subset engine. Leave it for the existing outer
        // assignment/grouping path rather than applying a different row.
        if !reduced {
            return Ok(());
        }
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

#[derive(Clone)]
enum ParseValue {
    Noun(Expr, usize),
    Verb(Verb),
    Function(Arc<FunctionEntity>),
    NameTarget {
        name: String,
        span: std::ops::Range<usize>,
    },
    Control {
        span: std::ops::Range<usize>,
    },
}

#[derive(Clone)]
struct Item {
    class: ParseClass,
    value: ParseValue,
    span_override: Option<std::ops::Range<usize>>,
}

impl Item {
    fn span(&self) -> std::ops::Range<usize> {
        if let Some(span) = &self.span_override {
            return span.clone();
        }
        match &self.value {
            ParseValue::Noun(expr, _) => expr.span.clone(),
            ParseValue::Verb(verb) => verb.span.clone(),
            ParseValue::Function(entity) => entity.span.clone(),
            ParseValue::NameTarget { span, .. } => span.clone(),
            ParseValue::Control { span } => span.clone(),
        }
    }

    fn with_span(mut self, span: std::ops::Range<usize>) -> Self {
        self.span_override = Some(span);
        self
    }

    fn mark(at: usize) -> Self {
        Self {
            class: ParseClass::Mark,
            value: ParseValue::Control { span: at..at },
            span_override: None,
        }
    }

    fn control(class: ParseClass, span: std::ops::Range<usize>) -> Self {
        debug_assert!(matches!(
            class,
            ParseClass::Assignment | ParseClass::LParen | ParseClass::RParen
        ));
        Self {
            class,
            value: ParseValue::Control { span },
            span_override: None,
        }
    }

    fn name_target(name: String, span: std::ops::Range<usize>) -> Self {
        Self {
            class: ParseClass::Name,
            value: ParseValue::NameTarget { name, span },
            span_override: None,
        }
    }

    fn noun(expr: Expr, height: usize) -> Self {
        Self {
            class: ParseClass::Noun,
            value: ParseValue::Noun(expr, height),
            span_override: None,
        }
    }

    fn verb(verb: Verb) -> Self {
        debug_assert_eq!(verb.entity.result_pos, FunctionPartOfSpeech::Verb);
        Self {
            class: ParseClass::Verb,
            value: ParseValue::Verb(verb),
            span_override: None,
        }
    }

    fn function(entity: Arc<FunctionEntity>) -> Self {
        Self {
            class: entity.result_pos.into(),
            value: ParseValue::Function(entity),
            span_override: None,
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
    if queue.is_empty() {
        return Ok(Program {
            source: source.to_owned(),
            assignment: None,
            assignment_span: None,
            expression: None,
        });
    }

    let mut pos = 0;
    let (result, _, pending_assignment) =
        expression(queue.as_mut_slice(), &mut pos, lookup, snapshot).map_err(|error| {
            let fallback = queue.get(pos).or_else(|| queue.last());
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
    if pos != queue.len() {
        let fallback = queue.get(pos).or_else(|| queue.last());
        let span = fallback
            .map(|word| word.span.clone())
            .unwrap_or(source.len()..source.len());
        let mut context = ErrorContext::phase(DiagnosticPhase::Parse).with_span(span);
        if let Some(word) = fallback {
            context = context.with_blame_word(word.word_index);
        }
        return Err(Error::Syntax("trailing tokens".into()).with_context(context));
    }

    let (assignment, assignment_span) = match pending_assignment {
        Some(PendingAssignment { name, span }) => (Some(name), Some(span)),
        None => (None, None),
    };
    Ok(Program {
        source: source.to_owned(),
        assignment,
        assignment_span,
        expression: Some(result),
    })
}
fn expression(
    tokens: &mut [EnqueuedWord<'_>],
    pos: &mut usize,
    lookup: NameLookup<'_>,
    snapshot: bool,
) -> Result<(Expr, usize, Option<PendingAssignment>)> {
    let mut items = Vec::new();
    let mut open_spans = Vec::new();
    while *pos < tokens.len() {
        match &tokens[*pos].payload {
            EnqueuedPayload::Close => {
                if open_spans.pop().is_none() {
                    return Err(
                        Error::Syntax("unexpected )".into()).at(tokens[*pos].span.clone()),
                    );
                }
                items.push(Item::control(
                    ParseClass::RParen,
                    tokens[*pos].span.clone(),
                ));
                *pos += 1;
            }
            EnqueuedPayload::Open => {
                if open_spans.len() >= MAX_EXPR_DEPTH {
                    return Err(Error::Limit);
                }
                open_spans.push(tokens[*pos].span.clone());
                items.push(Item::control(
                    ParseClass::LParen,
                    tokens[*pos].span.clone(),
                ));
                *pos += 1;
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
                        items.push(Item::name_target(
                            (*n).to_owned(),
                            tokens[*pos].span.clone(),
                        ));
                        *pos += 1;
                        continue;
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
            EnqueuedPayload::Assign => {
                items.push(Item::control(
                    ParseClass::Assignment,
                    tokens[*pos].span.clone(),
                ));
                *pos += 1;
            }
        }
    }
    if let Some(span) = open_spans.last() {
        return Err(Error::Syntax("missing )".into()).at(span.clone()));
    }

    let (mut items, assignment) = reduce_parse_stack_subset(items)?;

    if items.len() != 1 {
        let span = items
            .first()
            .map(Item::span)
            .unwrap_or_else(|| tokens.last().map(|word| word.span.clone()).unwrap_or(0..0));
        return Err(
            Error::Syntax("unreduced parser stack after rows 0-6".into()).at(span),
        );
    }

    let item = items.pop().expect("one reduced parser item");
    let span = item.span();
    match item.value {
        ParseValue::Noun(expr, height) => Ok((expr, height, assignment)),
        ParseValue::Verb(verb) => Ok((
            Expr {
                span: verb.span.clone(),
                kind: ExprKind::VerbValue(verb),
            },
            0,
            assignment,
        )),
        ParseValue::Function(_) => Err(
            Error::Syntax("unapplied function modifier".into()).at(span),
        ),
        ParseValue::NameTarget { .. } | ParseValue::Control { .. } => Err(
            Error::Syntax("unexpected parser control result".into()).at(span),
        ),
    }
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
            ([Mark, Conjunction, Verb, Noun], ParseRow::Hook),
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

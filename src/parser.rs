//! J parser queue/stack reductions and parser-time construction.
//! Consumes typed enqueue records; produces target-independent Semantic IR.
use crate::parser_capture::{CaptureEvent, OccurrenceId, ParseCapture};
use crate::{
    Error, Result, Value,
    enqueuer::{EnqueueFlags, EnqueuedPayload, EnqueuedWord, enqueue},
    error::{DiagnosticPhase, ErrorContext},
    semantic::{
        Expr, ExprKind, FunctionEntity, FunctionHead, FunctionOperand, FunctionPartOfSpeech,
        JEntity, MAX_EXPR_DEPTH, Program, Verb, VerbTarget, rank_noun_contract,
    },
};
use std::sync::Arc;

/// Execution-free source framing before the ordinary enqueue/row pipeline.
pub use crate::definition_input::{DefinitionInput, InputFrame, frame as frame_definition_input};

/// Original enqueue-word coverage and inherited diagnostic token (zero-based).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseProvenance {
    pub word_range: std::ops::Range<usize>,
    pub blame_word_index: usize,
}

/// Completed grammar action; records source structure, never execution choices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseReduction {
    pub row: ParseRow,
    pub inputs: Vec<ParseProvenance>,
    pub result: ParseProvenance,
    pub result_class: ParseClass,
    pub span: std::ops::Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssignmentSource {
    pub target: ParseProvenance,
    pub copula: ParseProvenance,
    pub flags: EnqueueFlags,
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

/// Parentheses are parser boundaries, not a change to a completed noun's value.
/// Computed operands still require the shared runtime semantic parse action.
fn completed_noun(mut expr: Expr, context: &str) -> Result<Value> {
    loop {
        match expr.kind {
            ExprKind::Literal(value) => return Ok(value),
            ExprKind::Group(inner) => expr = *inner,
            _ => {
                return Err(Error::Unsupported(format!(
                    "{context} requires semantic parser execution"
                )));
            }
        }
    }
}

fn train_noun_fork(noun: Expr, g: Verb, h: Verb) -> Result<Verb> {
    let noun_span = noun.span.clone();
    let value = completed_noun(noun, "runtime-dependent noun-left fork")?;
    let span = noun_span.start..h.span.end;
    Ok(Verb {
        span: span.clone(),
        target: VerbTarget::Derived,
        entity: FunctionEntity::derived(
            FunctionHead::Fork,
            FunctionPartOfSpeech::Verb,
            span,
            vec![
                FunctionOperand::Noun {
                    value,
                    span: noun_span,
                },
                FunctionOperand::Function(g.entity),
                FunctionOperand::Function(h.entity),
            ],
        ),
    })
}

fn resolve_modifier(
    operator: Arc<FunctionEntity>,
    span: std::ops::Range<usize>,
    context: &mut ActionContext<'_>,
    row: ParseRow,
) -> Result<Arc<FunctionEntity>> {
    let FunctionHead::NameRef(name) = &operator.head else {
        return Ok(operator);
    };
    let Some(host) = context.host.as_mut() else {
        return Err(Error::Unsupported(format!(
            "static construction needs modifier semantics for {name:?}; POS alone is insufficient"
        ))
        .at(span)
        .with_context(
            ErrorContext::phase(DiagnosticPhase::Parse).with_current_name(name.clone()),
        ));
    };
    let resolved = host.resolve_modifier(name, operator.result_pos)?;
    if let Some(capture) = &mut context.capture {
        for (name, version) in resolved.bindings {
            capture.events.push(CaptureEvent::ModifierResolved {
                binding: crate::parser_capture::ModifierBinding {
                    name,
                    version,
                    expected: operator.result_pos,
                    row,
                    function: resolved.function.clone(),
                    span: span.clone(),
                },
            });
        }
    }
    Ok(resolved.function)
}

/// Read the current construction environment without snapshotting a sentence.
#[derive(Clone, Copy, Default)]
struct ConstructionNames<'a, 'h> {
    lookup: NameLookup<'a>,
    host: Option<&'a std::cell::RefCell<&'h mut dyn RuntimeParserHost>>,
    observations: Option<&'a std::cell::RefCell<Vec<CaptureEvent>>>,
    row: Option<ParseRow>,
}

fn construction_host<'h>(
    host: &'h mut Option<&mut dyn RuntimeParserHost>,
) -> Option<std::cell::RefCell<&'h mut dyn RuntimeParserHost>> {
    host.as_mut()
        .map(|host| std::cell::RefCell::new(&mut **host as &mut dyn RuntimeParserHost))
}

impl ConstructionNames<'_, '_> {
    fn resolve_modifier(
        self,
        operator: Arc<FunctionEntity>,
        span: std::ops::Range<usize>,
    ) -> Result<Arc<FunctionEntity>> {
        let FunctionHead::NameRef(name) = &operator.head else {
            return Ok(operator);
        };
        let Some(host) = self.host else {
            return Err(Error::Unsupported(
                "late modifier application requires runtime semantic host".into(),
            )
            .at(span)
            .with_context(
                ErrorContext::phase(DiagnosticPhase::Parse).with_current_name(name.clone()),
            ));
        };
        let resolved = host
            .borrow_mut()
            .resolve_modifier(name, operator.result_pos)
            .map_err(|error| {
                error.at(span.clone()).with_context(
                    ErrorContext::phase(DiagnosticPhase::Parse).with_current_name(name.clone()),
                )
            })?;
        if let Some(observations) = self.observations {
            for (name, version) in resolved.bindings {
                observations
                    .borrow_mut()
                    .push(CaptureEvent::ModifierResolved {
                        binding: crate::parser_capture::ModifierBinding {
                            name,
                            version,
                            expected: operator.result_pos,
                            row: self.row.expect("constructor row"),
                            function: resolved.function.clone(),
                            span: span.clone(),
                        },
                    });
            }
        }
        Ok(resolved.function)
    }
    fn binding(&self, name: &str) -> Result<Option<ParserNameBinding>> {
        if let Some(host) = self.host {
            host.borrow().gerund_binding(name)
        } else if let Some(lookup) = self.lookup {
            Ok(lookup(name))
        } else {
            Err(Error::Unsupported(
                "gerund name construction requires a name environment".into(),
            ))
        }
    }
    fn apply_noun(
        self,
        verb: Verb,
        left: Option<Expr>,
        right: Expr,
        span: std::ops::Range<usize>,
    ) -> Result<Item> {
        let Some(host) = self.host else {
            return Err(Error::Unsupported(
                "constructor noun call requires runtime semantic host".into(),
            ));
        };
        let right_span = right.span.clone();
        let right = completed_noun(right, "constructor right noun")?.into_shared();
        let left = left
            .map(|expr| {
                let span = expr.span.clone();
                completed_noun(expr, "constructor left noun")
                    .map(|value| (value.into_shared(), span))
            })
            .transpose()?;
        let observed = self.observations.map(|_| {
            (
                left.as_ref()
                    .map(|(value, _)| crate::j_graph_ir::GraphFacts::of(value)),
                crate::j_graph_ir::GraphFacts::of(&right),
            )
        });
        let function = verb.entity.clone();
        let right = Box::new(Expr {
            span: right_span,
            kind: ExprKind::Literal(right),
        });
        let expression = Expr {
            span: span.clone(),
            kind: if let Some((value, noun_span)) = left {
                ExprKind::Dyad {
                    verb,
                    left: Box::new(Expr {
                        span: noun_span,
                        kind: ExprKind::Literal(value),
                    }),
                    right,
                }
            } else {
                ExprKind::Monad {
                    verb,
                    argument: right,
                }
            },
        };
        let result = host.borrow_mut().apply(expression);
        if let (Some(observations), Some((left, right))) = (self.observations, observed) {
            let outcome = match &result {
                Ok(value) => crate::parser_capture::ConstructorCallOutcome::Success(
                    crate::j_graph_ir::GraphFacts::of(value),
                ),
                Err(error) => crate::parser_capture::ConstructorCallOutcome::Failure {
                    kind: error.kind().into(),
                    context: error.context().cloned(),
                },
            };
            observations
                .borrow_mut()
                .push(CaptureEvent::ConstructorApply {
                    row: self.row.expect("constructor row"),
                    call: crate::parser_capture::ConstructorCall {
                        function,
                        left,
                        right,
                        span: span.clone(),
                        outcome,
                    },
                });
        }
        result.map(|value| {
            Item::noun(
                Expr {
                    span,
                    kind: ExprKind::Literal(value.into_shared()),
                },
                0,
            )
        })
    }
}

/// The public parser row supplies a N/V operand. Modifier actions may return
/// another modifier, so propagate an Item with its actual result POS.
fn apply_adverb(
    left: Item,
    operator: Arc<FunctionEntity>,
    span: std::ops::Range<usize>,
    depth: usize,
    names: ConstructionNames<'_, '_>,
) -> Result<Item> {
    if depth >= MAX_EXPR_DEPTH {
        return Err(Error::Limit);
    }
    debug_assert_eq!(operator.result_pos, FunctionPartOfSpeech::Adverb);
    let operator = names.resolve_modifier(operator, span.clone())?;
    if matches!(operator.head, FunctionHead::ModifierTrain) {
        if let [first, second] = operator.operands.as_slice() {
            // tcNV: u (C n/v) -> u C n/v; tNVc reverses the binding.
            if let FunctionOperand::Function(conjunction) = first {
                if conjunction.result_pos == FunctionPartOfSpeech::Conjunction
                    && (matches!(second, FunctionOperand::Noun { .. })
                        || matches!(second, FunctionOperand::Function(f) if f.result_pos == FunctionPartOfSpeech::Verb))
                {
                    return apply_conjunction_items(
                        left,
                        conjunction.clone(),
                        modifier_operand(second, span.clone()),
                        span,
                        depth + 1,
                        names,
                    );
                }
            }
            if let FunctionOperand::Function(conjunction) = second {
                if conjunction.result_pos == FunctionPartOfSpeech::Conjunction
                    && (matches!(first, FunctionOperand::Noun { .. })
                        || matches!(first, FunctionOperand::Function(f) if f.result_pos == FunctionPartOfSpeech::Verb))
                {
                    return apply_conjunction_items(
                        modifier_operand(first, span.clone()),
                        conjunction.clone(),
                        left,
                        span,
                        depth + 1,
                        names,
                    );
                }
            }
            // taAV: apply f first, then hook its actual result with g.
            if let [
                FunctionOperand::Function(first),
                FunctionOperand::Function(second),
            ] = operator.operands.as_slice()
            {
                if first.result_pos == FunctionPartOfSpeech::Adverb {
                    let original = share_modifier_input(left);
                    let result = apply_adverb(
                        original.clone(),
                        first.clone(),
                        span.clone(),
                        depth + 1,
                        names,
                    )?;
                    // A C is tac, not taAV: C must receive both t and the
                    // original input. Never approximate it as a two-item hook.
                    if second.result_pos == FunctionPartOfSpeech::Conjunction {
                        return construct_modifier_trident(
                            result,
                            modifier_operand(&operator.operands[1], span.clone()),
                            original,
                            span,
                            depth + 1,
                            names,
                        );
                    }
                    let right = modifier_operand(&operator.operands[1], span.clone());
                    return construct_modifier_bident(result, right, span, depth + 1, names);
                }
            }
        }
        if operator.operands.len() == 3 {
            return apply_modifier_trident(left, None, &operator.operands, span, depth + 1, names);
        }
        return Err(Error::Unsupported(
            "derived modifier application semantics".into(),
        ));
    }
    if !operator.is_primitive_modifier() {
        return Err(Error::Unsupported(
            "modifier child identity requires resolution".into(),
        ));
    }
    if matches!(left.class, ParseClass::Noun)
        && matches!(
            operator.head,
            FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::PrefixInfix)
        )
    {
        let (noun, _) = left.into_noun().unwrap();
        let noun_span = noun.span.clone();
        let value = completed_noun(noun, "runtime-dependent prefix gerund operand")?;
        let decoded = audit_gerund(&value, noun_span.clone(), depth + 1, names)?;
        return Ok(Item::verb(Verb {
            span: span.clone(),
            target: VerbTarget::Derived,
            entity: FunctionEntity::with_decoded_gerund(
                FunctionEntity::derived(
                    operator.head.clone(),
                    FunctionPartOfSpeech::Verb,
                    span,
                    vec![FunctionOperand::Noun {
                        value: value.into_shared(),
                        span: noun_span,
                    }],
                ),
                Some(decoded),
            ),
        }));
    }
    let Some(left) = left.into_verb() else {
        return Err(match operator.head {
            FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert) => Error::Domain,
            _ => Error::Unsupported(
                "noun adverb construction requires semantic parser execution".into(),
            ),
        });
    };
    Ok(Item::verb(Verb {
        span: span.clone(),
        target: VerbTarget::Derived,
        entity: FunctionEntity::derived(
            operator.head.clone(),
            FunctionPartOfSpeech::Verb,
            span,
            vec![FunctionOperand::Function(left.entity)],
        ),
    }))
}

/// cg.c::jtfxeachv(1): decode in element order, then require actual Verb POS.
/// Retain the J-visible gerund noun rather than execution-only fgh auxiliaries.
fn audit_gerund(
    value: &Value,
    span: std::ops::Range<usize>,
    depth: usize,
    names: ConstructionNames<'_, '_>,
) -> Result<Vec<Arc<FunctionEntity>>> {
    if depth >= MAX_EXPR_DEPTH {
        return Err(Error::Limit);
    }
    if value.shape.len() > 1 {
        return Err(Error::Rank);
    }
    if value.is_empty() {
        return Err(Error::Length);
    }
    let crate::value::Data::Boxed(leaves) = &value.data else {
        return Err(Error::Domain);
    };
    let mut decoded = Vec::with_capacity(leaves.len());
    for leaf in leaves.iter() {
        let item = decode_gerund_ar(leaf, span.clone(), depth + 1, names)?;
        if item.class != ParseClass::Verb {
            return Err(Error::Domain);
        }
        decoded.push(item.into_verb().unwrap().entity);
    }
    Ok(decoded)
}

fn gerund_primitive(spelling: &str, span: std::ops::Range<usize>) -> Result<Item> {
    use crate::primitive::PrimitiveSemanticId;
    let Some(primitive) =
        crate::primitive::PrimitiveResolver::core().resolve_core_for_enqueue(spelling)
    else {
        return Err(Error::Unsupported(
            "gerund name or unregistered primitive decoding".into(),
        ));
    };
    Ok(match primitive.semantic_id {
        PrimitiveSemanticId::Verb(id) => Item::verb(Verb {
            span: span.clone(),
            target: VerbTarget::Derived,
            entity: FunctionEntity::primitive(id, span),
        }),
        PrimitiveSemanticId::Adverb(id) => {
            Item::function(FunctionEntity::primitive_adverb(id, span))
        }
        PrimitiveSemanticId::Conjunction(id) => {
            Item::function(FunctionEntity::primitive_conjunction(id, span))
        }
        _ => return Err(Error::Unsupported("extension gerund decoding".into())),
    })
}

/// r.c::fxchar -> a.c::swap/sc.c::nameref: obtain actual current POS,
/// then retain ordinary function name references for later execution.
fn gerund_character(
    spelling: &str,
    span: std::ops::Range<usize>,
    names: ConstructionNames<'_, '_>,
) -> Result<Item> {
    let bytes = spelling.as_bytes();
    if bytes[0].is_ascii_alphabetic() && !matches!(bytes.last(), Some(b'.' | b':')) {
        if !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
        {
            return Err(Error::IllFormedName);
        }
        let queue = enqueue(spelling)?;
        if queue.len() != 1
            || !matches!(queue[0].payload, EnqueuedPayload::Name(name) if name == spelling)
        {
            return Err(Error::IllFormedName);
        }
        let binding = names.binding(spelling)?;
        if let Some(observations) = names.observations {
            let (class, facts) = match &binding {
                Some(ParserNameBinding::Noun(value)) => (
                    ParseClass::Noun,
                    Some(crate::j_graph_ir::GraphFacts::of(value)),
                ),
                Some(ParserNameBinding::AbstractNoun) => (ParseClass::Noun, None),
                Some(ParserNameBinding::Function(pos)) => ((*pos).into(), None),
                Some(ParserNameBinding::KnownModifier { function, .. }) => {
                    (function.result_pos.into(), None)
                }
                None => (ParseClass::Verb, None),
            };
            observations
                .borrow_mut()
                .push(CaptureEvent::GerundNameResolved {
                    row: names.row.expect("constructor row"),
                    read: crate::parser_capture::GerundNameRead {
                        name: spelling.into(),
                        version: names.host.and_then(|host| host.borrow().version(spelling)),
                        class,
                        facts,
                        span: span.clone(),
                    },
                });
        }
        let pos = match binding {
            Some(ParserNameBinding::Noun(value)) => {
                return Ok(Item::noun(
                    Expr {
                        span,
                        kind: ExprKind::Literal(value.into_shared()),
                    },
                    0,
                ));
            }
            Some(ParserNameBinding::AbstractNoun) => {
                return Ok(Item::noun(
                    Expr {
                        span,
                        kind: ExprKind::ReadName(spelling.into()),
                    },
                    0,
                ));
            }
            None => FunctionPartOfSpeech::Verb,
            Some(ParserNameBinding::Function(pos)) => pos,
            Some(ParserNameBinding::KnownModifier { function, .. }) => function.result_pos,
        };
        let entity = FunctionEntity::name_ref(spelling.into(), pos, span.clone());
        return Ok(if pos == FunctionPartOfSpeech::Verb {
            Item::verb(Verb {
                span,
                target: VerbTarget::Named(spelling.into()),
                entity,
            })
        } else {
            Item::function(entity)
        });
    }
    gerund_primitive(spelling, span)
}

/// r.c::jtfx core AR decoding. Constructor reductions share the parser's
/// disposition/actions; this is a serialized entity format, not another grammar.
fn decode_gerund_ar(
    value: &Value,
    span: std::ops::Range<usize>,
    depth: usize,
    names: ConstructionNames<'_, '_>,
) -> Result<Item> {
    use crate::value::Data;
    if depth >= MAX_EXPR_DEPTH {
        return Err(Error::Limit);
    }
    if let Data::Char(bytes) = &value.data {
        if value.shape.len() > 1 {
            return Err(Error::Rank);
        }
        if value.is_empty() {
            return Err(Error::Length);
        }
        if bytes.iter().any(|c| !(32..127).contains(c)) {
            return Err(Error::Spelling);
        }
        return gerund_character(std::str::from_utf8(bytes.as_slice()).unwrap(), span, names);
    }
    let Data::Boxed(fields) = &value.data else {
        return Err(Error::Domain);
    };
    if value.shape.len() > 1 {
        return Err(Error::Rank);
    }
    if !(1..=2).contains(&value.len()) {
        return Err(Error::Length);
    }
    let first = &fields[0];
    if first.is_empty() {
        return Err(Error::Length);
    }
    enum Head {
        Entity(Item),
        Noun,
        Hook,
        Fork,
        Modifier,
    }
    let head = if matches!(first.data, Data::Boxed(_)) {
        Head::Entity(decode_gerund_ar(first, span.clone(), depth + 1, names)?)
    } else {
        // u.c::vs audits header rank before converting to literal.
        if first.shape.len() > 1 {
            return Err(Error::Rank);
        }
        let Data::Char(bytes) = &first.data else {
            return Err(Error::Domain);
        };
        let spelling = std::str::from_utf8(bytes.as_slice()).map_err(|_| Error::Spelling)?;
        match spelling {
            "0" => Head::Noun,
            "2" => Head::Hook,
            "3" => Head::Fork,
            "4" => Head::Modifier,
            _ => Head::Entity(gerund_primitive(spelling, span.clone())?),
        }
    };
    if fields.len() == 2 && matches!(head, Head::Noun) {
        return Ok(Item::noun(
            Expr {
                span,
                kind: ExprKind::Literal(fields[1].as_ref().clone().into_shared()),
            },
            0,
        ));
    }
    let args = if fields.len() == 2 {
        let value = &fields[1];
        if value.shape.len() > 1 {
            return Err(Error::Rank);
        }
        let Data::Boxed(args) = &value.data else {
            return Err(Error::Domain);
        };
        args.as_slice()
    } else {
        &[]
    };
    let decode =
        |index: usize| decode_gerund_ar(args[index].as_ref(), span.clone(), depth + 1, names);
    match head {
        Head::Noun => Err(Error::Domain),
        Head::Hook | Head::Modifier => {
            if (matches!(head, Head::Hook) && args.len() != 2) || !(2..=3).contains(&args.len()) {
                return Err(Error::Length);
            }
            // r.c explicitly decodes h first. Both supplied Windows C variants
            // evaluate hook(fx(f),fx(g),h) with g before f; fixture this order.
            let third = if args.len() == 3 {
                Some(decode(2)?)
            } else {
                None
            };
            let second = decode(1)?;
            let first = decode(0)?;
            if let Some(third) = third {
                construct_modifier_trident(first, second, third, span, depth + 1, names)
            } else {
                construct_modifier_bident(first, second, span, depth + 1, names)
            }
        }
        Head::Fork => {
            if args.len() != 3 {
                return Err(Error::Length);
            }
            let first = decode(0)?;
            if !matches!(first.class, ParseClass::Noun | ParseClass::Verb) {
                return Err(Error::Syntax("invalid AR fork first operand".into()));
            }
            let second = decode(1)?;
            if second.class != ParseClass::Verb {
                return Err(Error::Syntax("invalid AR fork second operand".into()));
            }
            let third = decode(2)?;
            if third.class != ParseClass::Verb {
                return Err(Error::Syntax("invalid AR fork third operand".into()));
            }
            construct_modifier_trident(first, second, third, span, depth + 1, names)
        }
        Head::Entity(operator) => {
            if args.is_empty() {
                return Ok(operator);
            }
            let expected = match operator.class {
                ParseClass::Adverb => 1,
                ParseClass::Conjunction => 2,
                _ => 0,
            };
            if args.len() != expected {
                return Err(Error::Length);
            }
            let first = decode(0)?;
            if !matches!(first.class, ParseClass::Noun | ParseClass::Verb) {
                return Err(Error::Domain);
            }
            if expected == 1 {
                apply_adverb(
                    first,
                    operator.into_function().unwrap(),
                    span,
                    depth + 1,
                    names,
                )
            } else {
                let second = decode(1)?;
                if !matches!(second.class, ParseClass::Noun | ParseClass::Verb) {
                    return Err(Error::Domain);
                }
                apply_conjunction_items(
                    first,
                    operator.into_function().unwrap(),
                    second,
                    span,
                    depth + 1,
                    names,
                )
            }
        }
    }
}

/// Make repeated use of a concrete noun cheap without cloning its payload.
fn share_modifier_input(mut item: Item) -> Item {
    fn share(expr: Expr) -> Expr {
        let kind = match expr.kind {
            ExprKind::Literal(value) => ExprKind::Literal(value.into_shared()),
            ExprKind::Group(inner) => ExprKind::Group(Box::new(share(*inner))),
            other => other,
        };
        Expr {
            span: expr.span,
            kind,
        }
    }
    if let ParseValue::Noun(expr, height) = item.value {
        item.value = ParseValue::Noun(share(expr), height);
    }
    item
}

fn construct_modifier_bident(
    left: Item,
    right: Item,
    span: std::ops::Range<usize>,
    depth: usize,
    names: ConstructionNames<'_, '_>,
) -> Result<Item> {
    if depth >= MAX_EXPR_DEPTH {
        return Err(Error::Limit);
    }
    match bident_disposition(left.class, right.class) {
        BidentDisposition::BuildHook => Ok(Item::verb(train_hook(
            left.into_verb().unwrap(),
            right.into_verb().unwrap(),
        ))
        .with_span(span)),
        BidentDisposition::BuildDerivedModifier(pos) => {
            modifier_train(vec![left, right], pos).map(Item::function)
        }
        BidentDisposition::ImmediateSemanticApply if right.class == ParseClass::Adverb => {
            apply_adverb(left, right.into_function().unwrap(), span, depth + 1, names)
        }
        BidentDisposition::ImmediateSemanticApply => names.apply_noun(
            left.into_verb().expect("V N bident"),
            None,
            right.into_noun().expect("V N bident").0,
            span,
        ),
        BidentDisposition::SyntaxError => Err(Error::Syntax(
            "invalid modifier bident application result".into(),
        )),
    }
}

fn construct_modifier_trident(
    first: Item,
    second: Item,
    third: Item,
    span: std::ops::Range<usize>,
    depth: usize,
    names: ConstructionNames<'_, '_>,
) -> Result<Item> {
    if depth >= MAX_EXPR_DEPTH {
        return Err(Error::Limit);
    }
    match trident_disposition(first.class, second.class, third.class) {
        TridentDisposition::BuildFork if first.class == ParseClass::Verb => {
            Ok(Item::verb(train_fork(
                first.into_verb().unwrap(),
                second.into_verb().unwrap(),
                third.into_verb().unwrap(),
            ))
            .with_span(span))
        }
        TridentDisposition::BuildFork => Ok(Item::verb(train_noun_fork(
            first.into_noun().unwrap().0,
            second.into_verb().unwrap(),
            third.into_verb().unwrap(),
        )?)
        .with_span(span)),
        TridentDisposition::BuildDerivedModifier(pos) => {
            modifier_train(vec![first, second, third], pos).map(Item::function)
        }
        TridentDisposition::ImmediateSemanticApply if second.class == ParseClass::Conjunction => {
            apply_conjunction_items(
                first,
                second.into_function().unwrap(),
                third,
                span,
                depth + 1,
                names,
            )
        }
        TridentDisposition::ImmediateSemanticApply => names.apply_noun(
            second.into_verb().expect("N V N trident"),
            Some(first.into_noun().expect("N V N trident").0),
            third.into_noun().expect("N V N trident").0,
            span,
        ),
        TridentDisposition::SyntaxError => Err(Error::Syntax(
            "invalid modifier trident application result".into(),
        )),
    }
}

fn apply_conjunction_items(
    left: Item,
    operator: Arc<FunctionEntity>,
    right: Item,
    span: std::ops::Range<usize>,
    depth: usize,
    names: ConstructionNames<'_, '_>,
) -> Result<Item> {
    if depth >= MAX_EXPR_DEPTH {
        return Err(Error::Limit);
    }
    debug_assert_eq!(operator.result_pos, FunctionPartOfSpeech::Conjunction);
    let operator = names.resolve_modifier(operator, span.clone())?;
    if let FunctionHead::DefinitionConstructor(origin) = &operator.head {
        let (left, _) = left.into_noun().ok_or(Error::Domain)?;
        let (right, _) = right.into_noun().ok_or(Error::Domain)?;
        let mode = completed_noun(left, "computed definition mode")?;
        let body = completed_noun(right, "computed definition body")?;
        let expected_mode = match origin.input.form {
            crate::definition_input::DefinitionForm::Direct => 9,
            crate::definition_input::DefinitionForm::NounDirect => return Err(Error::Domain),
            crate::definition_input::DefinitionForm::ExplicitString(m)
            | crate::definition_input::DefinitionForm::ExplicitBlock(m) => i64::from(m),
        };
        if !mode.shape().is_empty() || mode.int_at(0)? != expected_mode {
            return Err(Error::Unsupported("computed definition mode".into()));
        }
        let crate::Data::Char(bytes) = &body.data else {
            return Err(Error::Domain);
        };
        if bytes.as_slice()
            != crate::definition_code::semantic_body(&origin.source, &origin.input)?.as_bytes()
        {
            return Err(Error::Unsupported("computed definition body".into()));
        }
        let code =
            crate::definition_code::compile(&origin.source, &origin.input, &origin.primitives)?;
        let function = FunctionEntity::derived(
            FunctionHead::ExplicitDefinition(code.clone()),
            code.result_pos,
            span.clone(),
            Vec::new(),
        );
        return Ok(if code.result_pos == FunctionPartOfSpeech::Verb {
            Item::verb(Verb {
                span,
                target: VerbTarget::Derived,
                entity: function,
            })
        } else {
            Item::function(function)
        });
    }

    if matches!(operator.head, FunctionHead::ModifierTrain) {
        match operator.operands.as_slice() {
            [
                FunctionOperand::Function(first),
                FunctionOperand::Function(second),
            ] if first.result_pos == FunctionPartOfSpeech::Conjunction => {
                let left = share_modifier_input(left);
                let right = share_modifier_input(right);
                let result = apply_conjunction_items(
                    left.clone(),
                    first.clone(),
                    right.clone(),
                    span.clone(),
                    depth + 1,
                    names,
                )?;
                let other = match second.result_pos {
                    FunctionPartOfSpeech::Adverb => {
                        modifier_operand(&operator.operands[1], span.clone())
                    } // tca
                    FunctionPartOfSpeech::Conjunction => apply_conjunction_items(
                        left,
                        second.clone(),
                        right,
                        span.clone(),
                        depth + 1,
                        names,
                    )?, // tcc
                    _ => {
                        return Err(Error::Unsupported(
                            "derived conjunction bident semantics".into(),
                        ));
                    }
                };
                return construct_modifier_bident(result, other, span, depth + 1, names);
            }
            operands if operands.len() == 3 => {
                return apply_modifier_trident(left, Some(right), operands, span, depth + 1, names);
            }
            _ => {
                return Err(Error::Unsupported(
                    "derived conjunction application semantics".into(),
                ));
            }
        }
    }
    if !operator.is_primitive_modifier() {
        return Err(Error::Unsupported(
            "conjunction child identity requires resolution".into(),
        ));
    }
    apply_conjunction_at(left, operator, right, span, depth + 1, names).map(Item::verb)
}

/// cf.c's trident actions construct with actual intermediate POS, in source
/// action order. Shared inputs survive every branch without payload copies.
fn apply_modifier_trident(
    left: Item,
    right: Option<Item>,
    operands: &[FunctionOperand],
    span: std::ops::Range<usize>,
    depth: usize,
    names: ConstructionNames<'_, '_>,
) -> Result<Item> {
    use ParseClass::{Adverb as A, Conjunction as C, Noun as N, Verb as V};
    if depth >= MAX_EXPR_DEPTH {
        return Err(Error::Limit);
    }
    let left = share_modifier_input(left);
    let right = right.map(share_modifier_input);
    let parts: Vec<_> = operands
        .iter()
        .map(|o| modifier_operand(o, span.clone()))
        .collect();
    let classes = [parts[0].class, parts[1].class, parts[2].class];
    let adv = |index: usize, input: Item| {
        apply_adverb(
            input,
            parts[index].clone().into_function().unwrap(),
            span.clone(),
            depth + 1,
            names,
        )
    };
    let conj = |index: usize| {
        apply_conjunction_items(
            left.clone(),
            parts[index].clone().into_function().unwrap(),
            right.clone().ok_or_else(|| {
                Error::Unsupported("conjunction train requires two inputs".into())
            })?,
            span.clone(),
            depth + 1,
            names,
        )
    };
    let finish = |f: Item, g: Item, h: Item| {
        construct_modifier_trident(f, g, h, span.clone(), depth + 1, names)
    };
    match classes {
        [A, A, A] => {
            // taaa
            let t = adv(0, left.clone())?;
            let t = adv(1, t)?;
            adv(2, t)
        }
        [N | V, C, A] => {
            // tNVca: h first, then f g t
            let t = adv(2, left.clone())?;
            finish(parts[0].clone(), parts[1].clone(), t)
        }
        [A, V | C, N | V] => {
            // taVCNV
            let t = adv(0, left.clone())?;
            finish(t, parts[1].clone(), parts[2].clone())
        }
        [A, A, V] => {
            // taav
            let t = adv(0, left.clone())?;
            let tt = adv(1, right.clone().unwrap())?;
            finish(t, tt, parts[2].clone())
        }
        [N | V, V, C] | [N | V, C, C] => {
            // tNVvc / tNVcc
            let t = conj(2)?;
            finish(parts[0].clone(), parts[1].clone(), t)
        }
        [C, V | C, C] => {
            // tcVCc
            let t = conj(0)?;
            let tt = conj(2)?;
            finish(t, parts[1].clone(), tt)
        }
        [C, A, A] => {
            // tcaa
            let t = conj(0)?;
            let t = adv(1, t)?;
            adv(2, t)
        }
        [A, C, A] => {
            // taca
            let t = adv(0, left.clone())?;
            let tt = adv(2, right.clone().unwrap())?;
            finish(t, parts[1].clone(), tt)
        }
        [A, C, C] => {
            // tacc
            let t = adv(0, left.clone())?;
            let tt = conj(2)?;
            finish(t, parts[1].clone(), tt)
        }
        [C, V | C, N | V] => {
            // tcVCNV
            let t = conj(0)?;
            finish(t, parts[1].clone(), parts[2].clone())
        }
        [C, C, A] => {
            // tcca
            let t = conj(0)?;
            let tt = adv(2, right.clone().unwrap())?;
            finish(t, parts[1].clone(), tt)
        }
        _ => Err(Error::Unsupported("derived modifier trident action".into())),
    }
}

/// Bound values are immutable and shared. Spans here describe their current
/// application use; the original definition remains on the train identity.
fn modifier_operand(operand: &FunctionOperand, span: std::ops::Range<usize>) -> Item {
    match operand {
        FunctionOperand::Noun { value, .. } => Item::noun(
            Expr {
                span,
                kind: ExprKind::Literal(value.clone()),
            },
            0,
        ),
        FunctionOperand::Function(function)
            if function.result_pos == FunctionPartOfSpeech::Verb =>
        {
            Item::verb(Verb {
                span,
                target: VerbTarget::Derived,
                entity: function.clone(),
            })
        }
        FunctionOperand::Function(function) => Item::function(function.clone()).with_span(span),
    }
}

fn apply_conjunction_at(
    left: Item,
    operator: Arc<FunctionEntity>,
    right: Item,
    span: std::ops::Range<usize>,
    depth: usize,
    names: ConstructionNames<'_, '_>,
) -> Result<Verb> {
    debug_assert_eq!(operator.result_pos, FunctionPartOfSpeech::Conjunction);
    let primitive_id = match &operator.head {
        FunctionHead::PrimitiveConjunction(id) => Some(*id),
        _ => None,
    };
    // cr.c::jtqq audits the right rank operand before inspecting noun-left
    // constant/gerund construction. Keep both original operands in the DAG.
    let ranks;
    let right_operand = match right.value {
        ParseValue::Noun(expr, _) => {
            if matches!(primitive_id, Some(crate::primitive::ConjunctionId::Atop)) {
                return Err(Error::Domain);
            }
            let noun_span = expr.span.clone();
            let value = completed_noun(expr, "runtime-dependent conjunction noun operand")?;
            ranks = Some(rank_noun_contract(&value)?);
            FunctionOperand::Noun {
                span: noun_span,
                value: value.into_shared(),
            }
        }
        ParseValue::Verb(verb) => {
            ranks = None;
            FunctionOperand::Function(verb.entity)
        }
        _ => return Err(Error::Syntax("invalid conjunction right operand".into())),
    };
    let mut decoded = None;
    let left_operand = match left.value {
        ParseValue::Verb(verb) => FunctionOperand::Function(verb.entity),
        ParseValue::Noun(expr, _) => {
            if !matches!(primitive_id, Some(crate::primitive::ConjunctionId::Rank)) {
                return Err(Error::Domain);
            }
            let noun_span = expr.span.clone();
            let value = completed_noun(expr, "runtime-dependent noun-left rank operand")?;
            if value.shape.len() == 1
                && matches!(value.data, crate::value::Data::Boxed(_))
                && ranks != Some([63; 3])
            {
                match audit_gerund(&value, noun_span.clone(), depth + 1, names) {
                    Ok(functions) => decoded = Some(functions),
                    Err(error) if error.kind() == "unsupported" => return Err(error),
                    // cr.c suppresses failed fx audits and uses the noun itself.
                    Err(_) => {}
                }
            }
            FunctionOperand::Noun {
                value: value.into_shared(),
                span: noun_span,
            }
        }
        _ => return Err(Error::Syntax("invalid conjunction left operand".into())),
    };
    let operands = vec![left_operand, right_operand];
    Ok(Verb {
        span: span.clone(),
        target: VerbTarget::Derived,
        entity: FunctionEntity::with_decoded_gerund(
            FunctionEntity::derived(
                operator.head.clone(),
                FunctionPartOfSpeech::Verb,
                span,
                operands,
            ),
            decoded,
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
    source: AssignmentSource,
}

fn reduce_parse_stack_subset(
    mut queue: Vec<Item>,
    context: &mut ActionContext<'_>,
    reductions: &mut Vec<ParseReduction>,
) -> Result<(Vec<Item>, Option<PendingAssignment>)> {
    let mut stack = Vec::<Item>::new();
    let mut assignment = None;

    while let Some(item) = queue.pop() {
        let name = match &item.value {
            ParseValue::LookupName { name, .. } => Some(name.clone()),
            _ => None,
        };
        let mut item = resolve_stack_item(item, context)?;
        let version = name
            .as_deref()
            .and_then(|name| context.host.as_ref().and_then(|host| host.version(name)));
        if let (Some(capture), ParseValue::Noun(expr, _)) = (&mut context.capture, &item.value) {
            if let ExprKind::Literal(value) = &expr.kind {
                let id = capture.next();
                capture.events.push(CaptureEvent::Input {
                    id,
                    name,
                    version,
                    span: item.span(),
                    facts: crate::j_graph_ir::GraphFacts::of(value),
                    word_index: item
                        .provenance
                        .as_ref()
                        .expect("queued input provenance")
                        .blame_word_index,
                });
                item.occurrence = Some(id);
            }
        }
        stack.insert(0, item);
        reduce_stack_prefix(
            &mut stack,
            &mut assignment,
            queue.is_empty(),
            context,
            reductions,
        )?;
    }

    // jsource realizes the virtual FRONT MARK only after the queue is empty.
    if assignment.is_none() {
        stack.insert(0, Item::mark(0));
        reduce_stack_prefix(&mut stack, &mut assignment, true, context, reductions)?;
    }

    if stack
        .first()
        .is_some_and(|item| item.class == ParseClass::Mark)
    {
        stack.remove(0);
    }
    Ok((stack, assignment))
}

fn stack_prefix_classes(stack: &[Item]) -> [ParseClass; 4] {
    let class = |index: usize| stack.get(index).map_or(ParseClass::Mark, |item| item.class);
    [class(0), class(1), class(2), class(3)]
}

fn reduce_stack_prefix(
    stack: &mut Vec<Item>,
    assignment: &mut Option<PendingAssignment>,
    queue_exhausted: bool,
    context: &mut ActionContext<'_>,
    reductions: &mut Vec<ParseReduction>,
) -> Result<()> {
    loop {
        let Some(row) = match_parse_row(stack_prefix_classes(stack)) else {
            return Ok(());
        };

        // Match jsource's inherited .t token: modifiers/forks/hooks inherit
        // the left operand; parentheses inherit '('. Noun call results keep the
        // right noun token, which p.c marks immaterial for non-executable nouns.
        let (start, count, inherit, fail) = match row {
            ParseRow::MonadEdge => (1, 2, 2, 1),
            ParseRow::MonadVVN => (2, 2, 3, 2),
            ParseRow::DyadNVN => (1, 3, 3, 2),
            ParseRow::Adverb => (1, 2, 1, 2),
            ParseRow::Conjunction => (1, 3, 1, 2),
            ParseRow::Fork => (1, 3, 1, 2),
            ParseRow::Hook if stack.get(3).is_some_and(|item| is_cavn(item.class)) => (1, 3, 2, 2),
            ParseRow::Hook => (1, 2, 1, 1),
            ParseRow::Assignment => (0, 3, 2, 1),
            ParseRow::Parenthesis => (0, 3, 0, 0),
        };
        let inputs: Vec<_> = stack[start..start + count]
            .iter()
            .map(|item| item.provenance.clone().expect("source item provenance"))
            .collect();
        let provenance = ParseProvenance {
            word_range: inputs[0].word_range.start..inputs.last().unwrap().word_range.end,
            blame_word_index: stack[inherit].provenance.as_ref().unwrap().blame_word_index,
        };
        let failure = stack[fail].provenance.as_ref().unwrap().blame_word_index;
        let failure_span = stack[fail].span();
        let reduction_span = stack[start].span().start..stack[start + count - 1].span().end;
        let is_call = matches!(
            row,
            ParseRow::MonadEdge | ParseRow::MonadVVN | ParseRow::DyadNVN
        );
        let is_construction = matches!(
            row,
            ParseRow::Adverb | ParseRow::Conjunction | ParseRow::Fork | ParseRow::Hook
        );
        let retained = match row {
            ParseRow::Parenthesis => stack[1].occurrence,
            ParseRow::Assignment => stack[2].occurrence,
            _ => None,
        };
        let mut output = None;
        if let Some(capture) = &mut context.capture {
            if is_call {
                let verb_slot = if row == ParseRow::MonadEdge { 1 } else { 2 };
                let ParseValue::Verb(verb) = &stack[verb_slot].value else {
                    unreachable!()
                };
                let right_slot = start + count - 1;
                let id = capture.next();
                capture.events.push(CaptureEvent::ApplyAttempt {
                    id,
                    function: verb.entity.clone(),
                    left: if row == ParseRow::DyadNVN {
                        stack[1].occurrence
                    } else {
                        None
                    },
                    right: stack[right_slot]
                        .occurrence
                        .expect("runtime noun occurrence"),
                    span: reduction_span.clone(),
                    word_index: failure,
                });
                output = Some(id);
            } else if is_construction {
                capture.events.push(CaptureEvent::ConstructionAttempt {
                    row,
                    noun_inputs: stack[start..start + count]
                        .iter()
                        .filter_map(|item| item.occurrence)
                        .collect(),
                    span: reduction_span.clone(),
                });
            }
        }
        let reduced = apply_parse_row(row, stack, assignment, queue_exhausted, context)
            .map_err(|error| error.at(failure_span).blamed_on_word(failure));
        let reduced = match reduced {
            Ok(reduced) => reduced,
            Err(error) => {
                if let Some(capture) = &mut context.capture {
                    if let Some(id) = output {
                        capture.events.push(CaptureEvent::ApplyFailure {
                            id,
                            kind: error.kind().into(),
                            context: error.context().cloned(),
                        });
                    } else if is_construction {
                        capture.events.push(CaptureEvent::ConstructionFailure {
                            row,
                            kind: error.kind().into(),
                            span: reduction_span,
                        });
                    }
                }
                return Err(error);
            }
        };
        if reduced {
            let result = &mut stack[start];
            result.provenance = Some(provenance.clone());
            result.occurrence = output.or(retained);
            if let Some(capture) = &mut context.capture {
                if let Some(id) = output {
                    let ParseValue::Noun(expr, _) = &result.value else {
                        unreachable!()
                    };
                    let ExprKind::Literal(value) = &expr.kind else {
                        unreachable!()
                    };
                    capture.events.push(CaptureEvent::ApplySuccess {
                        id,
                        facts: crate::j_graph_ir::GraphFacts::of(value),
                    });
                } else if is_construction {
                    let function = match &result.value {
                        ParseValue::Verb(verb) => verb.entity.clone(),
                        ParseValue::Function(function) => function.clone(),
                        _ => unreachable!(),
                    };
                    capture.events.push(CaptureEvent::ConstructionSuccess {
                        row,
                        function,
                        span: reduction_span.clone(),
                    });
                }
            }
            reductions.push(ParseReduction {
                row,
                inputs,
                result: provenance,
                result_class: result.class,
                span: reduction_span,
            });
        }

        if !reduced || assignment.is_some() {
            return Ok(());
        }
    }
}

fn runtime_noun(
    mut expression: Expr,
    height: usize,
    context: &mut ActionContext<'_>,
) -> Result<Item> {
    if let Some(host) = &mut context.host {
        let span = expression.span.clone();
        let value = host.apply(expression)?;
        expression = Expr {
            span,
            kind: ExprKind::Literal(value),
        };
    }
    Ok(Item::noun(expression, height))
}

fn apply_parse_row(
    row: ParseRow,
    stack: &mut Vec<Item>,
    assignment: &mut Option<PendingAssignment>,
    queue_exhausted: bool,
    context: &mut ActionContext<'_>,
) -> Result<bool> {
    Ok(match row {
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
            stack.insert(1, runtime_noun(expr, checked_height(height)?, context)?);
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
            stack.insert(2, runtime_noun(expr, checked_height(height)?, context)?);
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
                runtime_noun(
                    expr,
                    checked_height(left_height.max(right_height))?,
                    context,
                )?,
            );
            true
        }
        ParseRow::Adverb => {
            let mut phrase: Vec<_> = stack.drain(1..3).collect();
            let left = phrase.remove(0);
            let operator = phrase.remove(0);
            let operator_span = operator.span();
            let span = left.span().start..operator_span.end;
            let operator = resolve_modifier(
                operator.into_function().expect("row 3 adverb"),
                operator_span,
                context,
                row,
            )?;
            let observations = std::cell::RefCell::new(Vec::new());
            let host = construction_host(&mut context.host);
            let names = ConstructionNames {
                lookup: context.lookup,
                host: host.as_ref(),
                observations: context.capture.as_ref().map(|_| &observations),
                row: Some(row),
            };
            let result = apply_adverb(left, operator, span.clone(), 0, names)
                .map_err(|error| error.at(span.clone()));
            if let Some(capture) = &mut context.capture {
                capture.events.extend(observations.into_inner());
            }
            stack.insert(1, result?.with_span(span));
            true
        }
        ParseRow::Conjunction => {
            let mut phrase: Vec<_> = stack.drain(1..4).collect();
            let left = phrase.remove(0);
            let operator = phrase.remove(0);
            let operator_span = operator.span();
            let right = phrase.remove(0);
            let span = left.span().start..right.span().end;
            let operator = resolve_modifier(
                operator.into_function().expect("row 4 conjunction"),
                operator_span,
                context,
                row,
            )?;
            let observations = std::cell::RefCell::new(Vec::new());
            let host = construction_host(&mut context.host);
            let names = ConstructionNames {
                lookup: context.lookup,
                host: host.as_ref(),
                observations: context.capture.as_ref().map(|_| &observations),
                row: Some(row),
            };
            let result = apply_conjunction_items(left, operator, right, span.clone(), 0, names)
                .map_err(|error| error.at(span.clone()));
            if let Some(capture) = &mut context.capture {
                capture.events.extend(observations.into_inner());
            }
            stack.insert(1, result?.with_span(span));
            true
        }
        ParseRow::Fork => {
            let classes = [
                stack.get(1).map_or(ParseClass::Mark, |item| item.class),
                stack.get(2).map_or(ParseClass::Mark, |item| item.class),
                stack.get(3).map_or(ParseClass::Mark, |item| item.class),
            ];
            match trident_disposition(classes[0], classes[1], classes[2]) {
                TridentDisposition::BuildFork
                    if classes == [ParseClass::Verb, ParseClass::Verb, ParseClass::Verb] =>
                {
                    let mut phrase: Vec<_> = stack.drain(1..4).collect();
                    let f = phrase.remove(0).into_verb().expect("row 5 f");
                    let g = phrase.remove(0).into_verb().expect("row 5 g");
                    let h = phrase.remove(0).into_verb().expect("row 5 h");
                    stack.insert(1, Item::verb(train_fork(f, g, h)));
                    true
                }
                TridentDisposition::BuildFork
                    if classes == [ParseClass::Noun, ParseClass::Verb, ParseClass::Verb] =>
                {
                    let mut phrase: Vec<_> = stack.drain(1..4).collect();
                    let (noun, _) = phrase.remove(0).into_noun().expect("row 5 noun");
                    let g = phrase.remove(0).into_verb().expect("row 5 g");
                    let h = phrase.remove(0).into_verb().expect("row 5 h");
                    let span = noun.span.start..h.span.end;
                    stack.insert(
                        1,
                        Item::verb(train_noun_fork(noun, g, h).map_err(|error| error.at(span))?),
                    );
                    true
                }
                TridentDisposition::BuildFork => {
                    return Err(Error::Syntax(
                        "row 5 fork disposition has invalid parser classes".into(),
                    ));
                }
                TridentDisposition::ImmediateSemanticApply => {
                    return Err(Error::Unsupported(
                        "row 5 requires parser-time semantic execution".into(),
                    ));
                }
                TridentDisposition::BuildDerivedModifier(result_pos) => {
                    return Err(Error::Unsupported(format!(
                        "row 5 derived modifier result {result_pos:?} is not yet represented"
                    )));
                }
                TridentDisposition::SyntaxError => {
                    return Err(Error::Syntax(
                        "invalid jsource trident part-of-speech combination".into(),
                    ));
                }
            }
        }
        ParseRow::Hook => {
            let left = stack[1].class;
            let right = stack[2].class;
            if stack.get(3).is_some_and(|item| is_cavn(item.class)) {
                match trident_disposition(left, right, stack[3].class) {
                    TridentDisposition::BuildDerivedModifier(pos) => {
                        let phrase = stack.drain(1..4).collect();
                        stack.insert(1, Item::function(modifier_train(phrase, pos)?));
                        true
                    }
                    TridentDisposition::ImmediateSemanticApply => {
                        return Err(Error::Unsupported(
                            "row 6 trident requires semantic application".into(),
                        ));
                    }
                    TridentDisposition::SyntaxError => {
                        return Err(Error::Syntax(
                            "invalid jsource trident part-of-speech combination".into(),
                        ));
                    }
                    TridentDisposition::BuildFork => {
                        return Err(Error::Syntax("fork must be selected by row 5".into()));
                    }
                }
            } else {
                match bident_disposition(left, right) {
                    BidentDisposition::BuildHook => {
                        let mut phrase: Vec<_> = stack.drain(1..3).collect();
                        let f = phrase.remove(0).into_verb().expect("row 6 f");
                        let g = phrase.remove(0).into_verb().expect("row 6 g");
                        stack.insert(1, Item::verb(train_hook(f, g)));
                        true
                    }
                    BidentDisposition::BuildDerivedModifier(pos) => {
                        let phrase = stack.drain(1..3).collect();
                        stack.insert(1, Item::function(modifier_train(phrase, pos)?));
                        true
                    }
                    BidentDisposition::ImmediateSemanticApply => {
                        return Err(Error::Unsupported(
                            "row 6 requires parser-time semantic execution".into(),
                        ));
                    }
                    BidentDisposition::SyntaxError => {
                        return Err(Error::Syntax(
                            "invalid jsource bident part-of-speech combination".into(),
                        ));
                    }
                }
            }
        }
        ParseRow::Assignment => {
            if !queue_exhausted && context.host.is_none() {
                return Err(Error::Unsupported(
                    "non-final assignment requires runtime semantic parsing".into(),
                ));
            }
            if assignment.is_some() {
                return Err(Error::Unsupported(
                    "multiple assignments in one sentence".into(),
                ));
            }
            if stack
                .first()
                .is_some_and(|item| item.class == ParseClass::Noun)
            {
                return Err(Error::Unsupported("noun/multiple assignment target".into()));
            }

            let mut phrase: Vec<_> = stack.drain(0..3).collect();
            let target = phrase.remove(0);
            let copula = phrase.remove(0);
            let value = phrase.remove(0);
            let ParseValue::NameTarget { name, span } = target.value else {
                return Err(Error::Syntax("row 7 requires a name target".into()));
            };
            let source = AssignmentSource {
                target: target.provenance.expect("assignment target provenance"),
                copula: copula.provenance.expect("copula provenance"),
                flags: copula.flags,
            };
            // p.c row 7 transports the stacked RHS. A nonnameless modifier
            // remains a POS-bearing NameRef; assigning it is not application.
            // Nameless modifiers were already stacked by value during lookup.
            if let Some(host) = context.host.as_mut() {
                if source.flags.local_assignment || !source.flags.global_assignment {
                    return Err(Error::Unsupported(
                        "local parser-time assignment scope".into(),
                    ));
                }
                let value_span = value.span();
                let occurrence = value.occurrence;
                let class = value.class;
                let (assigned, height, verb_adapter) = match value.value {
                    ParseValue::Noun(expr, height) => (
                        JEntity::Noun(completed_noun(expr, "assignment value")?),
                        height,
                        None,
                    ),
                    ParseValue::Verb(verb) => (
                        JEntity::Function(verb.entity),
                        0,
                        Some((verb.span, verb.target)),
                    ),
                    ParseValue::Function(function) => (JEntity::Function(function), 0, None),
                    _ => return Err(Error::Syntax("invalid assignment value".into())),
                };
                let previous = host.version(&name);
                let assigned = host.assign(&name, assigned)?;
                let function = match &assigned {
                    JEntity::Function(function) => Some(function.clone()),
                    _ => None,
                };
                if let Some(capture) = &mut context.capture {
                    capture.events.push(CaptureEvent::Commit {
                        name: name.clone(),
                        version: host.version(&name).expect("committed version"),
                        previous,
                        span: span.clone(),
                        value: occurrence,
                        final_assignment: queue_exhausted,
                        class,
                        function,
                        source: source.clone(),
                    });
                }
                let result = match assigned {
                    JEntity::Noun(value) => Item::noun(
                        Expr {
                            span: value_span,
                            kind: ExprKind::Literal(value),
                        },
                        height,
                    ),
                    JEntity::Function(function)
                        if function.result_pos == FunctionPartOfSpeech::Verb =>
                    {
                        let (span, target) = verb_adapter.ok_or(Error::Domain)?;
                        Item::verb(Verb {
                            span,
                            target,
                            entity: function,
                        })
                    }
                    JEntity::Function(function) => Item::function(function),
                };
                stack.insert(0, result);
                if queue_exhausted {
                    *assignment = Some(PendingAssignment { name, span, source });
                }
            } else {
                *assignment = Some(PendingAssignment { name, span, source });
                stack.insert(0, value);
            }
            true
        }
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
                ParseValue::Function(entity) => Item::function(entity).with_span(group_span),
                ParseValue::LookupName { .. }
                | ParseValue::NameTarget { .. }
                | ParseValue::Control { .. } => {
                    return Err(
                        Error::Syntax("invalid parenthesized parser control".into()).at(group_span)
                    );
                }
            };
            stack.insert(0, grouped);
            true
        }
    })
}

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
    matches!(
        class,
        ParseClass::Adverb | ParseClass::Verb | ParseClass::Noun
    )
}

fn is_cavn(class: ParseClass) -> bool {
    matches!(
        class,
        ParseClass::Conjunction | ParseClass::Adverb | ParseClass::Verb | ParseClass::Noun
    )
}

fn is_edge(class: ParseClass) -> bool {
    matches!(
        class,
        ParseClass::Mark | ParseClass::Assignment | ParseClass::LParen
    )
}

fn is_edge_or_avn(class: ParseClass) -> bool {
    is_edge(class) || is_avn(class)
}

/// Declarative equivalent of the pinned jsource p.c cases table.
///
/// The first matching row is the parser precedence. Actions are migrated onto
/// this table incrementally; the table itself is already the single source for
/// row eligibility.
pub fn match_parse_row(classes: [ParseClass; 4]) -> Option<ParseRow> {
    use ParseClass::*;
    let [a, b, c, d] = classes;
    [
        (ParseRow::MonadEdge, is_edge(a) && b == Verb && c == Noun),
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
        (ParseRow::Hook, is_edge(a) && is_cavn(b) && is_cavn(c)),
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

/// cf.c jthook with a nonzero action function: construction returns a
/// modifier without applying it. N/V operands remain semantic operands;
/// no execution-only fgh helper or compiler fact becomes a child.
fn modifier_train(phrase: Vec<Item>, result: ParseClass) -> Result<Arc<FunctionEntity>> {
    let span = phrase.first().unwrap().span().start..phrase.last().unwrap().span().end;
    let mut operands = Vec::with_capacity(phrase.len());
    for item in phrase {
        let item_span = item.span();
        operands.push(match item.value {
            ParseValue::Noun(expr, _) => FunctionOperand::Noun {
                value: completed_noun(expr, "runtime-dependent modifier train noun operand")?
                    .into_shared(),
                span: item_span,
            },
            ParseValue::Verb(verb) => FunctionOperand::Function(verb.entity),
            ParseValue::Function(function) => FunctionOperand::Function(function),
            _ => return Err(Error::Syntax("invalid modifier train operand".into())),
        });
    }
    let result_pos = match result {
        ParseClass::Adverb => FunctionPartOfSpeech::Adverb,
        ParseClass::Conjunction => FunctionPartOfSpeech::Conjunction,
        _ => return Err(Error::Syntax("invalid modifier train result POS".into())),
    };
    Ok(FunctionEntity::derived(
        FunctionHead::ModifierTrain,
        result_pos,
        span,
        operands,
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Execution-free classification shared by row 6 and AR construction.
/// Immediate application has value-dependent success and actual result POS.
pub enum BidentDisposition {
    SyntaxError,
    ImmediateSemanticApply,
    BuildHook,
    BuildDerivedModifier(ParseClass),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Execution-free classification shared by row 5 and AR construction.
/// This is construction dispatch, not a call/lowering capability promise.
pub enum TridentDisposition {
    SyntaxError,
    ImmediateSemanticApply,
    BuildFork,
    BuildDerivedModifier(ParseClass),
}

pub fn bident_disposition(left: ParseClass, right: ParseClass) -> BidentDisposition {
    use BidentDisposition::*;
    use ParseClass::{Adverb as A, Conjunction as C, Noun as N, Verb as V};
    match (left, right) {
        (V, V) => BuildHook,
        (V, N) | (N, A) | (V, A) => ImmediateSemanticApply,

        (N, C) | (V, C) => BuildDerivedModifier(A),
        (A, V) | (A, A) | (A, C) => BuildDerivedModifier(A),
        (C, N) | (C, V) => BuildDerivedModifier(A),
        (C, A) | (C, C) => BuildDerivedModifier(C),

        _ => SyntaxError,
    }
}

pub fn trident_disposition(
    first: ParseClass,
    second: ParseClass,
    third: ParseClass,
) -> TridentDisposition {
    use ParseClass::{Adverb as A, Conjunction as C, Noun as N, Verb as V};
    use TridentDisposition::*;
    match (first, second, third) {
        (V, V, V) | (N, V, V) => BuildFork,

        (N, V, N) | (N, C, N) | (N, C, V) | (V, C, N) | (V, C, V) => ImmediateSemanticApply,

        (A, A, A) => BuildDerivedModifier(A),
        (A, A, V) => BuildDerivedModifier(C),
        (V, V, C) | (N, V, C) => BuildDerivedModifier(C),
        (A, V, V) => BuildDerivedModifier(A),
        (C, V, V) | (C, V, C) => BuildDerivedModifier(C),
        (C, A, A) => BuildDerivedModifier(C),

        (N, C, A) | (V, C, A) => BuildDerivedModifier(A),
        (N, C, C) | (V, C, C) => BuildDerivedModifier(C),

        (A, C, N) | (A, C, V) => BuildDerivedModifier(A),
        (A, C, A) | (A, C, C) => BuildDerivedModifier(C),

        (C, C, N) | (C, C, V) | (C, C, A) | (C, C, C) => BuildDerivedModifier(C),

        _ => SyntaxError,
    }
}

#[derive(Clone)]
enum ParseValue {
    LookupName {
        name: String,
        span: std::ops::Range<usize>,
    },
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
    provenance: Option<ParseProvenance>,
    flags: EnqueueFlags,
    occurrence: Option<OccurrenceId>,
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
            ParseValue::NameTarget { span, .. } | ParseValue::LookupName { span, .. } => {
                span.clone()
            }
            ParseValue::Control { span } => span.clone(),
        }
    }

    fn with_source(mut self, word: &EnqueuedWord<'_>) -> Self {
        self.provenance = Some(ParseProvenance {
            word_range: word.word_index..word.word_index + 1,
            blame_word_index: word.word_index,
        });
        self.flags = word.flags;
        self
    }

    fn lookup_name(name: String, span: std::ops::Range<usize>) -> Self {
        Self {
            class: ParseClass::Name,
            value: ParseValue::LookupName { name, span },
            span_override: None,
            provenance: None,
            flags: EnqueueFlags::default(),
            occurrence: None,
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
            provenance: None,
            flags: EnqueueFlags::default(),
            occurrence: None,
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
            provenance: None,
            flags: EnqueueFlags::default(),
            occurrence: None,
        }
    }

    fn name_target(name: String, span: std::ops::Range<usize>) -> Self {
        Self {
            class: ParseClass::Name,
            value: ParseValue::NameTarget { name, span },
            span_override: None,
            provenance: None,
            flags: EnqueueFlags::default(),
            occurrence: None,
        }
    }

    fn noun(expr: Expr, height: usize) -> Self {
        Self {
            class: ParseClass::Noun,
            value: ParseValue::Noun(expr, height),
            span_override: None,
            provenance: None,
            flags: EnqueueFlags::default(),
            occurrence: None,
        }
    }

    fn verb(verb: Verb) -> Self {
        debug_assert_eq!(verb.entity.result_pos, FunctionPartOfSpeech::Verb);
        Self {
            class: ParseClass::Verb,
            value: ParseValue::Verb(verb),
            span_override: None,
            provenance: None,
            flags: EnqueueFlags::default(),
            occurrence: None,
        }
    }

    fn function(entity: Arc<FunctionEntity>) -> Self {
        Self {
            class: entity.result_pos.into(),
            value: ParseValue::Function(entity),
            span_override: None,
            provenance: None,
            flags: EnqueueFlags::default(),
            occurrence: None,
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
    parse_with(source, None, ParseContext::Analysis).map_err(Error::into_unlocated)
}

/// Diagnostic frontend entry point. It uses the same parser semantics as
/// `parse` but retains source spans on errors for compiler/interpreter/JIT UI.
pub fn parse_diagnostic(source: &str) -> Result<Program> {
    parse_with(source, None, ParseContext::Analysis)
}

#[derive(Clone, Debug)]
pub(crate) enum ParserNameBinding {
    Noun(Value),
    /// Analysis-only noun class, with facts owned by the input catalog.
    /// This is not a dummy Value and must never enter concrete execution.
    AbstractNoun,
    Function(FunctionPartOfSpeech),
    KnownModifier {
        function: Arc<FunctionEntity>,
        version: crate::semantic::NameVersion,
    },
}

#[cfg(test)]
fn parse_runtime(
    source: &str,
    lookup: &dyn Fn(&str) -> Option<ParserNameBinding>,
) -> Result<Program> {
    parse_with(source, Some(lookup), ParseContext::Runtime)
}

pub(crate) fn parse_analysis(
    source: &str,
    lookup: &dyn Fn(&str) -> Option<ParserNameBinding>,
) -> Result<Program> {
    parse_with(source, Some(lookup), ParseContext::Analysis)
}

pub(crate) struct ResolvedModifier {
    pub function: Arc<FunctionEntity>,
    pub bindings: Vec<(String, crate::semantic::NameVersion)>,
}

pub(crate) trait RuntimeParserHost {
    fn lookup(&mut self, name: &str) -> Option<ParserNameBinding>;
    fn stacked_modifier(
        &self,
        _name: &str,
    ) -> Option<(Arc<FunctionEntity>, crate::semantic::NameVersion)> {
        None
    }
    fn gerund_binding(&self, _name: &str) -> Result<Option<ParserNameBinding>> {
        Err(Error::Unsupported(
            "host does not support gerund name lookup".into(),
        ))
    }
    fn version(&self, name: &str) -> Option<crate::semantic::NameVersion>;
    /// Operands have already reduced to actual nouns; execute exactly one call.
    fn apply(&mut self, expression: Expr) -> Result<Value>;
    fn resolve_modifier(
        &mut self,
        _name: &str,
        _expected: FunctionPartOfSpeech,
    ) -> Result<ResolvedModifier> {
        Err(Error::Unsupported(
            "named modifier construction requires a runtime host".into(),
        ))
    }
    /// Return the assigned value with storage shared at the binding boundary.
    fn assign(&mut self, _name: &str, _value: JEntity) -> Result<JEntity> {
        Err(Error::Unsupported(
            "host does not support parser-time assignment".into(),
        ))
    }
}

pub(crate) fn parse_runtime_host(
    source: &str,
    host: &mut dyn RuntimeParserHost,
    capture: Option<&mut ParseCapture>,
) -> Result<Program> {
    let mut capture = capture;
    if let Some(capture) = &mut capture {
        capture.set_source(source);
    }
    parse_context(
        source,
        &mut ActionContext {
            mode: ParseContext::Runtime,
            lookup: None,
            host: Some(host),
            capture,
            modifier_snapshots: Vec::new(),
        },
    )
}

struct ActionContext<'a> {
    mode: ParseContext,
    lookup: NameLookup<'a>,
    host: Option<&'a mut dyn RuntimeParserHost>,
    capture: Option<&'a mut ParseCapture>,
    modifier_snapshots: Vec<crate::semantic::ModifierSnapshot>,
}

/// Resolve one ordinary name only when its queue entry reaches the stack.
fn resolve_stack_item(item: Item, context: &mut ActionContext<'_>) -> Result<Item> {
    let ParseValue::LookupName { name, span } = &item.value else {
        return Ok(item);
    };
    let name = name.clone();
    let span = span.clone();
    let binding = if let Some(host) = &mut context.host {
        host.lookup(&name)
    } else {
        context.lookup.and_then(|lookup| lookup(&name))
    };
    let mut resolved = match binding {
        Some(ParserNameBinding::Noun(value)) => Item::noun(
            Expr {
                span,
                kind: if context.mode == ParseContext::Runtime {
                    ExprKind::Literal(value)
                } else {
                    ExprKind::ReadName(name)
                },
            },
            0,
        ),
        Some(ParserNameBinding::AbstractNoun) if context.mode == ParseContext::Runtime => {
            return Err(
                Error::Unsupported("abstract noun requires static analysis".into())
                    .at(span)
                    .blamed_on_word(item.provenance.as_ref().unwrap().blame_word_index),
            );
        }
        Some(ParserNameBinding::AbstractNoun) => Item::noun(
            Expr {
                span,
                kind: ExprKind::ReadName(name),
            },
            0,
        ),
        None if context.lookup.is_none() && context.host.is_none() => Item::noun(
            Expr {
                span,
                kind: ExprKind::ReadName(name),
            },
            0,
        ),
        Some(ParserNameBinding::KnownModifier { function, version }) => {
            if context.mode != ParseContext::Analysis || !function.is_known_modifier() {
                return Err(Error::Unsupported(
                    "modifier snapshot has no supported static constructor semantics".into(),
                )
                .at(span));
            }
            context
                .modifier_snapshots
                .push(crate::semantic::ModifierSnapshot {
                    name,
                    version,
                    expected: function.result_pos,
                    function: function.clone(),
                    span: span.clone(),
                });
            Item::function(function).with_span(span)
        }
        Some(ParserNameBinding::Function(pos)) => {
            if pos != FunctionPartOfSpeech::Verb {
                if let Some((function, version)) = context
                    .host
                    .as_ref()
                    .and_then(|host| host.stacked_modifier(&name))
                {
                    let snapshot = crate::semantic::ModifierSnapshot {
                        name,
                        version,
                        expected: pos,
                        function: function.clone(),
                        span: span.clone(),
                    };
                    if let Some(capture) = &mut context.capture {
                        capture.events.push(CaptureEvent::ModifierStacked {
                            snapshot: snapshot.clone(),
                        });
                    }
                    context.modifier_snapshots.push(snapshot);
                    let mut stacked = Item::function(function).with_span(span);
                    stacked.provenance = item.provenance;
                    stacked.flags = item.flags;
                    return Ok(stacked);
                }
            }
            let entity = FunctionEntity::name_ref(name.clone(), pos, span.clone());
            if pos == FunctionPartOfSpeech::Verb {
                Item::verb(Verb {
                    span,
                    target: VerbTarget::Named(name),
                    entity,
                })
            } else {
                Item::function(entity)
            }
        }
        None => Item::verb(Verb {
            span: span.clone(),
            target: VerbTarget::Named(name.clone()),
            entity: FunctionEntity::name_ref(name, FunctionPartOfSpeech::Verb, span),
        }),
    };
    resolved.provenance = item.provenance;
    resolved.flags = item.flags;
    Ok(resolved)
}

/// Both modes use the same grammar; only runtime mode snapshots concrete nouns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParseContext {
    Analysis,
    Runtime,
}

type NameLookup<'a> = Option<&'a dyn Fn(&str) -> Option<ParserNameBinding>>;
fn parse_with(source: &str, lookup: NameLookup<'_>, mode: ParseContext) -> Result<Program> {
    parse_context(
        source,
        &mut ActionContext {
            mode,
            lookup,
            host: None,
            capture: None,
            modifier_snapshots: Vec::new(),
        },
    )
}

fn parse_context(source: &str, context: &mut ActionContext<'_>) -> Result<Program> {
    let mut queue = enqueue(source)?;
    if queue.is_empty() {
        return Ok(Program {
            source: source.to_owned(),
            assignment: None,
            assignment_span: None,
            expression: None,
            reductions: Vec::new(),
            assignment_source: None,
            modifier_snapshots: Vec::new(),
        });
    }

    let mut pos = 0;
    let mut reductions = Vec::new();
    let (result, _, pending_assignment) =
        expression(queue.as_mut_slice(), &mut pos, context, &mut reductions).map_err(|error| {
            let fallback = queue.get(pos).or_else(|| queue.last());
            let fallback_span = fallback
                .map(|word| word.span.clone())
                .unwrap_or(source.len()..source.len());
            let mut context = ErrorContext::phase(DiagnosticPhase::Parse).with_span(fallback_span);
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

    let (assignment, assignment_span, assignment_source) = match pending_assignment {
        Some(PendingAssignment { name, span, source }) => (Some(name), Some(span), Some(source)),
        None => (None, None, None),
    };
    Ok(Program {
        source: source.to_owned(),
        assignment,
        assignment_span,
        expression: Some(result),
        reductions,
        assignment_source,
        modifier_snapshots: std::mem::take(&mut context.modifier_snapshots),
    })
}
fn expression(
    tokens: &mut [EnqueuedWord<'_>],
    pos: &mut usize,
    context: &mut ActionContext<'_>,
    reductions: &mut Vec<ParseReduction>,
) -> Result<(Expr, usize, Option<PendingAssignment>)> {
    let mut items = Vec::new();
    let mut open_spans = Vec::new();
    let mut unexpected_close = None;
    while *pos < tokens.len() {
        let source_word = *pos;
        match &tokens[*pos].payload {
            EnqueuedPayload::Close => {
                if open_spans.pop().is_none() {
                    unexpected_close
                        .get_or_insert((tokens[*pos].span.clone(), tokens[*pos].word_index));
                }
                items.push(Item::control(ParseClass::RParen, tokens[*pos].span.clone()));
                *pos += 1;
            }
            EnqueuedPayload::Open => {
                if open_spans.len() >= MAX_EXPR_DEPTH {
                    return Err(Error::Limit);
                }
                open_spans.push((tokens[*pos].span.clone(), tokens[*pos].word_index));
                items.push(Item::control(ParseClass::LParen, tokens[*pos].span.clone()));
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
                let EnqueuedPayload::Noun(v) =
                    std::mem::replace(&mut tokens[*pos].payload, EnqueuedPayload::Open)
                else {
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
            EnqueuedPayload::Name(name) => {
                let span = tokens[*pos].span.clone();
                items.push(if tokens[*pos].flags.lookup_name {
                    Item::lookup_name((*name).to_owned(), span)
                } else {
                    Item::name_target((*name).to_owned(), span)
                });
                *pos += 1;
            }
            EnqueuedPayload::Verb(id) => {
                let span = tokens[*pos].span.clone();
                items.push(Item::verb(Verb {
                    span: span.clone(),
                    target: VerbTarget::Primitive(*id),
                    entity: FunctionEntity::primitive(*id, span),
                }));
                *pos += 1;
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
            EnqueuedPayload::Function(function) => {
                let item = if function.result_pos == FunctionPartOfSpeech::Verb {
                    Item::verb(Verb {
                        span: tokens[*pos].span.clone(),
                        target: VerbTarget::Derived,
                        entity: function.clone(),
                    })
                } else {
                    Item::function(function.clone())
                };
                items.push(item.with_span(tokens[*pos].span.clone()));
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
        let item = items.pop().expect("one item per enqueue word");
        items.push(item.with_source(&tokens[source_word]));
    }
    // Diagnose unmatched controls after reachable actions, preserving their
    // original source token. Do not replace an earlier runtime error class.
    let control_error = unexpected_close
        .map(|(span, word)| {
            Error::Syntax("unexpected )".into())
                .at(span)
                .blamed_on_word(word)
        })
        .or_else(|| {
            open_spans.last().map(|(span, word)| {
                Error::Syntax("missing )".into())
                    .at(span.clone())
                    .blamed_on_word(*word)
            })
        });
    let reduced = reduce_parse_stack_subset(items, context, reductions);
    let (mut items, assignment) = match reduced {
        Err(error) if error.kind() == "syntax error" && control_error.is_some() => {
            return Err(control_error.unwrap());
        }
        other => other?,
    };
    if let Some(error) = control_error {
        return Err(error);
    }

    if items.len() != 1 {
        let span = items
            .first()
            .map(Item::span)
            .unwrap_or_else(|| tokens.last().map(|word| word.span.clone()).unwrap_or(0..0));
        let mut error = Error::Syntax("unreduced parser stack after rows 0-8".into()).at(span);
        if let Some(provenance) = items.first().and_then(|item| item.provenance.as_ref()) {
            error = error.blamed_on_word(provenance.blame_word_index);
        }
        return Err(error);
    }

    let item = items.pop().expect("one reduced parser item");
    let span = item.span();
    if let Some(capture) = &mut context.capture {
        capture.result = item.occurrence;
        let function = match &item.value {
            ParseValue::Verb(verb) => Some(verb.entity.clone()),
            ParseValue::Function(function) => Some(function.clone()),
            _ => None,
        };
        if let Some(function) = function {
            let event = CaptureEvent::FunctionResult {
                function,
                span: span.clone(),
            };
            // Declaration of the final entity precedes its final commit in the
            // observation log; no computation or binding is replayed here.
            if matches!(
                capture.events.last(),
                Some(CaptureEvent::Commit {
                    final_assignment: true,
                    ..
                })
            ) {
                capture.events.insert(capture.events.len() - 1, event);
            } else {
                capture.events.push(event);
            }
        }
    }
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
        ParseValue::Function(entity) => Ok((
            Expr {
                span,
                kind: ExprKind::ModifierValue(entity),
            },
            0,
            assignment,
        )),
        ParseValue::LookupName { .. }
        | ParseValue::NameTarget { .. }
        | ParseValue::Control { .. } => {
            Err(Error::Syntax("unexpected parser control result".into()).at(span))
        }
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

#[cfg(test)]
mod parser_table_tests {
    use super::{ParseClass::*, ParseRow, match_parse_row};

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

    #[test]
    fn pinned_jsource_bident_dispositions_match_cf_c() {
        use super::BidentDisposition::{
            BuildDerivedModifier as D, BuildHook, ImmediateSemanticApply as I, SyntaxError as S,
        };

        let expected = [
            ((Verb, Verb), BuildHook),
            ((Verb, Noun), I),
            ((Noun, Adverb), I),
            ((Verb, Adverb), I),
            ((Noun, Conjunction), D(Adverb)),
            ((Verb, Conjunction), D(Adverb)),
            ((Adverb, Verb), D(Adverb)),
            ((Adverb, Adverb), D(Adverb)),
            ((Adverb, Conjunction), D(Adverb)),
            ((Conjunction, Noun), D(Adverb)),
            ((Conjunction, Verb), D(Adverb)),
            ((Conjunction, Adverb), D(Conjunction)),
            ((Conjunction, Conjunction), D(Conjunction)),
        ];
        for ((left, right), disposition) in expected {
            assert_eq!(super::bident_disposition(left, right), disposition);
        }
        assert_eq!(super::bident_disposition(Noun, Noun), S);
        assert_eq!(super::bident_disposition(Noun, Verb), S);
    }

    #[test]
    fn pinned_jsource_trident_dispositions_match_cf_c() {
        use super::TridentDisposition::{
            BuildDerivedModifier as D, BuildFork, ImmediateSemanticApply as I, SyntaxError as S,
        };

        let expected = [
            ((Noun, Verb, Noun), I),
            ((Verb, Verb, Verb), BuildFork),
            ((Noun, Verb, Verb), BuildFork),
            ((Adverb, Adverb, Adverb), D(Adverb)),
            ((Adverb, Adverb, Verb), D(Conjunction)),
            ((Verb, Verb, Conjunction), D(Conjunction)),
            ((Noun, Verb, Conjunction), D(Conjunction)),
            ((Adverb, Verb, Verb), D(Adverb)),
            ((Conjunction, Verb, Verb), D(Conjunction)),
            ((Conjunction, Verb, Conjunction), D(Conjunction)),
            ((Conjunction, Adverb, Adverb), D(Conjunction)),
            ((Noun, Conjunction, Noun), I),
            ((Noun, Conjunction, Verb), I),
            ((Verb, Conjunction, Noun), I),
            ((Verb, Conjunction, Verb), I),
            ((Noun, Conjunction, Adverb), D(Adverb)),
            ((Verb, Conjunction, Adverb), D(Adverb)),
            ((Noun, Conjunction, Conjunction), D(Conjunction)),
            ((Verb, Conjunction, Conjunction), D(Conjunction)),
            ((Adverb, Conjunction, Noun), D(Adverb)),
            ((Adverb, Conjunction, Verb), D(Adverb)),
            ((Adverb, Conjunction, Adverb), D(Conjunction)),
            ((Adverb, Conjunction, Conjunction), D(Conjunction)),
            ((Conjunction, Conjunction, Verb), D(Conjunction)),
            ((Conjunction, Conjunction, Noun), D(Conjunction)),
            ((Conjunction, Conjunction, Adverb), D(Conjunction)),
            ((Conjunction, Conjunction, Conjunction), D(Conjunction)),
        ];
        for ((first, second, third), disposition) in expected {
            assert_eq!(
                super::trident_disposition(first, second, third),
                disposition
            );
        }
        assert_eq!(super::trident_disposition(Verb, Noun, Verb), S);
        assert_eq!(super::trident_disposition(Noun, Noun, Noun), S);
    }
}

#[cfg(test)]
mod stack_entry_tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn names_resolve_right_to_left_and_nouns_snapshot_at_each_stack_entry() {
        let seen = RefCell::new(Vec::new());
        let program = parse_runtime("x+x", &|name| {
            seen.borrow_mut().push(name.to_owned());
            let value = Value::scalar(seen.borrow().len() as i64);
            Some(ParserNameBinding::Noun(value))
        })
        .unwrap();
        assert_eq!(*seen.borrow(), vec!["x", "x"]);
        let ExprKind::Dyad { left, right, .. } = program.expression.unwrap().kind else {
            panic!()
        };
        let ExprKind::Literal(left) = left.kind else {
            panic!()
        };
        let ExprKind::Literal(right) = right.kind else {
            panic!()
        };
        assert_eq!(left.json(), Value::scalar(2).json());
        assert_eq!(right.json(), Value::scalar(1).json());

        seen.borrow_mut().clear();
        parse_analysis("a+b*c", &|name| {
            seen.borrow_mut().push(name.to_owned());
            Some(ParserNameBinding::AbstractNoun)
        })
        .unwrap();
        assert_eq!(*seen.borrow(), vec!["c", "b", "a"]);
    }

    #[test]
    fn constructor_failure_stops_lookup_of_unvisited_left_names() {
        let seen = RefCell::new(Vec::new());
        let error = parse_runtime("unvisited (+\"'bad') right", &|name| {
            seen.borrow_mut().push(name.to_owned());
            Some(ParserNameBinding::Noun(Value::scalar(1)))
        })
        .unwrap_err();
        assert_eq!(error.kind(), "domain error");
        assert_eq!(*seen.borrow(), vec!["right"]);
        assert_eq!(error.context().unwrap().blame_word_index, Some(3));
    }

    #[test]
    fn assignment_target_is_not_looked_up_and_named_modifiers_keep_pos() {
        let seen = RefCell::new(Vec::new());
        let program = parse_analysis("out=:f adv x", &|name| {
            seen.borrow_mut().push(name.to_owned());
            Some(match name {
                "x" => ParserNameBinding::AbstractNoun,
                "adv" => ParserNameBinding::KnownModifier {
                    function: FunctionEntity::primitive_adverb(
                        crate::primitive::AdverbId::Insert,
                        0..0,
                    ),
                    version: crate::semantic::NameVersion(1),
                },
                "f" => ParserNameBinding::Function(FunctionPartOfSpeech::Verb),
                _ => panic!("unexpected lookup: {name}"),
            })
        })
        .unwrap();
        assert_eq!(*seen.borrow(), vec!["x", "adv", "f"]);
        assert_eq!(program.assignment.as_deref(), Some("out"));
        let ExprKind::Monad { verb, .. } = program.expression.unwrap().kind else {
            panic!()
        };
        assert_eq!(
            verb.entity.head,
            FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert)
        );
        assert_eq!(program.modifier_snapshots.len(), 1);
        assert_eq!(program.modifier_snapshots[0].name, "adv");
        assert_eq!(verb.entity.result_pos, FunctionPartOfSpeech::Verb);
    }
}

#[cfg(test)]
mod runtime_action_tests {
    use super::*;
    struct Host {
        log: Vec<String>,
        left: i64,
    }
    impl RuntimeParserHost for Host {
        fn lookup(&mut self, name: &str) -> Option<ParserNameBinding> {
            self.log.push(format!("lookup:{name}"));
            Some(ParserNameBinding::Noun(Value::scalar(if name == "left" {
                self.left
            } else {
                3
            })))
        }
        fn version(&self, _: &str) -> Option<crate::semantic::NameVersion> {
            None
        }
        fn apply(&mut self, expression: Expr) -> Result<Value> {
            let ExprKind::Dyad { verb, left, right } = expression.kind else {
                panic!()
            };
            let left = completed_noun(*left, "test left")?;
            let right = completed_noun(*right, "test right")?;
            let VerbTarget::Primitive(id) = verb.target else {
                panic!()
            };
            self.log.push(format!("apply:{}", id.spelling()));
            self.left = 8; // Proves subsequent parser lookup sees host state now.
            crate::kernels::dyad(id.spelling(), left, right)
        }
    }
    #[test]
    fn runtime_rows_invoke_before_later_lookup_and_return_a_completed_noun() {
        let mut host = Host {
            log: Vec::new(),
            left: 100,
        };
        let program = parse_runtime_host("left+(right*2)", &mut host, None).unwrap();
        assert_eq!(
            host.log,
            ["lookup:right", "apply:*", "lookup:left", "apply:+"]
        );
        let ExprKind::Literal(value) = program.expression.unwrap().kind else {
            panic!("runtime result must not replay an AST")
        };
        assert_eq!(value.int_at(0).unwrap(), 14);
    }
}

#[cfg(test)]
mod modifier_storage_tests {
    use super::*;
    use crate::{storage::CpuStorage, value::Data};

    #[test]
    fn reused_grouped_noun_shares_owned_payload_without_copy_and_outlives_original() {
        let value = crate::Value::ints([65_536], (0..65_536).collect()).unwrap();
        let Data::Int(storage) = &value.data else {
            panic!();
        };
        let pointer = storage.as_slice().as_ptr();
        let literal = Expr {
            span: 0..1,
            kind: ExprKind::Literal(value),
        };
        let item = Item::noun(
            Expr {
                span: 0..1,
                kind: ExprKind::Group(Box::new(literal)),
            },
            1,
        );
        let first = share_modifier_input(item);
        let second = first.clone();
        let first = completed_noun(first.into_noun().unwrap().0, "test").unwrap();
        let second = completed_noun(second.into_noun().unwrap().0, "test").unwrap();
        let (Data::Int(CpuStorage::Shared(a)), Data::Int(CpuStorage::Shared(b))) =
            (&first.data, &second.data)
        else {
            panic!();
        };
        assert!(std::sync::Arc::ptr_eq(a, b));
        assert_eq!(a.as_ptr(), pointer);
        drop(first);
        assert_eq!(second.int_at(65_535).unwrap(), 65_535);
    }
}

#[cfg(test)]
mod gerund_ar_tests {
    use super::*;

    #[test]
    fn constructor_call_observes_updated_host_and_static_path_never_executes() {
        struct Host {
            calls: usize,
        }
        impl RuntimeParserHost for Host {
            fn lookup(&mut self, _: &str) -> Option<ParserNameBinding> {
                None
            }
            fn version(&self, _: &str) -> Option<crate::semantic::NameVersion> {
                None
            }
            fn gerund_binding(&self, _: &str) -> Result<Option<ParserNameBinding>> {
                Ok(Some(if self.calls == 0 {
                    ParserNameBinding::Function(FunctionPartOfSpeech::Verb)
                } else {
                    ParserNameBinding::Noun(Value::scalar(2))
                }))
            }
            fn apply(&mut self, expression: Expr) -> Result<Value> {
                assert!(matches!(expression.kind, ExprKind::Monad { .. }));
                self.calls += 1;
                Ok(Value::scalar(2))
            }
        }
        let call = ar("4", vec![text("+"), noun(Value::scalar(1))]);
        assert_eq!(
            decode_gerund_ar(&call, 0..1, 0, ConstructionNames::default())
                .err()
                .unwrap()
                .kind(),
            "unsupported"
        );
        let value = ar("3", vec![call, text("aftercall"), text("")]);
        let mut host = Host { calls: 0 };
        {
            let bridge = std::cell::RefCell::new(&mut host as &mut dyn RuntimeParserHost);
            let names = ConstructionNames {
                host: Some(&bridge),
                ..ConstructionNames::default()
            };
            assert_eq!(
                decode_gerund_ar(&value, 0..1, 0, names)
                    .err()
                    .unwrap()
                    .kind(),
                "syntax error"
            );
        }
        assert_eq!(host.calls, 1);
    }
    use crate::{storage::CpuStorage, value::Data};

    fn text(s: &str) -> Value {
        Value::new(
            [s.len()],
            Data::Char(CpuStorage::new(s.as_bytes().to_vec())),
        )
        .unwrap()
    }
    fn boxes(values: Vec<Value>) -> Value {
        Value::new(
            [values.len()],
            Data::Boxed(CpuStorage::new(
                values
                    .into_iter()
                    .map(|v| Arc::new(v.into_shared()))
                    .collect(),
            )),
        )
        .unwrap()
    }
    fn ar(head: &str, args: Vec<Value>) -> Value {
        boxes(vec![text(head), boxes(args)])
    }
    fn noun(value: Value) -> Value {
        boxes(vec![text("0"), value])
    }

    fn function(item: Item) -> Arc<FunctionEntity> {
        match item.value {
            ParseValue::Verb(verb) => verb.entity,
            ParseValue::Function(function) => function,
            _ => panic!(),
        }
    }

    #[test]
    fn serialized_core_entities_use_parser_constructor_dags_and_actual_pos() {
        for (value, expected, pos) in [
            (
                boxes(vec![text("+")]),
                FunctionHead::PrimitiveVerb(crate::primitive::PrimitiveId::Add),
                FunctionPartOfSpeech::Verb,
            ),
            (
                ar("/", vec![text("+")]),
                FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert),
                FunctionPartOfSpeech::Verb,
            ),
            (
                ar("\"", vec![text("+"), noun(Value::scalar(1))]),
                FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Rank),
                FunctionPartOfSpeech::Verb,
            ),
            (
                ar("@:", vec![text("+"), text("-")]),
                FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Atop),
                FunctionPartOfSpeech::Verb,
            ),
            (
                ar("2", vec![text("+"), text("-")]),
                FunctionHead::Hook,
                FunctionPartOfSpeech::Verb,
            ),
            (
                ar("3", vec![noun(Value::scalar(7)), text("+"), text("*")]),
                FunctionHead::Fork,
                FunctionPartOfSpeech::Verb,
            ),
            (
                ar("4", vec![text("/"), text("/")]),
                FunctionHead::ModifierTrain,
                FunctionPartOfSpeech::Adverb,
            ),
        ] {
            let function =
                function(decode_gerund_ar(&value, 5..9, 0, ConstructionNames::default()).unwrap());
            assert_eq!(function.head, expected);
            assert_eq!(function.result_pos, pos);
        }
        let modifier = ar("4", vec![text("/"), text("/")]);
        let applied = boxes(vec![modifier, boxes(vec![text("+")])]);
        let function =
            function(decode_gerund_ar(&applied, 5..9, 0, ConstructionNames::default()).unwrap());
        let FunctionOperand::Function(child) = &function.operands[0] else {
            panic!();
        };
        assert_eq!(
            function.head,
            FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Insert)
        );
        assert_eq!(child.head, function.head);
    }

    #[test]
    fn serialized_noun_shares_payload_and_recursion_is_bounded() {
        let value = Value::ints([65_536], (0..65_536).collect())
            .unwrap()
            .into_shared();
        let pointer = match &value.data {
            Data::Int(v) => v.as_slice().as_ptr(),
            _ => panic!(),
        };
        let representation = noun(value);
        let decoded =
            decode_gerund_ar(&representation, 2..6, 0, ConstructionNames::default()).unwrap();
        let noun = completed_noun(decoded.into_noun().unwrap().0, "test").unwrap();
        assert_eq!(
            match &noun.data {
                Data::Int(v) => v.as_slice().as_ptr(),
                _ => panic!(),
            },
            pointer
        );
        drop(representation);
        assert_eq!(noun.int_at(65_535).unwrap(), 65_535);
        let mut deep = text("+");
        for _ in 0..=MAX_EXPR_DEPTH {
            deep = boxes(vec![deep]);
        }
        assert_eq!(
            decode_gerund_ar(&deep, 0..1, 0, ConstructionNames::default())
                .err()
                .unwrap()
                .kind(),
            "limit error"
        );
    }
    #[test]
    fn decoded_name_reference_keeps_actual_pos_without_capturing_a_verb_value() {
        let lookup = |name: &str| {
            if name == "fn" {
                Some(ParserNameBinding::Function(FunctionPartOfSpeech::Verb))
            } else {
                None
            }
        };
        let names = ConstructionNames {
            lookup: Some(&lookup),
            host: None,
            observations: None,
            row: None,
        };
        let decoded = function(decode_gerund_ar(&text("fn"), 3..8, 0, names).unwrap());
        assert_eq!(decoded.head, FunctionHead::NameRef("fn".into()));
        assert_eq!(decoded.result_pos, FunctionPartOfSpeech::Verb);
        assert!(decoded.operands.is_empty());
        let noun_lookup = |_: &str| Some(ParserNameBinding::Noun(Value::scalar(1)));
        let names = ConstructionNames {
            lookup: Some(&noun_lookup),
            host: None,
            observations: None,
            row: None,
        };
        assert_eq!(
            audit_gerund(&boxes(vec![text("fn")]), 3..8, 0, names)
                .unwrap_err()
                .kind(),
            "domain error"
        );
        assert_eq!(decoded.result_pos, FunctionPartOfSpeech::Verb);
        let fork = function(
            decode_gerund_ar(
                &ar("3", vec![text("fn"), text("+"), text("-")]),
                3..8,
                0,
                names,
            )
            .unwrap(),
        );
        let FunctionOperand::Noun { value, .. } = &fork.operands[0] else {
            panic!()
        };
        assert_eq!(value.int_at(0).unwrap(), 1);
        let abstract_lookup = |_: &str| Some(ParserNameBinding::AbstractNoun);
        let names = ConstructionNames {
            lookup: Some(&abstract_lookup),
            ..ConstructionNames::default()
        };
        assert_eq!(
            decode_gerund_ar(
                &ar("3", vec![text("fn"), text("+"), text("-")]),
                3..8,
                0,
                names
            )
            .err()
            .unwrap()
            .kind(),
            "unsupported"
        );
        assert_eq!(
            decode_gerund_ar(&text("fn"), 3..8, 0, ConstructionNames::default())
                .err()
                .unwrap()
                .kind(),
            "unsupported"
        );
    }
}

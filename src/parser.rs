//! J parser queue/stack reductions and parser-time construction.
//! Consumes typed enqueue records; produces target-independent Semantic IR.
use crate::frontend_context::{
    FrontendContext, ItemId, ItemProducer, ItemRecord, NameEvidence, NamePolicy, NameUseId,
    NameUseRecord, NodeKind, ParseStep, PendingAction, ReductionId, ReductionRecord, WordId,
    WordRecord,
};
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
    pub noun_target: bool,
    /// Commit.value denotes the whole RHS. A multiple assignment selects a
    /// leading-axis item (None for scalar extension), then opens it once.
    pub selection: Option<AssignmentSelection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssignmentSelection {
    pub item: Option<usize>,
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

fn train_fork(f: Verb, g: Verb, h: Verb, names: ConstructionNames<'_, '_>) -> Result<Verb> {
    let capped = names.fork_cap(&f.entity)?;
    let span = f.span.start..h.span.end;
    let mut verb = Verb {
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
    };
    Arc::get_mut(&mut verb.entity)
        .expect("fresh fork")
        .fork_semantics = Some(if capped {
        crate::semantic::ForkSemantics::Capped
    } else {
        crate::semantic::ForkSemantics::Ordinary
    });
    Ok(verb)
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
    let operand = CompletedParseResult::from_noun(noun, 0, "runtime-dependent noun-left fork")?
        .into_operand();
    let span = noun_span.start..h.span.end;
    Ok(Verb {
        span: span.clone(),
        target: VerbTarget::Derived,
        entity: FunctionEntity::derived(
            FunctionHead::Fork,
            FunctionPartOfSpeech::Verb,
            span,
            vec![
                operand,
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
    if matches!(operator.head, FunctionHead::TakeName { .. }) {
        return Err(Error::Unsupported(
            "deferred modifier abandon requires ordered NAME effect IR".into(),
        )
        .at(span));
    }
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
    fork_reads: Option<&'a std::cell::RefCell<Vec<crate::semantic::NameUse>>>,
}

fn construction_host<'h>(
    host: &'h mut Option<&mut dyn RuntimeParserHost>,
) -> Option<std::cell::RefCell<&'h mut dyn RuntimeParserHost>> {
    host.as_mut()
        .map(|host| std::cell::RefCell::new(&mut **host as &mut dyn RuntimeParserHost))
}

impl ConstructionNames<'_, '_> {
    fn fork_cap(self, first: &FunctionEntity) -> Result<bool> {
        if matches!(first.head, FunctionHead::TakeName { .. }) {
            return Err(Error::Unsupported(
                "fork cap inspection requires the deferred abandon value".into(),
            ));
        }
        if matches!(
            first.head,
            FunctionHead::PrimitiveVerb(crate::primitive::PrimitiveId::Cap)
        ) {
            return Ok(true);
        }
        let FunctionHead::NameRef(name) = &first.head else {
            return Ok(false);
        };
        // cf.c::jtcap inspects only this binding's head, never an alias chain.
        let binding = if let Some(host) = self.host {
            host.borrow().fork_cap_binding(name)?
        } else {
            match self.lookup.and_then(|lookup| lookup(name)) {
                Some(ParserNameBinding::KnownVerb { function, version }) => Some((
                    matches!(
                        function.head,
                        FunctionHead::PrimitiveVerb(crate::primitive::PrimitiveId::Cap)
                    ),
                    version,
                )),
                _ => None,
            }
        };
        let (capped, version) = binding.ok_or_else(|| {
            Error::Unsupported("fork construction needs a direct first-name binding witness".into())
        })?;
        let read = crate::semantic::NameUse {
            name: name.clone(),
            version,
            span: first.span.clone(),
        };
        if let Some(reads) = self.fork_reads {
            reads.borrow_mut().push(read.clone());
        }
        if let Some(observations) = self.observations {
            observations
                .borrow_mut()
                .push(CaptureEvent::ForkNameResolved {
                    row: self.row.expect("constructor row"),
                    read,
                    capped,
                });
        }
        Ok(capped)
    }

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
    fn apply_definition(
        self,
        operator: Arc<FunctionEntity>,
        left: Item,
        right: Option<Item>,
        span: std::ops::Range<usize>,
    ) -> Result<Item> {
        let Some(host) = self.host else {
            return Err(Error::Unsupported(
                "explicit modifier invocation requires runtime host".into(),
            ));
        };
        let left = CompletedParseResult::from_item(left, "explicit left operand")?.into_operand();
        let right = right
            .map(|item| {
                CompletedParseResult::from_item(item, "explicit right operand")
                    .map(CompletedParseResult::into_operand)
            })
            .transpose()?;
        if let Some(observations) = self.observations {
            observations
                .borrow_mut()
                .push(CaptureEvent::ExplicitModifierApply {
                    row: self.row.expect("explicit constructor row"),
                    function: operator.clone(),
                    span: span.clone(),
                });
        }
        let entity = host
            .borrow_mut()
            .apply_definition(operator, left, right)
            .map_err(|error| {
                // Body spans index DefinitionCode.body, not the caller's source.
                // Keep bounded operation details, but locate this failure at the
                // outer invocation until diagnostics support separate source frames.
                let mut context = error.context().cloned().unwrap_or_default();
                context.span = Some(span.clone());
                context.blame_word_index = None;
                error.into_unlocated().with_context(context)
            })?;
        match entity {
            JEntity::Noun(value) => {
                CompletedParseResult::noun(value.into_shared(), span, 0).into_item()
            }
            JEntity::Function(function) => {
                CompletedParseResult::function(function, span, VerbTarget::Derived).into_item()
            }
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
            origin: None,
            span: right_span,
            kind: ExprKind::Literal(right),
        });
        let expression = Expr {
            origin: None,
            span: span.clone(),
            kind: if let Some((value, noun_span)) = left {
                ExprKind::Dyad {
                    verb,
                    left: Box::new(Expr {
                        origin: None,
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
        result
            .and_then(|value| CompletedParseResult::noun(value.into_shared(), span, 0).into_item())
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
    if let FunctionHead::VocabularyPrimitive(id) = operator.head {
        return Err(Error::Unsupported(format!(
            "core modifier {} construction",
            id.spelling()
        )));
    }
    if matches!(
        operator.head,
        FunctionHead::PrimitiveAdverb(crate::primitive::AdverbId::Ident)
    ) {
        // v.c::jtlev returns the existing entity. Operand lookup/reduction has
        // already occurred; do not execute a selected verb or drop noun effects.
        return Ok(left.with_span(span));
    }
    if matches!(operator.head, FunctionHead::ExplicitDefinition(_)) {
        return names.apply_definition(operator, left, None, span);
    }
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
        PrimitiveSemanticId::Vocabulary(id) => {
            let function = FunctionEntity::derived(
                FunctionHead::VocabularyPrimitive(id),
                id.part_of_speech().into(),
                span.clone(),
                Vec::new(),
            );
            if function.result_pos == FunctionPartOfSpeech::Verb {
                Item::verb(Verb {
                    span,
                    target: VerbTarget::Derived,
                    entity: function,
                })
            } else {
                Item::function(function)
            }
        }
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
        if queue[0].flags.name_form.is_locative() {
            return Err(Error::Unsupported("J locative gerund lookup".into()));
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
                Some(ParserNameBinding::KnownVerb { function, .. })
                | Some(ParserNameBinding::KnownModifier { function, .. }) => {
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
                        origin: None,
                        span,
                        kind: ExprKind::Literal(value.into_shared()),
                    },
                    0,
                ));
            }
            Some(ParserNameBinding::AbstractNoun) => {
                return Ok(Item::noun(
                    Expr {
                        origin: None,
                        span,
                        kind: ExprKind::ReadName(spelling.into()),
                    },
                    0,
                ));
            }
            None => FunctionPartOfSpeech::Verb,
            Some(ParserNameBinding::Function(pos)) => pos,
            Some(ParserNameBinding::KnownVerb { function, .. })
            | Some(ParserNameBinding::KnownModifier { function, .. }) => function.result_pos,
        };
        if let Some(function) = names
            .host
            .and_then(|host| host.borrow().operand_function(spelling))
        {
            return CompletedParseResult::function(function, span, VerbTarget::Derived).into_item();
        }
        let ranks = if let Some(host) = names.host {
            host.borrow().function_name_ranks(spelling)
        } else {
            match names.lookup.and_then(|lookup| lookup(spelling)) {
                Some(ParserNameBinding::KnownVerb { function, .. }) => function.innate_ranks(),
                None if names.lookup.is_some() => Some([63; 3]),
                _ => None,
            }
        };
        let entity = FunctionEntity::with_name_ranks(
            FunctionEntity::name_ref(spelling.into(), pos, span.clone()),
            ranks,
        );
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
    mut value: &Value,
    span: std::ops::Range<usize>,
    mut depth: usize,
    names: ConstructionNames<'_, '_>,
) -> Result<Item> {
    use crate::value::Data;
    // Singleton boxed headers only transport their decoded entity. Peel this
    // tail path iteratively so retained provenance cannot inflate recursive
    // stack usage; keep exactly the same depth/rank/empty checks and no lookup.
    loop {
        if depth >= MAX_EXPR_DEPTH {
            return Err(Error::Limit);
        }
        let Data::Boxed(fields) = &value.data else {
            break;
        };
        if value.shape.len() > 1 || value.len() != 1 {
            break;
        }
        let first = &fields[0];
        if first.is_empty() {
            return Err(Error::Length);
        }
        if !matches!(first.data, Data::Boxed(_)) {
            break;
        }
        value = first;
        depth += 1;
    }
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
        Entity(Box<Item>),
        Noun,
        Hook,
        Fork,
        Modifier,
    }
    let head = if matches!(first.data, Data::Boxed(_)) {
        Head::Entity(Box::new(decode_gerund_ar(
            first,
            span.clone(),
            depth + 1,
            names,
        )?))
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
            _ => Head::Entity(Box::new(gerund_primitive(spelling, span.clone())?)),
        }
    };
    if fields.len() == 2 && matches!(head, Head::Noun) {
        return Ok(Item::noun(
            Expr {
                origin: None,
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
            let operator = *operator;
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
            origin: expr.origin,
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
                names,
            )?)
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

/// A static constructor cannot discard an unevaluated noun expression: its
/// errors/effects precede selection. Runtime nouns already have literal payloads.
fn verify_discarded_selector_operand(item: &Item) -> Result<()> {
    if let ParseValue::Noun(expr, _) = &item.value {
        let mut completed = expr;
        while let ExprKind::Group(inner) = &completed.kind {
            completed = inner;
        }
        if !matches!(completed.kind, ExprKind::Literal(_)) {
            return Err(Error::Unsupported(
                "selector discarded noun requires semantic reduction graph".into(),
            ));
        }
    }
    Ok(())
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
    if let FunctionHead::VocabularyPrimitive(id) = operator.head {
        return Err(Error::Unsupported(format!(
            "core modifier {} construction",
            id.spelling()
        )));
    }
    match operator.head {
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Lev) => {
            verify_discarded_selector_operand(&right)?;
            return Ok(left.with_span(span));
        }
        FunctionHead::PrimitiveConjunction(crate::primitive::ConjunctionId::Dex) => {
            verify_discarded_selector_operand(&left)?;
            return Ok(right.with_span(span));
        }
        _ => {}
    }
    if matches!(operator.head, FunctionHead::ExplicitDefinition(_)) {
        return names.apply_definition(operator, left, Some(right), span);
    }
    if let FunctionHead::DefinitionConstructor(origin) = &operator.head {
        let (left, _) = left.into_noun().ok_or(Error::Domain)?;
        let (right, _) = right.into_noun().ok_or(Error::Domain)?;
        let mode = CompletedParseResult::from_noun(left, 0, "computed definition mode")?
            .into_noun_value()?;
        let body = CompletedParseResult::from_noun(right, 0, "computed definition body")?
            .into_noun_value()?;
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
        let code = crate::definition_code::compile_with_origin(
            &origin.source,
            &origin.input,
            &origin.primitives,
            origin.origin.clone(),
        )?;
        let result_pos = code.result_pos;
        let function = FunctionEntity::derived(
            FunctionHead::ExplicitDefinition(code),
            result_pos,
            span.clone(),
            Vec::new(),
        );
        return CompletedParseResult::function(function, span, VerbTarget::Derived).into_item();
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
                origin: None,
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
    let right_span = match &right.value {
        ParseValue::Noun(expr, _) => {
            if matches!(primitive_id, Some(crate::primitive::ConjunctionId::Atop)) {
                return Err(Error::Domain);
            }
            expr.span.clone()
        }
        ParseValue::Verb(_) => right.span(),
        _ => return Err(Error::Syntax("invalid conjunction right operand".into())),
    };
    let mut right =
        CompletedParseResult::from_item(right, "runtime-dependent conjunction noun operand")?;
    // Preserve the constructor's original Expr provenance, independently of
    // parser reinsertion overrides. Audit before inspecting any left operand.
    right.span = right_span;
    let ranks = match &right.entity {
        JEntity::Noun(value) => Some(rank_noun_contract(value)?),
        JEntity::Function(function)
            if primitive_id == Some(crate::primitive::ConjunctionId::Rank) =>
        {
            Some(function.innate_ranks().ok_or_else(|| {
                Error::Unsupported(
                    "verb-valued rank construction needs a stacked innate-rank witness".into(),
                )
            })?)
        }
        JEntity::Function(_) => None,
    };
    let right_operand = right.into_operand();

    let left_span = match &left.value {
        ParseValue::Verb(_) => left.span(),
        ParseValue::Noun(expr, _) => {
            if !matches!(primitive_id, Some(crate::primitive::ConjunctionId::Rank)) {
                return Err(Error::Domain);
            }
            expr.span.clone()
        }
        _ => return Err(Error::Syntax("invalid conjunction left operand".into())),
    };
    let mut left =
        CompletedParseResult::from_item(left, "runtime-dependent noun-left rank operand")?;
    left.span = left_span;
    let mut decoded = None;
    if let JEntity::Noun(value) = &left.entity {
        if value.shape.len() == 1
            && matches!(value.data, crate::value::Data::Boxed(_))
            && ranks != Some([63; 3])
        {
            match audit_gerund(value, left.span.clone(), depth + 1, names) {
                Ok(functions) => decoded = Some(functions),
                Err(error) if error.kind() == "unsupported" => return Err(error),
                // cr.c suppresses failed fx audits and uses the noun itself.
                Err(_) => {}
            }
        }
    }
    let left_operand = left.into_operand();
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
    name: Option<String>,
    noun: Option<crate::semantic::NounAssignment>,
    span: std::ops::Range<usize>,
    source: AssignmentSource,
}

fn string_assignment_names(target: &Expr) -> Result<Vec<String>> {
    let value = completed_noun(target.clone(), "computed assignment target")?;
    literal_assignment_names(&value)
}

pub(crate) fn literal_assignment_names(value: &Value) -> Result<Vec<String>> {
    let crate::Data::Char(bytes) = value.data() else {
        return Err(Error::Unsupported(
            "non-character/boxed assignment target".into(),
        ));
    };
    if value.shape().len() > 1 {
        return Err(Error::Rank);
    }
    if bytes.first() == Some(&b'`') {
        return Err(Error::Unsupported(
            "atomic-representation assignment".into(),
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Error::IllFormedName)?;
    Ok(crate::tokenizer::word_texts(text)?
        .into_iter()
        .map(str::to_owned)
        .collect())
}

fn assignment_item(value: &Value, item: Option<usize>) -> Result<Value> {
    let selected = if let Some(index) = item {
        let shape = &value.shape()[1..];
        let atoms = crate::value::count(shape)?;
        value.select(shape, index * atoms..(index + 1) * atoms)?
    } else {
        value.clone()
    };
    crate::kernels::monad(">", selected)
}

fn reduce_parse_stack_subset(
    mut queue: Vec<Item>,
    context: &mut ActionContext<'_>,
    reductions: &mut Vec<ParseReduction>,
) -> Result<(Vec<Item>, Option<PendingAssignment>)> {
    let mut stack = Vec::<Item>::new();
    let mut assignment = None;

    while let Some(item) = queue.pop() {
        let queued_id = item.frontend_id();
        let name = match &item.value {
            ParseValue::LookupName { name, .. } => Some(name.clone()),
            _ => None,
        };
        let lookup_before = name
            .as_deref()
            .filter(|_| context.frontend.is_some())
            .and_then(|name| {
                context
                    .host
                    .as_ref()
                    .and_then(|host| host.lookup_observation(name))
            });
        let mut item = resolve_stack_item(item, context)?;
        // A successful named lookup can create its starting locale. Preserve pre-read
        // observations when available (especially abandon); otherwise observe that new scope.
        let lookup_before = lookup_before.or_else(|| {
            name.as_deref()
                .filter(|_| context.frontend.is_some())
                .and_then(|name| {
                    context
                        .host
                        .as_ref()
                        .and_then(|host| host.lookup_observation(name))
                })
        });
        if let Some(trace) = &mut context.frontend {
            let queued = queued_id.expect("queued frontend occurrence");
            if name.is_some() {
                let id = NameUseId(trace.name_uses.len());
                let policy = context.last_name_policy.expect("NAME resolution policy");
                let kind = (item.class == ParseClass::Noun).then_some(
                    if policy == NamePolicy::CaptureAndAbandon {
                        NodeKind::TakeName(id)
                    } else {
                        NodeKind::ReadNoun(id)
                    },
                );
                item.record_frontend(trace, ItemProducer::NameUse(id), kind, None);
                trace.name_uses.push(NameUseRecord {
                    word: WordId(trace.items[queued.0].word_range.start),
                    input: queued,
                    output: item.frontend_id().unwrap(),
                    result_class: item.class,
                    policy,
                    resolution: match (policy, item.class) {
                        (
                            NamePolicy::CaptureAtRead | NamePolicy::CaptureAndAbandon,
                            ParseClass::Noun,
                        ) => crate::frontend_context::NameResolution::NounValue,
                        (NamePolicy::CaptureAtRead | NamePolicy::CaptureAndAbandon, _) => {
                            crate::frontend_context::NameResolution::FunctionValue
                        }
                        _ => crate::frontend_context::NameResolution::FunctionReference,
                    },
                    binding_version: context.last_lookup_version,
                    lookup: lookup_before,
                    evidence: if context.host.is_some() {
                        NameEvidence::RuntimeClass
                    } else if context.lookup.is_some() {
                        NameEvidence::CatalogClass
                    } else {
                        NameEvidence::DiagnosticAssumption
                    },
                });
            }
            trace.steps.push(ParseStep::Stack {
                queued,
                resolved: item.frontend_id().expect("stack frontend occurrence"),
            });
        }
        let version = name.as_ref().and(context.last_lookup_version);
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
        let mut mark = Item::mark(0);
        if let Some(trace) = &mut context.frontend {
            mark.record_frontend(trace, ItemProducer::FrontMark, None, None);
            trace
                .steps
                .push(ParseStep::FrontMark(mark.frontend_id().unwrap()));
        }
        stack.insert(0, mark);
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
        let frontend_window =
            std::array::from_fn(|i| stack.get(i).and_then(|item| item.frontend_id()));
        let frontend_inputs: Vec<_> = stack[start..start + count]
            .iter()
            .filter_map(|item| item.frontend_id())
            .collect();
        if let Some(trace) = &mut context.frontend {
            trace.pending = Some(PendingAction {
                row,
                window: frontend_window,
            });
        }
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
            if let Some(trace) = &mut context.frontend {
                let reduction = ReductionId(trace.reductions.len());
                let node = |index: usize| {
                    trace.items[frontend_inputs[index].0]
                        .semantic
                        .expect("semantic parser operand")
                };
                let (kind, alias) = match row {
                    ParseRow::MonadEdge | ParseRow::MonadVVN => (
                        Some(NodeKind::Monad {
                            function: node(0),
                            argument: node(1),
                        }),
                        None,
                    ),
                    ParseRow::DyadNVN => (
                        Some(NodeKind::Dyad {
                            function: node(1),
                            left: node(0),
                            right: node(2),
                        }),
                        None,
                    ),
                    ParseRow::Parenthesis => (None, Some(node(1))),
                    ParseRow::Assignment => (
                        Some(NodeKind::WriteName {
                            target: frontend_inputs[0],
                            copula: WordId(trace.items[frontend_inputs[1].0].word_range.start),
                            value: node(2),
                        }),
                        None,
                    ),
                    _ => (
                        Some(NodeKind::Construct {
                            row,
                            inputs: frontend_inputs
                                .iter()
                                .filter_map(|item| trace.items[item.0].semantic)
                                .collect(),
                            function: result.value.function_entity().cloned(),
                        }),
                        None,
                    ),
                };
                result.record_frontend(trace, ItemProducer::Reduction(reduction), kind, alias);
                trace.reductions.push(ReductionRecord {
                    row,
                    window: frontend_window,
                    consumed: frontend_inputs,
                    produced: result.frontend_id().unwrap(),
                });
                trace.steps.push(ParseStep::Reduce(reduction));
                trace.pending = None;
            }
            let selected_input = is_construction.then_some(result.occurrence).flatten();
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
                    if let Some(function) = result.value.function_entity() {
                        capture.events.push(CaptureEvent::ConstructionSuccess {
                            row,
                            function: function.clone(),
                            span: reduction_span.clone(),
                        });
                    } else {
                        let ParseValue::Noun(expr, _) = &result.value else {
                            unreachable!()
                        };
                        let mut completed = expr;
                        while let ExprKind::Group(inner) = &completed.kind {
                            completed = inner;
                        }
                        let ExprKind::Literal(value) = &completed.kind else {
                            unreachable!()
                        };
                        let id = capture.next();
                        result.occurrence = Some(id);
                        capture.events.push(CaptureEvent::ConstructionNounSuccess {
                            row,
                            id,
                            selected_input,
                            facts: crate::j_graph_ir::GraphFacts::of(value),
                            span: reduction_span.clone(),
                        });
                    }
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
            if !reduced {
                if let Some(trace) = &mut context.frontend {
                    trace.pending = None;
                }
            }
            return Ok(());
        }
    }
}

fn runtime_noun(expression: Expr, height: usize, context: &mut ActionContext<'_>) -> Result<Item> {
    if let Some(host) = &mut context.host {
        let span = expression.span.clone();
        let value = host.apply(expression)?;
        CompletedParseResult::noun(value, span, height).into_item()
    } else {
        // Analysis keeps computation structure without invoking the host.
        Ok(Item::noun(expression, height))
    }
}

fn apply_parse_row(
    row: ParseRow,
    stack: &mut Vec<Item>,
    assignment: &mut Option<PendingAssignment>,
    queue_exhausted: bool,
    context: &mut ActionContext<'_>,
) -> Result<bool> {
    // C's by-value abandon path can retain nameless conjunction tagging.
    // Transport is supported, but consuming that transient stack value is
    // not ordinary conjunction application. Keep this boundary after lookup
    // and deletion, and after any nested assignment that already committed.
    if !matches!(row, ParseRow::Assignment | ParseRow::Parenthesis) {
        let range = match row {
            ParseRow::MonadEdge | ParseRow::Adverb => 1..3,
            ParseRow::MonadVVN => 2..4,
            ParseRow::Hook if !stack.get(3).is_some_and(|item| is_cavn(item.class)) => 1..3,
            _ => 1..4,
        };
        if stack[range]
            .iter()
            .any(|item| item.abandoned_nameless_conjunction)
        {
            return Err(Error::Unsupported(
                "abandoned nameless conjunction application".into(),
            ));
        }
    }
    Ok(match row {
        ParseRow::MonadEdge => {
            let mut phrase: Vec<_> = stack.drain(1..3).collect();
            let verb = phrase.remove(0).into_verb().expect("row 0 verb");
            let (argument, height) = phrase.remove(0).into_noun().expect("row 0 noun");
            let expr = Expr {
                origin: None,
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
                origin: None,
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
                origin: None,
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
                fork_reads: Some(&context.fork_name_reads),
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
                fork_reads: Some(&context.fork_name_reads),
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
                    let host = construction_host(&mut context.host);
                    let observations = std::cell::RefCell::new(Vec::new());
                    let names = ConstructionNames {
                        lookup: context.lookup,
                        host: host.as_ref(),
                        observations: context.capture.as_ref().map(|_| &observations),
                        row: Some(row),
                        fork_reads: Some(&context.fork_name_reads),
                    };
                    let result = train_fork(f, g, h, names);
                    if let Some(capture) = &mut context.capture {
                        capture.events.extend(observations.into_inner());
                    }
                    stack.insert(1, Item::verb(result?));
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
                // Ordered row eligibility admits only NVV/VVV here. Immediate
                // cf.c constructors are invoked by AR/modifier execution, not row 5.
                TridentDisposition::ImmediateSemanticApply
                | TridentDisposition::BuildDerivedModifier(_) => {
                    return Err(Error::Syntax(
                        "invalid row 5 constructor disposition".into(),
                    ));
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
                        // Rows 2/4 consume NVN and N/V C N/V before row 6.
                        return Err(Error::Syntax(
                            "immediate trident must be selected by an earlier row".into(),
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
                        // Rows 0/3 consume VN and N/V A before row 6.
                        return Err(Error::Syntax(
                            "immediate bident must be selected by an earlier row".into(),
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
            let mut phrase: Vec<_> = stack.drain(0..3).collect();
            let target = phrase.remove(0);
            let copula = phrase.remove(0);
            let value = phrase.remove(0);
            let span = target.span();
            let (single, noun) = match target.value {
                ParseValue::NameTarget { name, .. } => (Some(name), None),
                ParseValue::Noun(expr, _) => {
                    let names = string_assignment_names(&expr)?;
                    let noun = crate::semantic::NounAssignment {
                        target: expr,
                        names,
                    };
                    (None, Some(noun))
                }
                _ => return Err(Error::Syntax("row 7 requires a name/noun target".into())),
            };
            // Simple NAME writes retain their existing allocation/validation
            // path; only noun targets need a computed name list.
            let names = single
                .as_ref()
                .map(std::slice::from_ref)
                .unwrap_or_else(|| noun.as_ref().expect("noun target").names.as_slice());
            let mut source = AssignmentSource {
                target: target.provenance.expect("assignment target provenance"),
                copula: copula.provenance.expect("copula provenance"),
                flags: copula.flags,
                noun_target: noun.is_some(),
                selection: None,
            };
            // p.c row 7 transports the stacked RHS. A nonnameless modifier
            // remains a POS-bearing NameRef; assigning it is not application.
            // Nameless modifiers were already stacked by value during lookup.
            if let Some(host) = context.host.as_mut() {
                let occurrence = value.occurrence;
                let class = value.class;
                let abandoned_nameless_conjunction = value.abandoned_nameless_conjunction;
                let mut completed = CompletedParseResult::from_item(value, "assignment value")?;
                if names.len() != 1 {
                    if names.is_empty()
                        && !matches!(&completed.entity, JEntity::Noun(rhs) if !rhs.shape().is_empty() && rhs.shape()[0] == 0)
                    {
                        return Err(Error::IllFormedName);
                    }
                    let JEntity::Noun(rhs) = &mut completed.entity else {
                        return Err(Error::Domain);
                    };
                    if !rhs.shape().is_empty() && rhs.shape()[0] != names.len() {
                        return Err(Error::Length);
                    }
                    // Freeze once before selecting/cloning. Never clone an owned
                    // whole RHS separately for each target or capture event.
                    *rhs = std::mem::replace(rhs, Value::scalar(0)).into_shared();
                }
                let function = match &completed.entity {
                    JEntity::Function(function) => Some(function.clone()),
                    _ => None,
                };
                for (index, name) in names.iter().enumerate() {
                    if noun.is_some() {
                        crate::enqueuer::validate_assignment_name(name)?;
                    }
                    let version_name = host.assignment_version_name(name);
                    let previous =
                        host.assignment_version(&version_name, source.flags.local_assignment);
                    if names.len() == 1 {
                        completed.entity = host.assign_scoped(
                            name,
                            completed.entity,
                            source.flags.local_assignment,
                        )?;
                    } else {
                        let JEntity::Noun(rhs) = &completed.entity else {
                            unreachable!()
                        };
                        let item = (!rhs.shape().is_empty()).then_some(index);
                        source.selection = Some(AssignmentSelection { item });
                        let selected = assignment_item(rhs, item)?;
                        host.assign_scoped(
                            name,
                            JEntity::Noun(selected),
                            source.flags.local_assignment,
                        )?;
                    }
                    if let Some(capture) = &mut context.capture {
                        capture.events.push(CaptureEvent::Commit {
                            name: name.clone(),
                            version: host
                                .assignment_version(&version_name, source.flags.local_assignment)
                                .expect("committed version"),
                            previous,
                            span: span.clone(),
                            value: occurrence,
                            final_assignment: queue_exhausted && index + 1 == names.len(),
                            class,
                            function: function.clone(),
                            source: source.clone(),
                        });
                    }
                }
                source.selection = None;
                let mut result = completed.into_item()?;
                result.abandoned_nameless_conjunction = abandoned_nameless_conjunction;
                stack.insert(0, result);
                if queue_exhausted {
                    let name = single.or_else(|| {
                        noun.as_ref()
                            .and_then(|n| (n.names.len() == 1).then(|| n.names[0].clone()))
                    });
                    *assignment = Some(PendingAssignment {
                        name,
                        noun,
                        span,
                        source,
                    });
                }
            } else {
                if noun.is_some() {
                    for name in names {
                        crate::enqueuer::validate_assignment_name(name)?;
                    }
                }
                let name = single.or_else(|| {
                    noun.as_ref()
                        .and_then(|n| (n.names.len() == 1).then(|| n.names[0].clone()))
                });
                *assignment = Some(PendingAssignment {
                    name,
                    noun,
                    span,
                    source,
                });
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
            let abandoned_nameless_conjunction = value.abandoned_nameless_conjunction;

            let mut grouped = match value.value {
                ParseValue::Noun(expr, height) => Item::noun(
                    Expr {
                        origin: None,
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
            grouped.abandoned_nameless_conjunction = abandoned_nameless_conjunction;
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
        if !matches!(
            item.value,
            ParseValue::Noun(..) | ParseValue::Verb(_) | ParseValue::Function(_)
        ) {
            return Err(Error::Syntax("invalid modifier train operand".into()));
        }
        let completed =
            CompletedParseResult::from_item(item, "runtime-dependent modifier train noun operand")?;
        operands.push(completed.into_operand());
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

/// Concrete parser result transport. Deferred noun expressions, unresolved
/// names and control items stay in ParseValue; they are not concrete JEntity.
/// Occurrence adapters belong here, separately from immutable function identity.
struct CompletedParseResult {
    entity: JEntity,
    span: std::ops::Range<usize>,
    height: usize,
    verb_adapter: Option<(std::ops::Range<usize>, VerbTarget)>,
}

impl CompletedParseResult {
    fn noun(value: Value, span: std::ops::Range<usize>, height: usize) -> Self {
        Self {
            entity: JEntity::Noun(value),
            span,
            height,
            verb_adapter: None,
        }
    }

    fn from_noun(expr: Expr, height: usize, context: &str) -> Result<Self> {
        let span = expr.span.clone();
        Ok(Self::noun(completed_noun(expr, context)?, span, height))
    }

    fn function(
        entity: Arc<FunctionEntity>,
        span: std::ops::Range<usize>,
        target: VerbTarget,
    ) -> Self {
        let verb_adapter =
            (entity.result_pos == FunctionPartOfSpeech::Verb).then(|| (span.clone(), target));
        Self {
            entity: JEntity::Function(entity),
            span,
            height: 0,
            verb_adapter,
        }
    }

    /// Definition inputs are transient nouns; extracting them does not freeze
    /// their storage or retain body values in the completed function identity.
    fn into_noun_value(self) -> Result<Value> {
        match self.entity {
            JEntity::Noun(value) => Ok(value),
            JEntity::Function(_) => Err(Error::Domain),
        }
    }

    /// Move an already completed RHS. Never evaluate a deferred expression.
    fn from_item(item: Item, context: &str) -> Result<Self> {
        let span = item.span();
        let (entity, height, verb_adapter) = match item.value {
            ParseValue::Noun(expr, height) => {
                let mut completed = Self::from_noun(expr, height, context)?;
                completed.span = span;
                return Ok(completed);
            }
            ParseValue::Verb(verb) => {
                let mut completed = Self::function(verb.entity, verb.span, verb.target);
                completed.span = span;
                return Ok(completed);
            }
            ParseValue::Function(function) => (JEntity::Function(function), 0, None),
            _ => return Err(Error::Syntax("invalid assignment value".into())),
        };
        Ok(Self {
            entity,
            span,
            height,
            verb_adapter,
        })
    }

    /// Move a completed value into an immutable constructor operand. Freeze
    /// nouns once so later bound-modifier reuse shares their payload. Retain
    /// source occurrence provenance without making it function identity.
    fn into_operand(self) -> FunctionOperand {
        match self.entity {
            JEntity::Noun(value) => FunctionOperand::Noun {
                value: value.into_shared(),
                span: self.span,
            },
            JEntity::Function(function) => FunctionOperand::Function(function),
        }
    }

    fn into_item(self) -> Result<Item> {
        Ok(match self.entity {
            JEntity::Noun(value) => Item::noun(
                Expr {
                    origin: None,
                    span: self.span,
                    kind: ExprKind::Literal(value),
                },
                self.height,
            ),
            JEntity::Function(function) if function.result_pos == FunctionPartOfSpeech::Verb => {
                let (span, target) = self.verb_adapter.ok_or(Error::Domain)?;
                Item::verb(Verb {
                    span,
                    target,
                    entity: function,
                })
            }
            JEntity::Function(function) => Item::function(function),
        })
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

impl ParseValue {
    /// Borrow only completed function identity. Noun expressions, lexical names
    /// and control are not functions; observation does not update Arc counts.
    fn function_entity(&self) -> Option<&Arc<FunctionEntity>> {
        match self {
            Self::Verb(verb) => Some(&verb.entity),
            Self::Function(function) => Some(function),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct Item {
    // Parser transport state, not a property of the immutable function value.
    abandoned_nameless_conjunction: bool,
    frontend_occurrence: Option<std::num::NonZeroUsize>,
    class: ParseClass,
    value: ParseValue,
    span_override: Option<std::ops::Range<usize>>,
    provenance: Option<ParseProvenance>,
    flags: EnqueueFlags,
    occurrence: Option<OccurrenceId>,
}

impl Item {
    fn record_frontend(
        &mut self,
        trace: &mut FrontendContext,
        producer: ItemProducer,
        kind: Option<NodeKind>,
        alias: Option<crate::frontend_context::NodeId>,
    ) {
        let semantic = alias.or_else(|| {
            let kind = kind.or_else(|| match &self.value {
                ParseValue::Noun(..) => Some(NodeKind::Literal),
                ParseValue::Verb(verb) => Some(NodeKind::Function(verb.entity.clone())),
                ParseValue::Function(function) => Some(NodeKind::Function(function.clone())),
                _ => None,
            });
            kind.map(|kind| trace.node(kind, self.class))
        });
        if let ParseValue::Noun(expr, _) = &mut self.value {
            expr.origin = semantic;
        }
        let id = trace.item(ItemRecord {
            producer,
            class: self.class,
            semantic,
            word_range: self
                .provenance
                .as_ref()
                .map_or(0..0, |p| p.word_range.clone()),
            blame_word: self.provenance.as_ref().map(|p| WordId(p.blame_word_index)),
        });
        self.frontend_occurrence = std::num::NonZeroUsize::new(
            id.0.checked_add(1).expect("parser item identity exhausted"),
        );
    }

    fn frontend_id(&self) -> Option<ItemId> {
        self.frontend_occurrence.map(|id| ItemId(id.get() - 1))
    }

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
            frontend_occurrence: None,
            abandoned_nameless_conjunction: false,
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
            frontend_occurrence: None,
            abandoned_nameless_conjunction: false,
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
            frontend_occurrence: None,
            abandoned_nameless_conjunction: false,
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
            frontend_occurrence: None,
            abandoned_nameless_conjunction: false,
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
            frontend_occurrence: None,
            abandoned_nameless_conjunction: false,
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
            frontend_occurrence: None,
            abandoned_nameless_conjunction: false,
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
            frontend_occurrence: None,
            abandoned_nameless_conjunction: false,
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

/// The same parser, retaining a structured prefix even on an error/boundary.
/// Diagnostic NAME assumptions are explicitly marked, not execution proofs.
pub fn parse_frontend(
    source: &str,
) -> std::result::Result<Program, crate::frontend_context::FrontendFailure> {
    parse_frontend_with_lookup(source, None)
}

pub(crate) fn parse_frontend_with_lookup(
    source: &str,
    lookup: NameLookup<'_>,
) -> std::result::Result<Program, crate::frontend_context::FrontendFailure> {
    parse_frontend_origin_with_lookup(
        crate::source::SourceUnit::new("<input>", source).origin(),
        lookup,
    )
}

/// Execution-free frontend with an owned path to the original input revision.
pub fn parse_frontend_source(
    origin: crate::source::SourceOrigin,
) -> std::result::Result<Program, crate::frontend_context::FrontendFailure> {
    parse_frontend_origin_with_lookup(origin, None)
}

fn parse_frontend_origin_with_lookup(
    origin: crate::source::SourceOrigin,
    lookup: NameLookup<'_>,
) -> std::result::Result<Program, crate::frontend_context::FrontendFailure> {
    let source = origin.text();
    let mut context = ActionContext {
        source_origin: origin.clone(),
        single_word: false,
        last_lookup_version: None,
        last_name_policy: None,
        frontend: Some(FrontendContext {
            source: Arc::from(source),
            ..Default::default()
        }),
        mode: ParseContext::Analysis,
        lookup,
        host: None,
        capture: None,
        modifier_snapshots: Vec::new(),
        fork_name_reads: Default::default(),
        name_rank_snapshots: Vec::new(),
    };
    parse_context(source, &mut context).map_err(|error| crate::frontend_context::FrontendFailure {
        error,
        context: Box::new(context.frontend.take().expect("failed frontend context")),
    })
}

#[derive(Clone, Debug)]
pub(crate) enum ParserNameBinding {
    Noun(Value),
    /// Analysis-only noun class, with facts owned by the input catalog.
    /// This is not a dummy Value and must never enter concrete execution.
    AbstractNoun,
    Function(FunctionPartOfSpeech),
    /// Analysis-only direct binding witness. Ordinary lookup still emits NAME.
    KnownVerb {
        function: Arc<FunctionEntity>,
        version: crate::semantic::NameVersion,
    },
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
    fn take_name(&mut self, _name: &str, _single_word: bool) -> Result<(JEntity, bool)> {
        Err(Error::Unsupported("abandon lookup host".into()))
    }
    fn lookup_observation(
        &self,
        _name: &str,
    ) -> Option<crate::frontend_context::LookupObservation> {
        None
    }
    fn function_name_ranks(&self, _name: &str) -> Option<[i64; 3]> {
        None
    }

    /// Opt in only when the host implements explicit base-locale noun lookup
    /// and writes; lexical acceptance alone must not enable namespace access.
    fn supports_base_locative_nouns(&self) -> bool {
        false
    }
    fn supports_named_direct_locative_nouns(&self) -> bool {
        false
    }
    fn supports_indirect_noun_reads(&self) -> bool {
        false
    }
    fn supports_indirect_noun_assignments(&self) -> bool {
        false
    }

    fn lookup(&mut self, name: &str) -> Result<Option<ParserNameBinding>>;
    fn fork_cap_binding(
        &self,
        _name: &str,
    ) -> Result<Option<(bool, crate::semantic::NameVersion)>> {
        Ok(None)
    }
    fn enqueue_environment(&self) -> crate::enqueuer::EnqueueEnvironment {
        crate::enqueuer::EnqueueEnvironment::TopLevel
    }
    fn assign_scoped(&mut self, name: &str, value: JEntity, local: bool) -> Result<JEntity> {
        if local {
            return Err(Error::Unsupported(
                "local parser-time assignment scope".into(),
            ));
        }
        self.assign(name, value)
    }

    /// mnuvxy are value substitutions in p.c, unlike ordinary function names.
    fn operand_function(&self, _name: &str) -> Option<Arc<FunctionEntity>> {
        None
    }
    fn apply_definition(
        &mut self,
        _operator: Arc<FunctionEntity>,
        _left: FunctionOperand,
        _right: Option<FunctionOperand>,
    ) -> Result<JEntity> {
        Err(Error::Unsupported(
            "explicit modifier invocation host".into(),
        ))
    }
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
    /// Freeze target identity before the commit can rebind its holder.
    fn assignment_version_name(&self, name: &str) -> String {
        name.to_owned()
    }
    /// Observe the table that assignment will update, without lookup fallback.
    fn assignment_version(&self, name: &str, _local: bool) -> Option<crate::semantic::NameVersion> {
        self.version(name)
    }
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

#[cfg(test)]
pub(crate) fn parse_runtime_host(
    source: &str,
    host: &mut dyn RuntimeParserHost,
    capture: Option<&mut ParseCapture>,
) -> Result<Program> {
    parse_runtime_source(
        crate::source::SourceUnit::new("<input>", source).origin(),
        host,
        capture,
    )
}

pub(crate) fn parse_runtime_source(
    origin: crate::source::SourceOrigin,
    host: &mut dyn RuntimeParserHost,
    capture: Option<&mut ParseCapture>,
) -> Result<Program> {
    let source = origin.text();
    let mut capture = capture;
    if let Some(capture) = &mut capture {
        capture.set_source(source);
    }
    let mut context = ActionContext {
        source_origin: origin.clone(),
        single_word: false,
        last_lookup_version: None,
        last_name_policy: None,
        frontend: None,
        mode: ParseContext::Runtime,
        lookup: None,
        host: Some(host),
        capture,
        modifier_snapshots: Vec::new(),
        fork_name_reads: Default::default(),
        name_rank_snapshots: Vec::new(),
    };
    let result = parse_context(source, &mut context);
    if let Some(capture) = &mut context.capture {
        capture.frontend = match &result {
            Ok(program) => program.frontend.clone(),
            Err(_) => context.frontend.take().map(Arc::new),
        };
    }
    result
}

struct ActionContext<'a> {
    source_origin: crate::source::SourceOrigin,
    single_word: bool,
    last_name_policy: Option<NamePolicy>,
    last_lookup_version: Option<crate::semantic::NameVersion>,
    frontend: Option<FrontendContext>,
    mode: ParseContext,
    lookup: NameLookup<'a>,
    host: Option<&'a mut dyn RuntimeParserHost>,
    capture: Option<&'a mut ParseCapture>,
    modifier_snapshots: Vec<crate::semantic::ModifierSnapshot>,
    fork_name_reads: std::cell::RefCell<Vec<crate::semantic::NameUse>>,
    name_rank_snapshots: Vec<crate::semantic::NameRankSnapshot>,
}

/// Resolve one ordinary name only when its queue entry reaches the stack.
fn resolve_stack_item(item: Item, context: &mut ActionContext<'_>) -> Result<Item> {
    let ParseValue::LookupName { name, span } = &item.value else {
        return Ok(item);
    };
    let name = name.clone();
    let span = span.clone();
    if item.flags.abandon_name {
        context.last_name_policy = Some(NamePolicy::CaptureAndAbandon);
        context.last_lookup_version = context.host.as_ref().and_then(|host| host.version(&name));
        let mut resolved = if let Some(host) = &mut context.host {
            let lookup = context
                .capture
                .as_ref()
                .and_then(|_| host.lookup_observation(&name));
            let (entity, deleted) = host
                .take_name(&name, context.single_word)
                .map_err(|e| e.at(span.clone()))?;
            let abandoned_nameless_conjunction = matches!(&entity,
                JEntity::Function(function)
                if function.result_pos == FunctionPartOfSpeech::Conjunction
                    && function.is_nameless_modifier());
            if let (Some(capture), Some(lookup)) = (&mut context.capture, lookup) {
                capture.events.push(CaptureEvent::Abandon {
                    name: name.clone(),
                    lookup,
                    deleted,
                    span: span.clone(),
                });
            }
            let completed = match entity {
                JEntity::Noun(value) => CompletedParseResult::noun(value, span.clone(), 0),
                JEntity::Function(function) => {
                    CompletedParseResult::function(function, span.clone(), VerbTarget::Derived)
                }
            };
            let mut resolved = completed.into_item()?;
            resolved.abandoned_nameless_conjunction = abandoned_nameless_conjunction;
            resolved
        } else {
            let pos = match context.lookup.and_then(|lookup| lookup(&name)) {
                Some(ParserNameBinding::Function(pos)) => Some(pos),
                Some(
                    ParserNameBinding::KnownVerb { function, version }
                    | ParserNameBinding::KnownModifier { function, version },
                ) => {
                    context.last_lookup_version = Some(version);
                    Some(function.result_pos)
                }
                None if context.lookup.is_some() => return Err(Error::Value(name).at(span)),
                _ => None,
            };
            if let Some(pos) = pos {
                let function = FunctionEntity::derived(
                    FunctionHead::TakeName {
                        name,
                        single_word: context.single_word,
                    },
                    pos,
                    span.clone(),
                    Vec::new(),
                );
                CompletedParseResult::function(function, span, VerbTarget::Derived).into_item()?
            } else {
                Item::noun(
                    Expr {
                        origin: None,
                        span,
                        kind: ExprKind::TakeName {
                            name,
                            single_word: context.single_word,
                        },
                    },
                    0,
                )
            }
        };
        resolved.provenance = item.provenance;
        resolved.flags = item.flags;
        return Ok(resolved);
    }
    let binding = if let Some(host) = &mut context.host {
        host.lookup(&name).map_err(|error| {
            let error = error.at(span.clone());
            match &item.provenance {
                Some(provenance) => error.blamed_on_word(provenance.blame_word_index),
                None => error,
            }
        })?
    } else {
        context.lookup.and_then(|lookup| lookup(&name))
    };
    let (name_ranks, name_version) = if let Some(host) = &context.host {
        (host.function_name_ranks(&name), host.version(&name))
    } else {
        match &binding {
            Some(ParserNameBinding::KnownVerb { function, version }) => {
                (function.innate_ranks(), Some(*version))
            }
            Some(ParserNameBinding::KnownModifier { version, .. }) => (None, Some(*version)),
            None if context.lookup.is_some() => (Some([63; 3]), None),
            _ => (None, None),
        }
    };
    context.last_lookup_version = name_version;
    let binding = match binding {
        Some(ParserNameBinding::KnownVerb { function, .. }) => {
            Some(ParserNameBinding::Function(function.result_pos))
        }
        other => other,
    };
    context.last_name_policy = Some(match &binding {
        Some(
            ParserNameBinding::Noun(_)
            | ParserNameBinding::AbstractNoun
            | ParserNameBinding::KnownModifier { .. },
        ) => NamePolicy::CaptureAtRead,
        Some(ParserNameBinding::Function(FunctionPartOfSpeech::Verb)) => NamePolicy::LateAtCall,
        Some(ParserNameBinding::Function(_)) => NamePolicy::ResolveAtConstruction,
        None if context.lookup.is_none() && context.host.is_none() => NamePolicy::CaptureAtRead,
        None => NamePolicy::LateAtCall,
        Some(ParserNameBinding::KnownVerb { .. }) => unreachable!("normalized verb witness"),
    });
    let mut resolved = match binding {
        Some(ParserNameBinding::KnownVerb { .. }) => unreachable!("normalized verb witness"),
        Some(ParserNameBinding::Noun(value)) => Item::noun(
            Expr {
                origin: None,
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
                origin: None,
                span,
                kind: ExprKind::ReadName(name),
            },
            0,
        ),
        None if context.lookup.is_none() && context.host.is_none() => Item::noun(
            Expr {
                origin: None,
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
            if let Some(function) = context
                .host
                .as_ref()
                .and_then(|host| host.operand_function(&name))
            {
                let mut substituted =
                    CompletedParseResult::function(function, span, VerbTarget::Derived)
                        .into_item()?;
                substituted.provenance = item.provenance;
                context.last_name_policy = Some(NamePolicy::CaptureAtRead);
                substituted.flags = item.flags;
                return Ok(substituted);
            }
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
                    context.last_name_policy = Some(NamePolicy::CaptureAtRead);
                    stacked.flags = item.flags;
                    return Ok(stacked);
                }
            }
            let entity = FunctionEntity::with_name_ranks(
                FunctionEntity::name_ref(name.clone(), pos, span.clone()),
                name_ranks,
            );
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
            entity: FunctionEntity::with_name_ranks(
                FunctionEntity::name_ref(name, FunctionPartOfSpeech::Verb, span),
                name_ranks,
            ),
        }),
    };
    if let Some(function) = resolved.value.function_entity() {
        if let (FunctionHead::NameRef(name), FunctionPartOfSpeech::Verb) =
            (&function.head, function.result_pos)
        {
            let snapshot = crate::semantic::NameRankSnapshot {
                name: name.clone(),
                version: name_version,
                ranks: name_ranks,
                span: function.span.clone(),
            };
            if let Some(capture) = &mut context.capture {
                capture.events.push(CaptureEvent::FunctionNameRank {
                    snapshot: snapshot.clone(),
                });
            }
            context.name_rank_snapshots.push(snapshot);
        }
    }
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
            source_origin: crate::source::SourceUnit::new("<input>", source).origin(),
            single_word: false,
            last_lookup_version: None,
            last_name_policy: None,
            frontend: None,
            mode,
            lookup,
            host: None,
            capture: None,
            modifier_snapshots: Vec::new(),
            fork_name_reads: Default::default(),
            name_rank_snapshots: Vec::new(),
        },
    )
}

fn parse_context(source: &str, context: &mut ActionContext<'_>) -> Result<Program> {
    if context.frontend.is_none()
        && (context.mode == ParseContext::Analysis || context.capture.is_some())
    {
        context.frontend = Some(FrontendContext {
            source: Arc::from(source),
            ..Default::default()
        });
    }
    if let Some(trace) = &mut context.frontend {
        trace.realization = if context.mode == ParseContext::Runtime {
            crate::frontend_context::ParseRealization::Observed
        } else {
            crate::frontend_context::ParseRealization::Deferred
        };
    }
    let environment = context
        .host
        .as_ref()
        .map_or(crate::enqueuer::EnqueueEnvironment::TopLevel, |host| {
            host.enqueue_environment()
        });
    if let Some(trace) = &mut context.frontend {
        trace.source_origin = Some(context.source_origin.clone());
    }
    let mut queue = crate::enqueuer::enqueue_with_origin(
        source,
        &crate::primitive::PrimitiveContext::core(),
        environment,
        Some(&context.source_origin),
    )?;
    context.single_word = queue.len() == 1;
    if let Some(trace) = &mut context.frontend {
        trace.words = queue
            .iter()
            .map(|word| WordRecord {
                span: word.span.clone(),
                class: word.class,
                flags: word.flags,
                environment,
                name: match &word.payload {
                    EnqueuedPayload::Name(name) => Some((*name).to_owned()),
                    _ => None,
                },
            })
            .collect();
    }
    if queue.is_empty() {
        return Ok(Program {
            frontend: context.frontend.take().map(|mut trace| {
                trace.complete = true;
                Arc::new(trace)
            }),
            source: source.to_owned(),
            assignment: None,
            noun_assignment: None,
            assignment_span: None,
            expression: None,
            reductions: Vec::new(),
            assignment_source: None,
            modifier_snapshots: Vec::new(),
            fork_name_reads: Default::default(),
            name_rank_snapshots: Vec::new(),
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

    let (assignment, noun_assignment, assignment_span, assignment_source) = match pending_assignment
    {
        Some(PendingAssignment {
            name,
            noun,
            span,
            source,
        }) => (name, noun, Some(span), Some(source)),
        None => (None, None, None, None),
    };
    Ok(Program {
        frontend: context.frontend.take().map(Arc::new),
        source: source.to_owned(),
        assignment,
        noun_assignment,
        assignment_span,
        expression: Some(result),
        reductions,
        assignment_source,
        modifier_snapshots: std::mem::take(&mut context.modifier_snapshots),
        fork_name_reads: context.fork_name_reads.take(),
        name_rank_snapshots: std::mem::take(&mut context.name_rank_snapshots),
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
                        origin: None,
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
                        origin: None,
                        span: tokens[*pos].span.clone(),
                        kind: ExprKind::Literal(*v),
                    },
                    0,
                ));
                *pos += 1;
            }
            EnqueuedPayload::Name(name) => {
                let span = tokens[*pos].span.clone();
                // Enqueue recognizes the J NAME grammar. Until locale-scoped
                // lookup/write exists, never flatten a locative into a key in
                // the ordinary namespace, including assignment targets.
                let base_noun_host = tokens[*pos].flags.name_form
                    == crate::enqueuer::NameForm::BaseLocative
                    && context
                        .host
                        .as_ref()
                        .is_some_and(|host| host.supports_base_locative_nouns());
                let named_noun_host = tokens[*pos].flags.name_form
                    == crate::enqueuer::NameForm::DirectLocative
                    && context
                        .host
                        .as_ref()
                        .is_some_and(|host| host.supports_named_direct_locative_nouns());
                let indirect_error_host = !tokens[*pos].flags.abandon_name
                    && tokens[*pos].flags.name_form == crate::enqueuer::NameForm::IndirectLocative
                    && context.host.as_ref().is_some_and(|host| {
                        if tokens[*pos].flags.lookup_name {
                            host.supports_indirect_noun_reads()
                        } else {
                            host.supports_indirect_noun_assignments()
                        }
                    });
                if tokens[*pos].flags.name_form.is_locative()
                    && !base_noun_host
                    && !named_noun_host
                    && !indirect_error_host
                {
                    return Err(Error::Unsupported("J locative namespace resolution".into())
                        .with_context(
                            ErrorContext::phase(DiagnosticPhase::Parse)
                                .with_span(span)
                                .with_blame_word(tokens[*pos].word_index),
                        ));
                }
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
        let mut item = item.with_source(&tokens[source_word]);
        if let Some(trace) = &mut context.frontend {
            item.record_frontend(
                trace,
                ItemProducer::Word(WordId(tokens[source_word].word_index)),
                None,
                None,
            );
        }
        items.push(item);
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
    if let Some(trace) = &mut context.frontend {
        trace.root = item.frontend_id();
    }
    let span = item.span();
    if let Some(capture) = &mut context.capture {
        capture.result = item.occurrence;
        let function = item.value.function_entity().cloned();
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
    let result_origin = item.frontend_id().and_then(|id| {
        context
            .frontend
            .as_ref()
            .and_then(|trace| trace.items[id.0].semantic)
    });
    let result = match item.value {
        ParseValue::Noun(expr, height) => Ok((expr, height, assignment)),
        ParseValue::Verb(verb) => Ok((
            Expr {
                origin: result_origin,
                span: verb.span.clone(),
                kind: ExprKind::VerbValue(verb),
            },
            0,
            assignment,
        )),
        ParseValue::Function(entity) => Ok((
            Expr {
                origin: result_origin,
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
    }?;
    if let Some(trace) = &mut context.frontend {
        trace.complete = true;
    }
    Ok(result)
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
mod deferred_abandon_tests {
    use super::*;

    fn function(program: &Program) -> &Arc<FunctionEntity> {
        match &program.expression.as_ref().unwrap().kind {
            ExprKind::VerbValue(verb) | ExprKind::Monad { verb, .. } => &verb.entity,
            ExprKind::ModifierValue(function) => function,
            kind => panic!("unexpected expression {kind:?}"),
        }
    }

    #[test]
    fn deferred_function_transport_preserves_pos_source_and_context() {
        for pos in [
            FunctionPartOfSpeech::Verb,
            FunctionPartOfSpeech::Adverb,
            FunctionPartOfSpeech::Conjunction,
        ] {
            for (source, single) in [
                ("taken_:", true),
                ("saved=:taken_:", false),
                ("(taken_:)", false),
            ] {
                let program =
                    parse_analysis(source, &|_| Some(ParserNameBinding::Function(pos))).unwrap();
                let entity = function(&program);
                assert_eq!(entity.result_pos, pos);
                assert!(
                    matches!(&entity.head, FunctionHead::TakeName { name, single_word } if name == "taken" && *single_word == single)
                );
                assert!(entity.operands.is_empty());
                assert!(entity.innate_ranks().is_none());
                let context = program.frontend.as_ref().unwrap();
                context.verify().unwrap();
                assert!(context.complete);
                assert_eq!(context.name_uses[0].policy, NamePolicy::CaptureAndAbandon);
                assert_eq!(context.name_uses[0].evidence, NameEvidence::CatalogClass);
                assert!(program.name_rank_snapshots.is_empty());
                assert_eq!(
                    crate::semantic::bind(program.clone(), |_| Some(crate::semantic::NameVersion(
                        1
                    )))
                    .unwrap_err()
                    .kind(),
                    "unsupported"
                );
                // Even callers constructing BoundProgram themselves cannot
                // silently turn the deferred effect into a function constant.
                if source == "taken_:" && pos == FunctionPartOfSpeech::Verb {
                    let mut observed = ParseCapture::default();
                    observed.set_source(source);
                    observed.events.push(CaptureEvent::FunctionResult {
                        function: function(&program).clone(),
                        span: 0..source.len(),
                    });
                    observed.verify().unwrap();
                    assert_eq!(
                        crate::j_graph_ir::Plan::from_capture(&observed)
                            .unwrap_err()
                            .kind(),
                        "unsupported"
                    );
                }
                let forged = crate::semantic::BoundProgram {
                    program,
                    reads: vec![],
                    verb_references: vec![],
                    write: None,
                };
                assert_eq!(
                    crate::j_graph_ir::Plan::from_bound(forged)
                        .unwrap_err()
                        .kind(),
                    "unsupported"
                );
            }
        }
    }

    #[test]
    fn deferred_function_effect_is_not_erased_inside_applied_trains() {
        for source in ["taken_: 3", "(-taken_:)3"] {
            let program = parse_analysis(source, &|_| {
                Some(ParserNameBinding::Function(FunctionPartOfSpeech::Verb))
            })
            .unwrap();
            program.frontend.as_ref().unwrap().verify().unwrap();
            assert_eq!(
                function(&program)
                    .reject_deferred_name_effects()
                    .unwrap_err()
                    .kind(),
                "unsupported"
            );
            assert_eq!(
                crate::semantic::bind(program.clone(), |_| Some(crate::semantic::NameVersion(1)))
                    .unwrap_err()
                    .kind(),
                "unsupported"
            );
            let forged = crate::semantic::BoundProgram {
                program,
                reads: vec![],
                verb_references: vec![],
                write: None,
            };
            assert_eq!(
                crate::j_graph_ir::Plan::from_bound(forged)
                    .unwrap_err()
                    .kind(),
                "unsupported"
            );
        }
    }

    #[test]
    fn catalog_identity_is_not_a_captured_function_value() {
        for binding in [
            ParserNameBinding::KnownVerb {
                function: FunctionEntity::primitive(crate::primitive::PrimitiveId::Add, 0..1),
                version: crate::semantic::NameVersion(7),
            },
            ParserNameBinding::KnownModifier {
                function: FunctionEntity::primitive_adverb(
                    crate::primitive::AdverbId::Insert,
                    0..1,
                ),
                version: crate::semantic::NameVersion(7),
            },
        ] {
            let program = parse_analysis("taken_:", &|_| Some(binding.clone())).unwrap();
            let context = program.frontend.as_ref().unwrap();
            context.verify().unwrap();
            assert_eq!(
                context.name_uses[0].binding_version,
                Some(crate::semantic::NameVersion(7))
            );
            assert!(matches!(
                function(&program).head,
                FunctionHead::TakeName { .. }
            ));
            assert!(program.modifier_snapshots.is_empty());
        }
    }

    #[test]
    fn deferred_function_verifier_rejects_wrong_name_or_sentence_context() {
        let program = parse_analysis("saved=:taken_:", &|_| {
            Some(ParserNameBinding::Function(FunctionPartOfSpeech::Verb))
        })
        .unwrap();
        let context = program.frontend.unwrap();
        for (name, single_word) in [("other", false), ("taken", true)] {
            let mut invalid = context.as_ref().clone();
            for node in &mut invalid.nodes {
                if matches!(&node.kind, NodeKind::Function(f) if matches!(f.head, FunctionHead::TakeName { .. }))
                {
                    node.kind = NodeKind::Function(FunctionEntity::derived(
                        FunctionHead::TakeName {
                            name: name.into(),
                            single_word,
                        },
                        FunctionPartOfSpeech::Verb,
                        7..14,
                        Vec::new(),
                    ));
                }
            }
            assert!(invalid.verify().is_err());
        }
    }
}

#[cfg(test)]
mod parser_table_tests {
    use super::{ParseClass::*, ParseRow, match_parse_row};

    #[test]
    fn modifier_nameref_is_resolved_at_construction_not_at_verb_call() {
        let program = super::parse_analysis("modifier", &|_| {
            Some(super::ParserNameBinding::Function(
                crate::semantic::FunctionPartOfSpeech::Conjunction,
            ))
        })
        .unwrap();
        let context = program.frontend.unwrap();
        context.verify().unwrap();
        assert_eq!(
            context.name_uses[0].policy,
            crate::frontend_context::NamePolicy::ResolveAtConstruction
        );
        assert_eq!(context.name_uses[0].result_class, Conjunction);
    }

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
    fn ordered_rows_exclude_immediate_actions_from_hook_and_fork() {
        // p.c row precedence must not expose cf.c's invisible execution cases
        // as surface Hook/Fork actions. Cover every four-class stack window.
        let classes = [
            Noun,
            Verb,
            Adverb,
            Conjunction,
            Name,
            Assignment,
            LParen,
            RParen,
            Mark,
        ];
        let mut hook_windows = 0;
        let mut fork_windows = 0;
        for a in classes {
            for b in classes {
                for c in classes {
                    for d in classes {
                        let window = [a, b, c, d];
                        match match_parse_row(window) {
                            Some(ParseRow::Fork) => {
                                fork_windows += 1;
                                assert_eq!(
                                    super::trident_disposition(b, c, d),
                                    super::TridentDisposition::BuildFork,
                                    "{window:?}",
                                );
                            }
                            Some(ParseRow::Hook) => {
                                hook_windows += 1;
                                if super::is_cavn(d) {
                                    assert!(
                                        !matches!(
                                            super::trident_disposition(b, c, d),
                                            super::TridentDisposition::ImmediateSemanticApply
                                                | super::TridentDisposition::BuildFork
                                        ),
                                        "{window:?}"
                                    );
                                } else {
                                    assert_ne!(
                                        super::bident_disposition(b, c),
                                        super::BidentDisposition::ImmediateSemanticApply,
                                        "{window:?}",
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        assert!(hook_windows > 0 && fork_windows > 0);
        for edge in [Mark, Assignment, LParen] {
            assert_eq!(
                match_parse_row([edge, Verb, Noun, RParen]),
                Some(ParseRow::MonadEdge)
            );
            for operand in [Noun, Verb] {
                assert_eq!(
                    match_parse_row([edge, operand, Adverb, RParen]),
                    Some(ParseRow::Adverb)
                );
                for right in [Noun, Verb] {
                    assert_eq!(
                        match_parse_row([edge, operand, Conjunction, right]),
                        Some(ParseRow::Conjunction)
                    );
                }
            }
            assert_eq!(
                match_parse_row([edge, Noun, Verb, Noun]),
                Some(ParseRow::DyadNVN)
            );
        }
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

    #[test]
    fn lookup_failure_preserves_error_span_and_stops_left_lookup_and_execution() {
        struct FailingHost {
            reads: Vec<String>,
            error: Option<Error>,
        }
        impl RuntimeParserHost for FailingHost {
            fn lookup(&mut self, name: &str) -> Result<Option<ParserNameBinding>> {
                self.reads.push(name.into());
                Err(self.error.take().expect("no lookup after failure"))
            }
            fn version(&self, _: &str) -> Option<crate::semantic::NameVersion> {
                panic!("no version observation after failed lookup")
            }
            fn apply(&mut self, _: Expr) -> Result<Value> {
                panic!("no execution after failed lookup")
            }
        }
        for failure in [
            Error::Domain,
            Error::Value("holder".into()),
            Error::Unsupported("locale lookup".into()),
        ] {
            let kind = failure.kind();
            let mut host = FailingHost {
                reads: Vec::new(),
                error: Some(failure),
            };
            let mut capture = ParseCapture::default();
            let error =
                parse_runtime_host("left + right", &mut host, Some(&mut capture)).unwrap_err();
            assert_eq!(error.kind(), kind);
            assert_eq!(error.span(), Some(&(7..12)));
            assert_eq!(error.context().unwrap().blame_word_index, Some(2));
            assert_eq!(host.reads, ["right"]);
            assert!(
                capture.events.is_empty(),
                "failed lookup is not a successful read"
            );
        }
    }

    #[test]
    fn failed_lookup_keeps_prior_execution_and_capture_without_replay() {
        struct Host {
            calls: usize,
            reads: usize,
        }
        impl RuntimeParserHost for Host {
            fn lookup(&mut self, name: &str) -> Result<Option<ParserNameBinding>> {
                assert_eq!(name, "missing");
                self.reads += 1;
                Err(Error::Domain)
            }
            fn version(&self, _: &str) -> Option<crate::semantic::NameVersion> {
                panic!("failed read has no version witness")
            }
            fn apply(&mut self, _: Expr) -> Result<Value> {
                self.calls += 1;
                Ok(Value::scalar(3))
            }
        }
        let mut host = Host { calls: 0, reads: 0 };
        let mut capture = ParseCapture::default();
        let error =
            parse_runtime_host("missing + (1 + 2)", &mut host, Some(&mut capture)).unwrap_err();
        assert_eq!(error.kind(), "domain error");
        assert_eq!(error.span(), Some(&(0..7)));
        assert_eq!(error.context().unwrap().blame_word_index, Some(0));
        assert_eq!((host.calls, host.reads), (1, 1));
        assert!(
            !capture.events.is_empty(),
            "completed right call remains captured"
        );
    }

    #[test]
    fn missing_lookup_still_builds_a_late_function_reference() {
        struct MissingHost;
        impl RuntimeParserHost for MissingHost {
            fn lookup(&mut self, _: &str) -> Result<Option<ParserNameBinding>> {
                Ok(None)
            }
            fn version(&self, _: &str) -> Option<crate::semantic::NameVersion> {
                None
            }
            fn apply(&mut self, _: Expr) -> Result<Value> {
                panic!("bare missing name is deferred")
            }
        }
        let program = parse_runtime_host("future", &mut MissingHost, None).unwrap();
        let ExprKind::VerbValue(verb) = program.expression.unwrap().kind else {
            panic!("missing name must remain a function reference");
        };
        assert_eq!(verb.target, VerbTarget::Named("future".into()));
        assert!(matches!(&verb.entity.head, FunctionHead::NameRef(name) if name == "future"));
    }

    struct Host {
        log: Vec<String>,
        left: i64,
    }
    impl RuntimeParserHost for Host {
        fn lookup(&mut self, name: &str) -> Result<Option<ParserNameBinding>> {
            self.log.push(format!("lookup:{name}"));
            Ok(Some(ParserNameBinding::Noun(Value::scalar(
                if name == "left" { self.left } else { 3 },
            ))))
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
            origin: None,
            span: 0..1,
            kind: ExprKind::Literal(value),
        };
        let item = Item::noun(
            Expr {
                origin: None,
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
    fn immediate_constructor_moves_large_host_result_without_copying_or_reexecution() {
        struct Host {
            output: Option<Value>,
            dyad: bool,
            calls: usize,
        }
        impl RuntimeParserHost for Host {
            fn lookup(&mut self, _: &str) -> Result<Option<ParserNameBinding>> {
                Ok(None)
            }
            fn version(&self, _: &str) -> Option<crate::semantic::NameVersion> {
                None
            }
            fn apply(&mut self, expression: Expr) -> Result<Value> {
                assert_eq!(matches!(expression.kind, ExprKind::Dyad { .. }), self.dyad);
                assert_eq!(expression.span, 11..29);
                self.calls += 1;
                Ok(self.output.take().expect("exactly one constructor call"))
            }
        }
        for dyad in [false, true] {
            let output = Value::new(
                [256, 256],
                Data::Int(CpuStorage::new((0..65_536).collect())),
            )
            .unwrap();
            let pointer = match &output.data {
                Data::Int(storage) => storage.as_ptr(),
                _ => panic!(),
            };
            let args = if dyad {
                vec![noun(Value::scalar(2)), text("+"), noun(Value::scalar(3))]
            } else {
                vec![text("+"), noun(Value::scalar(3))]
            };
            let serialized = ar("4", args);
            let mut host = Host {
                output: Some(output),
                dyad,
                calls: 0,
            };
            let item = {
                let bridge = std::cell::RefCell::new(&mut host as &mut dyn RuntimeParserHost);
                decode_gerund_ar(
                    &serialized,
                    11..29,
                    0,
                    ConstructionNames {
                        host: Some(&bridge),
                        ..ConstructionNames::default()
                    },
                )
                .unwrap()
            };
            assert_eq!(host.calls, 1);
            assert_eq!(item.class, ParseClass::Noun);
            assert_eq!(item.span(), 11..29);
            let retained = CompletedParseResult::from_item(item, "immediate result")
                .unwrap()
                .into_operand();
            let FunctionOperand::Noun { value, span } = retained else {
                panic!()
            };
            assert_eq!(span, 11..29);
            let Data::Int(storage) = &value.data else {
                panic!()
            };
            assert_eq!(storage.as_ptr(), pointer);
            assert_eq!(value.shape(), &[256, 256]);
            assert_eq!(value.int_at(65_535).unwrap(), 65_535);
        }
    }

    #[test]
    fn constructor_call_observes_updated_host_and_static_path_never_executes() {
        struct Host {
            calls: usize,
        }
        impl RuntimeParserHost for Host {
            fn lookup(&mut self, _: &str) -> Result<Option<ParserNameBinding>> {
                Ok(None)
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
    fn fork_first_name_needs_constructor_identity_not_only_pos() {
        let lookup = |_: &str| Some(ParserNameBinding::Function(FunctionPartOfSpeech::Verb));
        let error = parse_analysis("f + -", &lookup).unwrap_err();
        assert_eq!(error.kind(), "unsupported");
        assert!(error.to_string().contains("binding witness"));
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
            fork_reads: None,
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
            fork_reads: None,
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

#[cfg(test)]
mod completed_result_tests {
    use super::*;
    use crate::value::Data;

    #[test]
    fn grouped_owned_noun_moves_without_copy_and_keeps_occurrence_and_height() {
        let value = Value::ints([65536], (0..65536).collect()).unwrap();
        let Data::Int(data) = value.data() else {
            panic!()
        };
        let pointer = data.as_ptr();
        let item = Item::noun(
            Expr {
                origin: None,
                span: 2..9,
                kind: ExprKind::Group(Box::new(Expr {
                    origin: None,
                    span: 3..8,
                    kind: ExprKind::Literal(value),
                })),
            },
            7,
        )
        .with_span(1..10);
        let completed = CompletedParseResult::from_item(item, "test").unwrap();
        assert_eq!(completed.span, 1..10);
        let item = completed.into_item().unwrap();
        assert_eq!(item.span(), 1..10);
        let (expr, height) = item.into_noun().unwrap();
        assert_eq!(height, 7);
        let value = completed_noun(expr, "test").unwrap();
        let Data::Int(data) = value.data() else {
            panic!()
        };
        assert_eq!(data.as_ptr(), pointer);
        assert_eq!(value.int_at(65535).unwrap(), 65535);
    }

    #[test]
    fn function_transport_preserves_identity_pos_and_verb_occurrence_adapter() {
        for pos in [
            FunctionPartOfSpeech::Verb,
            FunctionPartOfSpeech::Adverb,
            FunctionPartOfSpeech::Conjunction,
        ] {
            let root = FunctionEntity::name_ref("late".into(), pos, 3..7);
            let item = if pos == FunctionPartOfSpeech::Verb {
                Item::verb(Verb {
                    span: 20..24,
                    target: VerbTarget::Named("late".into()),
                    entity: root.clone(),
                })
            } else {
                Item::function(root.clone())
            };
            let count = Arc::strong_count(&root);
            let completed = CompletedParseResult::from_item(item, "test").unwrap();
            assert_eq!(Arc::strong_count(&root), count);
            let item = completed.into_item().unwrap();
            assert_eq!(item.class, ParseClass::from(pos));
            assert_eq!(Arc::strong_count(&root), count);
            match item.value {
                ParseValue::Verb(verb) => {
                    assert_eq!(verb.span, 20..24);
                    assert_eq!(verb.target, VerbTarget::Named("late".into()));
                    assert!(Arc::ptr_eq(&root, &verb.entity));
                }
                ParseValue::Function(function) => assert!(Arc::ptr_eq(&root, &function)),
                _ => panic!(),
            }
            assert_eq!(root.span, 3..7);
            assert_eq!(root.head, FunctionHead::NameRef("late".into()));
        }
    }

    #[test]
    fn analysis_retains_calls_and_names_until_runtime_completes_them_once() {
        let program = parse("outer=:inner=:1+2");
        assert_eq!(program.unwrap_err().kind(), "unsupported");
        for source in ["1+2", "unknown", "(1+2)"] {
            let program = parse(source).unwrap();
            let expr = program.expression.unwrap();
            assert!(!matches!(expr.kind, ExprKind::Literal(_)));
            let item = Item::noun(expr, 1);
            assert_eq!(
                CompletedParseResult::from_item(item, "test")
                    .err()
                    .unwrap()
                    .kind(),
                "unsupported"
            );
        }
        struct Host {
            log: Vec<String>,
            version: u64,
        }
        impl RuntimeParserHost for Host {
            fn lookup(&mut self, _: &str) -> Result<Option<ParserNameBinding>> {
                panic!("no name reads")
            }
            fn version(&self, _: &str) -> Option<crate::semantic::NameVersion> {
                Some(crate::semantic::NameVersion(self.version))
            }
            fn apply(&mut self, expression: Expr) -> Result<Value> {
                self.log.push("apply".into());
                let ExprKind::Dyad { left, right, .. } = expression.kind else {
                    panic!()
                };
                crate::kernels::dyad(
                    "+",
                    completed_noun(*left, "test")?,
                    completed_noun(*right, "test")?,
                )
            }
            fn assign(&mut self, name: &str, value: JEntity) -> Result<JEntity> {
                self.log.push(format!("assign:{name}"));
                self.version += 1;
                Ok(value)
            }
        }
        let mut host = Host {
            log: vec![],
            version: 0,
        };
        let mut capture = ParseCapture::default();
        let program =
            parse_runtime_host("outer=:inner=:1+2", &mut host, Some(&mut capture)).unwrap();
        capture.verify().unwrap();
        assert_eq!(host.log, ["apply", "assign:inner", "assign:outer"]);
        let value = completed_noun(program.expression.unwrap(), "test").unwrap();
        assert_eq!(value.int_at(0).unwrap(), 3);
        assert_eq!(
            capture
                .events
                .iter()
                .filter(|e| matches!(e, CaptureEvent::ApplySuccess { .. }))
                .count(),
            1
        );
        assert_eq!(
            capture
                .events
                .iter()
                .filter(|e| matches!(e, CaptureEvent::Commit { .. }))
                .count(),
            2
        );
    }
}

#[cfg(test)]
mod completed_constructor_operand_tests {
    use super::*;
    use crate::{semantic::JEntityRef, storage::CpuStorage, value::Data};

    #[test]
    fn modifier_train_freezes_grouped_owned_noun_once_and_reuses_after_train_drop() {
        let value = Value::ints([65536], (0..65536).collect()).unwrap();
        let Data::Int(data) = value.data() else {
            panic!()
        };
        let pointer = data.as_ptr();
        let noun = Item::noun(
            Expr {
                origin: None,
                span: 2..8,
                kind: ExprKind::Group(Box::new(Expr {
                    origin: None,
                    span: 3..7,
                    kind: ExprKind::Literal(value),
                })),
            },
            1,
        )
        .with_span(1..9);
        let conjunction =
            FunctionEntity::name_ref("rank".into(), FunctionPartOfSpeech::Conjunction, 10..14);
        let train = modifier_train(
            vec![noun, Item::function(conjunction.clone())],
            ParseClass::Adverb,
        )
        .unwrap();
        assert_eq!(train.span, 1..14);
        assert_eq!(train.head, FunctionHead::ModifierTrain);
        assert_eq!(train.result_pos, FunctionPartOfSpeech::Adverb);
        let operand = &train.operands[0];
        assert_eq!(operand.span(), &(1..9));
        let JEntityRef::Noun(value) = operand.as_entity_ref() else {
            panic!()
        };
        let Data::Int(CpuStorage::Shared(data)) = value.data() else {
            panic!()
        };
        assert_eq!(data.as_ptr(), pointer);
        let uses_before = Arc::strong_count(data);
        let reused = modifier_operand(operand, 30..40);
        assert_eq!(Arc::strong_count(data), uses_before + 1);
        let FunctionOperand::Function(retained) = &train.operands[1] else {
            panic!()
        };
        assert!(Arc::ptr_eq(retained, &conjunction));
        drop(train);
        assert_eq!(reused.span(), 30..40);
        let value = completed_noun(reused.into_noun().unwrap().0, "test").unwrap();
        let Data::Int(data) = value.data() else {
            panic!()
        };
        assert_eq!(data.as_ptr(), pointer);
        assert_eq!(value.int_at(65535).unwrap(), 65535);
    }

    #[test]
    fn modifier_train_moves_all_function_pos_and_rejects_deferred_nouns() {
        for (pos, result) in [
            (FunctionPartOfSpeech::Verb, ParseClass::Adverb),
            (FunctionPartOfSpeech::Adverb, ParseClass::Conjunction),
            (FunctionPartOfSpeech::Conjunction, ParseClass::Conjunction),
        ] {
            let left =
                FunctionEntity::name_ref("left".into(), FunctionPartOfSpeech::Conjunction, 0..4);
            let right = FunctionEntity::name_ref("right".into(), pos, 5..10);
            let item = if pos == FunctionPartOfSpeech::Verb {
                Item::verb(Verb {
                    span: 5..10,
                    target: VerbTarget::Named("right".into()),
                    entity: right.clone(),
                })
            } else {
                Item::function(right.clone())
            };
            assert_eq!(
                bident_disposition(ParseClass::Conjunction, pos.into()),
                BidentDisposition::BuildDerivedModifier(result)
            );
            let before = Arc::strong_count(&right);
            let train = modifier_train(vec![Item::function(left.clone()), item], result).unwrap();
            assert_eq!(Arc::strong_count(&right), before);
            assert_eq!(ParseClass::from(train.result_pos), result);
            let FunctionOperand::Function(child) = &train.operands[1] else {
                panic!()
            };
            assert!(Arc::ptr_eq(child, &right));
            assert_eq!(child.result_pos, pos);
            assert_eq!(child.head, FunctionHead::NameRef("right".into()));
        }
        for source in ["unknown", "1+2", "(1+2)"] {
            let noun = parse(source).unwrap().expression.unwrap();
            let conjunction =
                FunctionEntity::name_ref("rank".into(), FunctionPartOfSpeech::Conjunction, 20..24);
            let error = modifier_train(
                vec![Item::noun(noun, 1), Item::function(conjunction)],
                ParseClass::Adverb,
            )
            .unwrap_err();
            assert_eq!(error.kind(), "unsupported");
        }
        let error =
            modifier_train(vec![Item::mark(0), Item::mark(1)], ParseClass::Adverb).unwrap_err();
        assert_eq!(error.kind(), "syntax error");
    }
}

#[cfg(test)]
mod rank_constructor_transport_tests {
    use super::*;
    use crate::{primitive::ConjunctionId, value::Data};

    #[test]
    fn rank_constructor_moves_both_nouns_and_preserves_expression_provenance() {
        let left = Value::ints([65536], (0..65536).collect()).unwrap();
        let right = Value::ints([3], vec![0, 1, 2]).unwrap();
        let pointer = |value: &Value| {
            let Data::Int(data) = value.data() else {
                panic!()
            };
            data.as_ptr()
        };
        let left_pointer = pointer(&left);
        let right_pointer = pointer(&right);
        let verb = apply_conjunction_at(
            Item::noun(
                Expr {
                    origin: None,
                    span: 1..8,
                    kind: ExprKind::Literal(left),
                },
                0,
            )
            .with_span(0..9),
            FunctionEntity::primitive_conjunction(ConjunctionId::Rank, 9..10),
            Item::noun(
                Expr {
                    origin: None,
                    span: 11..16,
                    kind: ExprKind::Literal(right),
                },
                0,
            )
            .with_span(10..17),
            0..17,
            0,
            ConstructionNames::default(),
        )
        .unwrap();
        let [
            FunctionOperand::Noun {
                value: left,
                span: left_span,
            },
            FunctionOperand::Noun {
                value: right,
                span: right_span,
            },
        ] = verb.entity.operands.as_slice()
        else {
            panic!()
        };
        assert_eq!(pointer(left), left_pointer);
        assert_eq!(pointer(right), right_pointer);
        assert_eq!(left_span, &(1..8));
        assert_eq!(right_span, &(11..16));
        assert_eq!(left.int_at(65535).unwrap(), 65535);
        assert_eq!(right.int_at(2).unwrap(), 2);
    }

    #[test]
    fn invalid_right_rank_precedes_deferred_left_and_atop_keeps_domain_precedence() {
        let deferred = || {
            Item::noun(
                Expr {
                    origin: None,
                    span: 0..4,
                    kind: ExprKind::ReadName("late".into()),
                },
                0,
            )
        };
        let rank = || FunctionEntity::primitive_conjunction(ConjunctionId::Rank, 4..5);
        let noun = |value| {
            Item::noun(
                Expr {
                    origin: None,
                    span: 5..9,
                    kind: ExprKind::Literal(value),
                },
                0,
            )
        };
        for (value, kind) in [
            (Value::ints([2, 2], vec![0; 4]).unwrap(), "rank error"),
            (Value::ints([4], vec![0; 4]).unwrap(), "length error"),
            (
                Value::new(
                    [1],
                    Data::Char(crate::storage::CpuStorage::Owned(vec![b'x'])),
                )
                .unwrap(),
                "domain error",
            ),
        ] {
            let error = apply_conjunction_at(
                deferred(),
                rank(),
                noun(value),
                0..9,
                0,
                ConstructionNames::default(),
            )
            .unwrap_err();
            assert_eq!(error.kind(), kind);
        }
        let error = apply_conjunction_at(
            deferred(),
            rank(),
            noun(Value::scalar(0)),
            0..9,
            0,
            ConstructionNames::default(),
        )
        .unwrap_err();
        assert_eq!(error.kind(), "unsupported");
        let error = apply_conjunction_at(
            deferred(),
            FunctionEntity::primitive_conjunction(ConjunctionId::Atop, 4..5),
            deferred(),
            0..9,
            0,
            ConstructionNames::default(),
        )
        .unwrap_err();
        assert_eq!(error.kind(), "domain error");
    }
}

#[cfg(test)]
mod fork_definition_transport_tests {
    use super::*;
    use crate::{storage::CpuStorage, value::Data};

    fn named(name: &str, span: std::ops::Range<usize>) -> Verb {
        Verb {
            span: span.clone(),
            target: VerbTarget::Named(name.into()),
            entity: FunctionEntity::name_ref(name.into(), FunctionPartOfSpeech::Verb, span),
        }
    }

    #[test]
    fn owned_fork_constant_is_frozen_once_and_shared_after_fork_drops() {
        let value = Value::ints([65536], (0..65536).collect()).unwrap();
        let Data::Int(data) = value.data() else {
            panic!()
        };
        let pointer = data.as_ptr();
        let g = named("g", 10..11);
        let h = named("h", 12..13);
        let original_g = g.entity.clone();
        let original_h = h.entity.clone();
        let fork = train_noun_fork(
            Expr {
                origin: None,
                span: 1..9,
                kind: ExprKind::Group(Box::new(Expr {
                    origin: None,
                    span: 2..8,
                    kind: ExprKind::Literal(value),
                })),
            },
            g,
            h,
        )
        .unwrap();
        assert_eq!(fork.span, 1..13);
        let [
            FunctionOperand::Noun { value, span },
            FunctionOperand::Function(g),
            FunctionOperand::Function(h),
        ] = fork.entity.operands.as_slice()
        else {
            panic!()
        };
        assert_eq!(span, &(1..9));
        assert!(Arc::ptr_eq(g, &original_g));
        assert!(Arc::ptr_eq(h, &original_h));
        let Data::Int(CpuStorage::Shared(data)) = value.data() else {
            panic!()
        };
        assert_eq!(data.as_ptr(), pointer);
        let count = Arc::strong_count(data);
        let first = modifier_operand(&fork.entity.operands[0], 20..30);
        let second = modifier_operand(&fork.entity.operands[0], 40..50);
        assert_eq!(Arc::strong_count(data), count + 2);
        drop(fork);
        for (item, span) in [(first, 20..30), (second, 40..50)] {
            assert_eq!(item.span(), span);
            let value = completed_noun(item.into_noun().unwrap().0, "test").unwrap();
            let Data::Int(data) = value.data() else {
                panic!()
            };
            assert_eq!(data.as_ptr(), pointer);
            assert_eq!(value.int_at(65535).unwrap(), 65535);
        }
        let error = train_noun_fork(
            Expr {
                origin: None,
                span: 0..4,
                kind: ExprKind::ReadName("late".into()),
            },
            named("g", 5..6),
            named("h", 7..8),
        )
        .unwrap_err();
        assert_eq!(error.kind(), "unsupported");
    }

    #[test]
    fn definition_constructor_keeps_class_guards_before_deferred_inputs() {
        let operator = enqueue("3 : 'y'")
            .unwrap()
            .into_iter()
            .find_map(|word| match word.payload {
                EnqueuedPayload::Function(f) => Some(f),
                _ => None,
            })
            .unwrap();
        let deferred = || {
            Item::noun(
                Expr {
                    origin: None,
                    span: 0..4,
                    kind: ExprKind::ReadName("late".into()),
                },
                0,
            )
        };
        let scalar = |value| {
            Item::noun(
                Expr {
                    origin: None,
                    span: 0..1,
                    kind: ExprKind::Literal(Value::scalar(value)),
                },
                0,
            )
        };
        let cases = [
            (deferred(), Item::verb(named("f", 5..6)), "domain error"),
            (Item::verb(named("f", 0..1)), deferred(), "domain error"),
            (deferred(), scalar(3), "unsupported"),
            (scalar(3), deferred(), "unsupported"),
            (scalar(3), scalar(7), "domain error"),
        ];
        for (left, right, kind) in cases {
            let error = apply_conjunction_items(
                left,
                operator.clone(),
                right,
                0..9,
                0,
                ConstructionNames::default(),
            )
            .err()
            .unwrap();
            assert_eq!(error.kind(), kind);
        }
    }
}

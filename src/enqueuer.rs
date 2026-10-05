//! J word interpretation between word formation and parsing.
//!
//! Word formation decides byte boundaries.  This module interprets those words
//! into parser-facing classes and semantic payloads.  It deliberately contains
//! no target, schedule, buffer or backend choices.

use crate::storage::{CpuStorage, Shape};
use crate::{
    error::{DiagnosticPhase, Error, ErrorContext, Result},
    value::{Data, Value},
};
use std::{borrow::Cow, ops::Range};

pub use crate::types::Scalar;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnqueueClass {
    Noun,
    Name,
    Verb,
    Adverb,
    Conjunction,
    Assignment,
    LeftParen,
    RightParen,
}

/// jtenqueue's sentence environment; explicit bodies retain local copulas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnqueueEnvironment {
    TopLevel,
    ExplicitDefinition,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EnqueueFlags {
    /// Ordinary names are resolved at parser-stack entry. Assignment targets
    /// deliberately keep this false, matching jsource QCNAMEASSIGNED.
    pub lookup_name: bool,
    /// Copula metadata retained for parser-time assignment semantics.
    pub global_assignment: bool,
    pub local_assignment: bool,
    pub assignment_to_name: bool,
}

#[derive(Clone, Debug)]
pub enum EnqueuedPayload<'a> {
    Scalar(Scalar),
    Noun(Box<Value>),
    Name(&'a str),
    Verb(crate::primitive::PrimitiveId),
    Adverb(crate::primitive::AdverbId),
    Conjunction(crate::primitive::ConjunctionId),
    Function(std::sync::Arc<crate::semantic::FunctionEntity>),
    Assign,
    Open,
    Close,
}

#[derive(Clone, Debug)]
pub struct EnqueuedWord<'a> {
    pub class: EnqueueClass,
    pub payload: EnqueuedPayload<'a>,
    pub span: Range<usize>,
    /// Original parse-visible word index.  Parser reductions must preserve this
    /// provenance so diagnostics can blame the source word after reduction.
    pub word_index: usize,
    pub flags: EnqueueFlags,
}

fn numeric_text(s: &str) -> Cow<'_, str> {
    if s.contains('_') {
        Cow::Owned(s.replace('_', "-"))
    } else {
        Cow::Borrowed(s)
    }
}

fn numeric_failure(s: &str) -> Error {
    // wn.c::numcase recognizes these alternate families. Do not report a J
    // lexical error just because RustJ has not implemented their constructors.
    if s.bytes()
        .any(|b| matches!(b, b'a' | b'b' | b'j' | b'p' | b'r' | b'x' | b'f'))
    {
        Error::Unsupported(format!("numeric literal {s}"))
    } else {
        Error::IllFormedNumber
    }
}

fn parse_int(s: &str) -> Result<i64> {
    numeric_text(s)
        .parse()
        .map_err(|e: std::num::ParseIntError| match e.kind() {
            std::num::IntErrorKind::PosOverflow | std::num::IntErrorKind::NegOverflow => {
                Error::Unsupported("overflowing integer literal conversion".into())
            }
            _ => numeric_failure(s),
        })
}

fn parse_float(s: &str) -> Result<f64> {
    match s {
        "_" => Ok(f64::INFINITY),
        "__" => Ok(f64::NEG_INFINITY),
        "_." => Ok(f64::NAN),
        _ => numeric_text(s).parse().map_err(|_| numeric_failure(s)),
    }
}

/// C t.c CALP/CACE are permanent immutable nouns, independent of function
/// construction/execution support. Share payloads across enqueue occurrences.
fn core_noun(word: &str) -> Option<Value> {
    use std::sync::OnceLock;
    static ALPHABET: OnceLock<Value> = OnceLock::new();
    static ACE: OnceLock<Value> = OnceLock::new();
    match word {
        "a." => Some(
            ALPHABET
                .get_or_init(|| {
                    Value::new([256], Data::Char(CpuStorage::new((0..=255).collect())))
                        .expect("alphabet has 256 bytes")
                        .into_shared()
                })
                .clone(),
        ),
        "a:" => Some(
            ACE.get_or_init(|| {
                Value::boxed(
                    Value::new([0], Data::Bool(CpuStorage::new(Vec::new())))
                        .expect("empty noun has zero atoms"),
                )
            })
            .clone(),
        ),
        _ => None,
    }
}

fn interpret_word<'a>(
    word: &'a str,
    span: &Range<usize>,
    primitives: &crate::primitive::PrimitiveContext,
) -> Result<(EnqueueClass, EnqueuedPayload<'a>, EnqueueFlags)> {
    if let Some(value) = core_noun(word) {
        return Ok((
            EnqueueClass::Noun,
            EnqueuedPayload::Noun(Box::new(value)),
            EnqueueFlags::default(),
        ));
    }
    let fixed = match word {
        "=:" => Some((
            EnqueueClass::Assignment,
            EnqueuedPayload::Assign,
            EnqueueFlags {
                global_assignment: true,
                ..EnqueueFlags::default()
            },
        )),
        "=." => Some((
            EnqueueClass::Assignment,
            EnqueuedPayload::Assign,
            EnqueueFlags {
                local_assignment: true,
                ..EnqueueFlags::default()
            },
        )),
        "(" => Some((
            EnqueueClass::LeftParen,
            EnqueuedPayload::Open,
            EnqueueFlags::default(),
        )),
        ")" => Some((
            EnqueueClass::RightParen,
            EnqueuedPayload::Close,
            EnqueueFlags::default(),
        )),
        _ => primitives.resolve_core_for_enqueue(word).map(|handle| {
            use crate::primitive::{PrimitivePartOfSpeech, PrimitiveSemanticId};
            let (class, payload) = match (handle.result_pos, handle.semantic_id) {
                (PrimitivePartOfSpeech::Verb, PrimitiveSemanticId::Verb(id)) => {
                    (EnqueueClass::Verb, EnqueuedPayload::Verb(id))
                }
                (PrimitivePartOfSpeech::Adverb, PrimitiveSemanticId::Adverb(id)) => {
                    (EnqueueClass::Adverb, EnqueuedPayload::Adverb(id))
                }
                (PrimitivePartOfSpeech::Conjunction, PrimitiveSemanticId::Conjunction(id)) => {
                    (EnqueueClass::Conjunction, EnqueuedPayload::Conjunction(id))
                }
                (pos, PrimitiveSemanticId::Vocabulary(id)) => {
                    let class = match pos {
                        PrimitivePartOfSpeech::Verb => EnqueueClass::Verb,
                        PrimitivePartOfSpeech::Adverb => EnqueueClass::Adverb,
                        PrimitivePartOfSpeech::Conjunction => EnqueueClass::Conjunction,
                    };
                    (
                        class,
                        EnqueuedPayload::Function(crate::semantic::FunctionEntity::derived(
                            crate::semantic::FunctionHead::VocabularyPrimitive(id),
                            pos.into(),
                            span.clone(),
                            Vec::new(),
                        )),
                    )
                }
                _ => unreachable!("primitive handle POS must match semantic ID"),
            };
            (class, payload, EnqueueFlags::default())
        }),
    };
    if let Some(fixed) = fixed {
        return Ok(fixed);
    }

    // w.c::jtenqueue checks installed spellin/ds entries first. An
    // unregistered inflection is a spelling error, except name_: (by-value
    // lookup/abandon), which is valid syntax with a separate runtime contract.
    // Numeric dots belong to connum; numeric colons require a registered
    // one-digit constant function. Do not invent arbitrary obsolete-word lists.
    let numeric = word.as_bytes()[0].is_ascii_digit() || word.starts_with('_');
    if word.ends_with(':') || (!numeric && word.ends_with('.')) {
        if word.as_bytes()[0].is_ascii_alphabetic() && word.ends_with("_:") {
            validate_name_syntax(&word[..word.len() - 2])?;
            return Err(Error::Unsupported("J name-by-value/abandon lookup".into()));
        }
        return Err(Error::Spelling);
    }

    if word.starts_with('\'') {
        let bytes = word.as_bytes();
        let mut value = Vec::new();
        let mut i = 1;
        while i + 1 < bytes.len() {
            value.push(bytes[i]);
            i += if bytes[i] == b'\'' { 2 } else { 1 };
        }
        if value.len() == 1 {
            return Ok((
                EnqueueClass::Noun,
                EnqueuedPayload::Scalar(Scalar::Char(value[0])),
                EnqueueFlags::default(),
            ));
        }
        return Ok((
            EnqueueClass::Noun,
            EnqueuedPayload::Noun(Box::new(Value::new(
                [value.len()],
                Data::Char(CpuStorage::new(value)),
            )?)),
            EnqueueFlags::default(),
        ));
    }

    if numeric {
        let fields = word.split_ascii_whitespace().count();
        let is_float = word
            .split_ascii_whitespace()
            .any(|part| part.contains(['.', 'e', 'E']) || part == "_" || part == "__");
        if fields == 1 {
            let value = if is_float {
                Scalar::Float(parse_float(word)?)
            } else {
                match parse_int(word)? {
                    x @ (0 | 1) => Scalar::Bool(x != 0),
                    x => Scalar::Int(x),
                }
            };
            return Ok((
                EnqueueClass::Noun,
                EnqueuedPayload::Scalar(value),
                EnqueueFlags::default(),
            ));
        }
        let data = if is_float {
            Data::Float(CpuStorage::new(
                word.split_ascii_whitespace()
                    .map(parse_float)
                    .collect::<Result<Vec<_>>>()?,
            ))
        } else {
            let values = word
                .split_ascii_whitespace()
                .map(parse_int)
                .collect::<Result<Vec<_>>>()?;
            if values.iter().all(|&n| n == 0 || n == 1) {
                Data::Bool(CpuStorage::new(
                    values.into_iter().map(|n| n as u8).collect(),
                ))
            } else {
                Data::Int(CpuStorage::new(values))
            }
        };
        return Ok((
            EnqueueClass::Noun,
            EnqueuedPayload::Noun(Box::new(Value::new(Shape::from([fields]), data)?)),
            EnqueueFlags::default(),
        ));
    }

    if word.as_bytes()[0].is_ascii_alphabetic()
        && word.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        // jsource vnm accepts ordinary underscores inside simple names.
        // Trailing '_' and '__' introduce direct/indirect locatives, which are
        // a separate name-resolution feature not implemented by this frontend.
        // sn.c::vnm rejects a trailing single underscore without a preceding
        // locale separator. foo__ is a valid base-locale name, still unsupported.
        validate_name_syntax(word)?;
        if word.ends_with('_') || word.contains("__") {
            return Err(Error::Unsupported("J locative names".into()));
        }
        return Ok((
            EnqueueClass::Name,
            EnqueuedPayload::Name(word),
            EnqueueFlags::default(),
        ));
    }

    // Remaining nonnumeric, nonquoted, nonname words are invalid characters
    // or uninstalled primitive spellings, not missing execution capabilities.
    Err(Error::Spelling)
}

// Bounded ASCII name grammar from sn.c::vnm/vlocnm, with no locale lookup.
// NAME allocation length limits and dynamic lookup are separate contracts.
fn validate_name_syntax(word: &str) -> Result<()> {
    let bytes = word.as_bytes();
    if bytes.is_empty()
        || !bytes[0].is_ascii_alphabetic()
        || !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
    {
        return Err(Error::IllFormedName);
    }
    let indirect = word.find("__");
    let valid = if let Some(untrailed) = word.strip_suffix('_') {
        if let Some(first) = indirect {
            // Only the final __ can denote the empty/base direct locale.
            first == word.len() - 2
        } else if let Some(separator) = untrailed.rfind('_') {
            let locale = &bytes[separator + 1..bytes.len() - 1];
            !locale.is_empty()
                && (locale[0].is_ascii_alphabetic()
                    || (locale.iter().all(u8::is_ascii_digit)
                        && (locale.len() == 1 || (locale[0] != b'0' && locale.len() <= 18))))
        } else {
            false
        }
    } else if let Some(first) = indirect {
        let mut components = word[first + 2..].split("__").peekable();
        let mut valid = true;
        while let Some(component) = components.next() {
            let part = component.as_bytes();
            let named = part.first().is_some_and(u8::is_ascii_alphabetic)
                && part.iter().all(u8::is_ascii_alphanumeric);
            let digits = part.strip_prefix(b"_").unwrap_or(part);
            let numeric = components.peek().is_none()
                && !digits.is_empty()
                && digits.iter().all(u8::is_ascii_digit);
            if !named && !numeric {
                valid = false;
                break;
            }
        }
        valid
    } else {
        true
    };
    if valid {
        Ok(())
    } else {
        Err(Error::IllFormedName)
    }
}

/// Interpret parse-visible words after word formation.
///
/// This is the F1 boundary: parser code receives typed enqueue records rather
/// than reclassifying raw spelling.  Full jsource name-resolution timing and
/// modifier result-POS semantics are subsequent F1/P-stage work.
pub fn enqueue(source: &str) -> Result<Vec<EnqueuedWord<'_>>> {
    enqueue_with_context(source, &crate::primitive::PrimitiveContext::core())
}

pub fn enqueue_with_context<'a>(
    source: &'a str,
    primitives: &crate::primitive::PrimitiveContext,
) -> Result<Vec<EnqueuedWord<'a>>> {
    enqueue_in_environment(source, primitives, EnqueueEnvironment::TopLevel)
}

/// Classify an explicit-definition body without turning local assignment global.
/// Selecting this environment does not implement local symbol tables or execution.
pub fn enqueue_in_environment<'a>(
    source: &'a str,
    primitives: &crate::primitive::PrimitiveContext,
    environment: EnqueueEnvironment,
) -> Result<Vec<EnqueuedWord<'a>>> {
    let definitions = match crate::definition_input::frame(source)? {
        crate::definition_input::InputFrame::Definition(input) => vec![input],
        crate::definition_input::InputFrame::Definitions(inputs) => inputs,
        crate::definition_input::InputFrame::NeedMore => {
            return Err(Error::Syntax("unterminated definition input".into()));
        }
        crate::definition_input::InputFrame::Sentence => Vec::new(),
    };
    let spans = if !definitions.is_empty() {
        let mut spans = Vec::new();
        let mut previous_end = 0;
        for input in &definitions {
            spans.extend(
                crate::tokenizer::parse_word_spans(
                    &source.as_bytes()[previous_end..input.span.start],
                )?
                .into_iter()
                .map(|s| previous_end + s.start..previous_end + s.end),
            );
            spans.push(input.span.clone());
            previous_end = input.span.end;
        }
        spans.extend(
            crate::tokenizer::parse_word_spans(&source.as_bytes()[previous_end..])?
                .into_iter()
                .map(|s| previous_end + s.start..previous_end + s.end),
        );
        spans
    } else {
        crate::tokenizer::parse_word_spans(source.as_bytes())
            .map_err(|error| error.in_phase(DiagnosticPhase::WordFormation))?
    };
    let origins = definitions
        .iter()
        .any(|input| input.form != crate::definition_input::DefinitionForm::NounDirect)
        .then(|| {
            (
                std::sync::Arc::<str>::from(source),
                std::sync::Arc::new(primitives.clone()),
            )
        });
    let mut out = Vec::with_capacity(spans.len());
    for span in spans {
        let word = source
            .get(span.clone())
            .ok_or_else(|| Error::Unsupported("non-UTF-8 word".into()).at(span.clone()))?;
        if let Some(input) = definitions.iter().find(|input| input.span == span) {
            if input.form == crate::definition_input::DefinitionForm::NounDirect {
                let body = input.body_text(source)?.as_bytes().to_vec();
                let payload = if body.len() == 1 {
                    EnqueuedPayload::Scalar(Scalar::Char(body[0]))
                } else {
                    EnqueuedPayload::Noun(Box::new(Value::new(
                        [body.len()],
                        Data::Char(CpuStorage::new(body)),
                    )?))
                };
                out.push(EnqueuedWord {
                    class: EnqueueClass::Noun,
                    payload,
                    span,
                    word_index: out.len(),
                    flags: EnqueueFlags::default(),
                });
                continue;
            }

            let mode = match input.form {
                crate::definition_input::DefinitionForm::Direct => 9,
                crate::definition_input::DefinitionForm::NounDirect => {
                    unreachable!("noun DD handled above")
                }
                crate::definition_input::DefinitionForm::ExplicitString(m)
                | crate::definition_input::DefinitionForm::ExplicitBlock(m) => m,
            };
            let (source_origin, primitive_origin) = origins.as_ref().unwrap();
            let provenance = std::sync::Arc::new(crate::definition_code::DefinitionSource {
                source: source_origin.clone(),
                input: input.clone(),
                primitives: primitive_origin.clone(),
            });
            let operator = crate::semantic::FunctionEntity::derived(
                crate::semantic::FunctionHead::DefinitionConstructor(provenance),
                crate::semantic::FunctionPartOfSpeech::Conjunction,
                span.clone(),
                Vec::new(),
            );
            let body = crate::definition_code::semantic_body(source, input)?
                .as_bytes()
                .to_vec();
            let body_payload = if body.len() == 1 {
                EnqueuedPayload::Scalar(Scalar::Char(body[0]))
            } else {
                EnqueuedPayload::Noun(Box::new(Value::new(
                    [body.len()],
                    Data::Char(CpuStorage::new(body)),
                )?))
            };
            let mut expanded = Vec::new();
            if input.form == crate::definition_input::DefinitionForm::Direct {
                expanded.push((EnqueueClass::LeftParen, EnqueuedPayload::Open));
            }
            expanded.extend([
                (
                    EnqueueClass::Noun,
                    EnqueuedPayload::Scalar(if mode == 1 {
                        Scalar::Bool(true)
                    } else {
                        Scalar::Int(mode.into())
                    }),
                ),
                (
                    EnqueueClass::Conjunction,
                    EnqueuedPayload::Function(operator),
                ),
                (EnqueueClass::Noun, body_payload),
            ]);
            if input.form == crate::definition_input::DefinitionForm::Direct {
                expanded.push((EnqueueClass::RightParen, EnqueuedPayload::Close));
            }
            for (class, payload) in expanded {
                out.push(EnqueuedWord {
                    class,
                    payload,
                    span: span.clone(),
                    word_index: out.len(),
                    flags: EnqueueFlags::default(),
                });
            }
            continue;
        }
        let interpreted = interpret_word(word, &span, primitives);
        let (class, payload, flags) = interpreted.map_err(|error| {
            error.with_context(
                ErrorContext::phase(DiagnosticPhase::Enqueue)
                    .with_span(span.clone())
                    .with_blame_word(out.len()),
            )
        })?;
        out.push(EnqueuedWord {
            class,
            payload,
            span,
            word_index: out.len(),
            flags,
        });
    }

    // jsource installs ordinary NAMEs as non-lookup first. A NAME becomes
    // lookup when followed by a non-assignment word, or when it is the final
    // word. A NAME immediately before a copula remains the assignment target.
    for index in 0..out.len() {
        if out[index].class == EnqueueClass::Name {
            out[index].flags.lookup_name =
                index + 1 == out.len() || out[index + 1].class != EnqueueClass::Assignment;
        }
        if out[index].class == EnqueueClass::Assignment {
            // w.c::jtenqueue env==1 upgrades local copulas at top level.
            if environment == EnqueueEnvironment::TopLevel && out[index].flags.local_assignment {
                out[index].flags.local_assignment = false;
                out[index].flags.global_assignment = true;
            }
            out[index].flags.assignment_to_name =
                index > 0 && out[index - 1].class == EnqueueClass::Name;
        }
    }

    // jsource rejects a one-word sentence whose sole entity cannot itself be
    // a sentence result.
    if out.len() == 1
        && !matches!(
            out[0].class,
            EnqueueClass::Noun
                | EnqueueClass::Name
                | EnqueueClass::Verb
                | EnqueueClass::Adverb
                | EnqueueClass::Conjunction
        )
    {
        return Err(
            Error::Syntax("single word cannot be a sentence result".into()).with_context(
                ErrorContext::phase(DiagnosticPhase::Enqueue)
                    .with_span(out[0].span.clone())
                    .with_blame_word(out[0].word_index),
            ),
        );
    }

    Ok(out)
}

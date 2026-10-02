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

fn parse_int(s: &str) -> Result<i64> {
    numeric_text(s)
        .parse()
        .map_err(|_| Error::Unsupported(format!("numeric literal {s}")))
}

fn parse_float(s: &str) -> Result<f64> {
    match s {
        "_" => Ok(f64::INFINITY),
        "__" => Ok(f64::NEG_INFINITY),
        "_." => Ok(f64::NAN),
        _ => numeric_text(s)
            .parse()
            .map_err(|_| Error::Unsupported(format!("numeric literal {s}"))),
    }
}

fn interpret_word<'a>(
    word: &'a str,
    span: &Range<usize>,
    primitives: &crate::primitive::PrimitiveContext,
) -> Result<(EnqueueClass, EnqueuedPayload<'a>, EnqueueFlags)> {
    let fixed = match word {
        "=:" => Some((
            EnqueueClass::Assignment,
            EnqueuedPayload::Assign,
            EnqueueFlags {
                global_assignment: true,
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
                _ => unreachable!("primitive handle POS must match semantic ID"),
            };
            (class, payload, EnqueueFlags::default())
        }),
    };
    if let Some(fixed) = fixed {
        return Ok(fixed);
    }

    if word.starts_with("NB..") || word.starts_with("NB.:") {
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

    if word.as_bytes()[0].is_ascii_digit() || word.starts_with('_') {
        if word.ends_with(':') {
            return Err(Error::Unsupported(format!("constant verb {word}")));
        }
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
        if word.ends_with('_') || word.contains("__") {
            return Err(Error::Unsupported("J locative names".into()));
        }
        return Ok((
            EnqueueClass::Name,
            EnqueuedPayload::Name(word),
            EnqueueFlags::default(),
        ));
    }

    Err(Error::Unsupported(format!(
        "word {word:?} at byte {}",
        span.start
    )))
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
    let spans = crate::scanner::parse_word_spans(source.as_bytes())
        .map_err(|error| error.in_phase(DiagnosticPhase::WordFormation))?;
    let mut out = Vec::with_capacity(spans.len());
    for (word_index, span) in spans.into_iter().enumerate() {
        let word = source
            .get(span.clone())
            .ok_or_else(|| Error::Unsupported("non-UTF-8 word".into()).at(span.clone()))?;
        let (class, payload, flags) = interpret_word(word, &span, primitives).map_err(|error| {
            error.with_context(
                ErrorContext::phase(DiagnosticPhase::Enqueue)
                    .with_span(span.clone())
                    .with_blame_word(word_index),
            )
        })?;
        out.push(EnqueuedWord {
            class,
            payload,
            span,
            word_index,
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

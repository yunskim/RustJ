use crate::{enqueuer::EnqueuedPayload, error::Result};

pub use crate::types::Scalar;

#[derive(Clone, Debug)]
pub enum Token<'a> {
    Scalar(Scalar),
    Noun(Box<crate::Value>),
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
pub struct SpannedToken<'a> {
    pub span: std::ops::Range<usize>,
    /// Original parse-visible word index. The parser stack carries this
    /// unchanged so error inference can blame source words after reductions.
    pub word_index: usize,
    pub token: Token<'a>,
}

pub fn lex(source: &str) -> Result<Vec<Token<'_>>> {
    Ok(lex_spanned(source)?.into_iter().map(|t| t.token).collect())
}

/// Compatibility adapter for the current parser.
///
/// Word interpretation is owned by `enqueuer`; this function only maps the
/// enqueue payload into the legacy Token surface until F2 cuts the parser over
/// to EnqueuedWord directly.
pub fn lex_spanned(source: &str) -> Result<Vec<SpannedToken<'_>>> {
    crate::enqueuer::enqueue(source)?
        .into_iter()
        .map(|word| {
            let token = match word.payload {
                EnqueuedPayload::Scalar(value) => Token::Scalar(value),
                EnqueuedPayload::Noun(value) => Token::Noun(value),
                EnqueuedPayload::Name(name) => Token::Name(name),
                EnqueuedPayload::Verb(id) => Token::Verb(id),
                EnqueuedPayload::Adverb(id) => Token::Adverb(id),
                EnqueuedPayload::Conjunction(id) => Token::Conjunction(id),
                EnqueuedPayload::Function(function) => Token::Function(function),
                EnqueuedPayload::Assign => Token::Assign,
                EnqueuedPayload::Open => Token::Open,
                EnqueuedPayload::Close => Token::Close,
            };
            Ok(SpannedToken {
                span: word.span,
                word_index: word.word_index,
                token,
            })
        })
        .collect()
}

/// Stream safety guard, not a function-definition parser. Strings and comments
/// remain single words, so delimiters inside them do not stop the CLI.
pub fn has_definition_syntax(source: &str) -> bool {
    crate::tokenizer::scan_unfinished(source.as_bytes())
        .into_iter()
        .any(|span| matches!(source.get(span), Some("{{" | "}}" | ":" | "define")))
}

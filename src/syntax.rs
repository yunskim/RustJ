use crate::storage::{CpuStorage, Shape};
use crate::{
    error::{Error, Result},
    value::{Data, Value},
};
use std::borrow::Cow;

pub use crate::types::Scalar;
#[derive(Clone, Debug)]
pub enum Token<'a> {
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

#[derive(Clone, Debug)]
pub struct SpannedToken<'a> {
    pub span: std::ops::Range<usize>,
    pub token: Token<'a>,
}

pub fn lex(source: &str) -> Result<Vec<Token<'_>>> {
    Ok(lex_spanned(source)?.into_iter().map(|t| t.token).collect())
}

/// Word formation is complete before unsupported words are rejected here.
pub fn lex_spanned(source: &str) -> Result<Vec<SpannedToken<'_>>> {
    let spans = crate::scanner::scan(source.as_bytes())?;
    let mut out = Vec::new();
    for span in spans {
        let word = source
            .get(span.clone())
            .ok_or_else(|| Error::Unsupported("non-ASCII word".into()))?;
        if word.starts_with("NB.")
            && !word
                .as_bytes()
                .get(3)
                .is_some_and(|b| matches!(b, b'.' | b':'))
        {
            continue;
        }
        let token;
        macro_rules! emit {
            ($value:expr) => {
                token = $value
            };
        }
        let fixed = match word {
            "=:" => Some(Token::Assign),
            "(" => Some(Token::Open),
            ")" => Some(Token::Close),
            _ => crate::primitive::PrimitiveId::from_spelling(word)
                .map(Token::Verb)
                .or_else(|| crate::primitive::AdverbId::from_spelling(word).map(Token::Adverb))
                .or_else(|| {
                    crate::primitive::ConjunctionId::from_spelling(word).map(Token::Conjunction)
                }),
        };
        if let Some(token) = fixed {
            emit!(token);
        } else if word.starts_with("NB..") || word.starts_with("NB.:") {
            return Err(Error::Spelling);
        } else if word.starts_with('\'') {
            let bytes = word.as_bytes();
            let mut value = Vec::new();
            let mut i = 1;
            while i + 1 < bytes.len() {
                value.push(bytes[i]);
                i += if bytes[i] == b'\'' { 2 } else { 1 };
            }
            if value.len() == 1 {
                emit!(Token::Scalar(Scalar::Char(value[0])));
            } else {
                emit!(Token::Noun(Box::new(Value::new(
                    [value.len()],
                    Data::Char(CpuStorage::new(value)),
                )?)));
            }
        } else if word.as_bytes()[0].is_ascii_digit() || word.starts_with('_') {
            if word.ends_with(':') {
                return Err(Error::Unsupported(format!("constant verb {word}")));
            }
            let literal = word;
            let fields = word.split_ascii_whitespace().count();
            let is_float = literal
                .split_ascii_whitespace()
                .any(|s| s.contains(['.', 'e', 'E']) || s == "_" || s == "__");
            if fields == 1 {
                let value = if is_float {
                    Scalar::Float(parse_float(literal)?)
                } else {
                    match parse_int(literal)? {
                        x @ (0 | 1) => Scalar::Bool(x != 0),
                        x => Scalar::Int(x),
                    }
                };
                emit!(Token::Scalar(value));
            } else {
                let data = if is_float {
                    Data::Float(CpuStorage::new(
                        literal
                            .split_ascii_whitespace()
                            .map(parse_float)
                            .collect::<Result<Vec<_>>>()?,
                    ))
                } else {
                    let v = literal
                        .split_ascii_whitespace()
                        .map(parse_int)
                        .collect::<Result<Vec<_>>>()?;
                    if v.iter().all(|&n| n == 0 || n == 1) {
                        Data::Bool(CpuStorage::new(v.into_iter().map(|n| n as u8).collect()))
                    } else {
                        Data::Int(CpuStorage::new(v))
                    }
                };
                emit!(Token::Noun(Box::new(Value::new(
                    Shape::from([fields]),
                    data,
                )?)));
            }
        } else if word.as_bytes()[0].is_ascii_alphabetic()
            && word.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            if word.contains('_') {
                return Err(Error::Unsupported("locatives and underscore names".into()));
            }
            emit!(Token::Name(word));
        } else {
            return Err(Error::Unsupported(format!(
                "word {word:?} at byte {}",
                span.start
            )));
        }
        out.push(SpannedToken { span, token });
    }
    Ok(out)
}

/// Stream safety guard, not a function-definition parser. Strings and comments
/// remain single words, so delimiters inside them do not stop the CLI.
pub fn has_definition_syntax(source: &str) -> bool {
    crate::scanner::scan_unfinished(source.as_bytes())
        .into_iter()
        .any(|span| matches!(source.get(span), Some("{{" | "}}" | ":" | "define")))
}

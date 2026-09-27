use crate::storage::{CpuStorage, Shape};
use crate::{
    error::{Error, Result},
    value::{Data, Value},
};
use std::borrow::Cow;

/// Compact scalar tokens avoid storing the full array header in every token.
#[derive(Clone, Copy, Debug)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    Char(u8),
}
impl Scalar {
    pub fn into_value(self) -> Value {
        let data = match self {
            Self::Bool(x) => Data::Bool(CpuStorage::Inline(x as u8)),
            Self::Int(x) => Data::Int(CpuStorage::Inline(x)),
            Self::Float(x) => Data::Float(CpuStorage::Inline(x)),
            Self::Char(x) => Data::Char(CpuStorage::Inline(x)),
        };
        // The scalar variants guarantee exactly one valid element.
        Value::new([], data).expect("valid scalar token")
    }
}
#[derive(Clone, Debug)]
pub enum Token<'a> {
    Scalar(Scalar),
    Noun(Box<Value>),
    Name(&'a str),
    Verb(&'static str),
    Assign,
    Open,
    Close,
    Slash,
    Rank,
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

pub fn lex(source: &str) -> Result<Vec<Token<'_>>> {
    let b = source.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if source[i..].starts_with("NB.") {
            break;
        }
        if source[i..].starts_with("=:") {
            out.push(Token::Assign);
            i += 2;
            continue;
        }
        if source[i..].starts_with("=.") {
            return Err(Error::Unsupported("local assignment".into()));
        }
        if source[i..].starts_with("i.") {
            out.push(Token::Verb("i."));
            i += 2;
            continue;
        }
        if b[i] == b'\'' {
            i += 1;
            let mut s = Vec::new();
            let mut closed = false;
            while i < b.len() {
                if b[i] == b'\'' {
                    i += 1;
                    if i < b.len() && b[i] == b'\'' {
                        s.push(b'\'');
                        i += 1;
                    } else {
                        closed = true;
                        break;
                    }
                } else {
                    s.push(b[i]);
                    i += 1;
                }
            }
            if !closed {
                return Err(Error::Syntax("unterminated literal".into()));
            }
            if s.len() == 1 {
                out.push(Token::Scalar(Scalar::Char(s[0])));
            } else {
                out.push(Token::Noun(Box::new(Value::new(
                    Shape::from([s.len()]),
                    Data::Char(CpuStorage::new(s)),
                )?)));
            }
            continue;
        }
        if b[i].is_ascii_digit() || b[i] == b'_' {
            let literal_start = i;
            let mut fields = 0;
            let literal_end;
            loop {
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b"_.".contains(&b[i])) {
                    i += 1;
                }
                fields += 1;
                let end = i;
                while i < b.len() && b[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i == end || i == b.len() || !(b[i].is_ascii_digit() || b[i] == b'_') {
                    literal_end = end;
                    break;
                }
            }
            let literal = &source[literal_start..literal_end];
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
                out.push(Token::Scalar(value));
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
                out.push(Token::Noun(Box::new(Value::new(
                    Shape::from([fields]),
                    data,
                )?)));
            }
            continue;
        }
        if b[i].is_ascii_alphabetic() {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let name = &source[start..i];
            if name.contains('_') {
                return Err(Error::Unsupported("locatives and underscore names".into()));
            }
            out.push(Token::Name(name));
            continue;
        }
        let t = match b[i] {
            b'(' => Token::Open,
            b')' => Token::Close,
            b'/' => Token::Slash,
            b'"' => Token::Rank,
            b'+' => Token::Verb("+"),
            b'-' => Token::Verb("-"),
            b'*' => Token::Verb("*"),
            b'%' => Token::Verb("%"),
            b'$' => Token::Verb("$"),
            b'#' => Token::Verb("#"),
            b',' => Token::Verb(","),
            b'=' => Token::Verb("="),
            b'<' => Token::Verb("<"),
            b'>' => Token::Verb(">"),
            b'{' => Token::Verb("{"),
            b'|' => Token::Verb("|"),
            _ => return Err(Error::Unsupported(format!("token at byte {i}"))),
        };
        i += 1;
        out.push(t);
    }
    Ok(out)
}

use crate::{
    error::{Error, Result},
    value::{Data, Value},
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum Token {
    Noun(Value),
    Name(String),
    Verb(String),
    Assign,
    Open,
    Close,
    Slash,
    Rank,
}

pub fn lex(source: &str) -> Result<Vec<Token>> {
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
            out.push(Token::Verb("i.".into()));
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
            let shape = if s.len() == 1 { vec![] } else { vec![s.len()] };
            out.push(Token::Noun(Value::new(shape, Data::Char(Arc::new(s)))?));
            continue;
        }
        if b[i].is_ascii_digit() || b[i] == b'_' {
            let mut nums = Vec::new();
            loop {
                let start = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b"_.".contains(&b[i])) {
                    i += 1;
                }
                nums.push(source[start..i].to_string());
                let end = i;
                while i < b.len() && b[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i == end || i == b.len() || !(b[i].is_ascii_digit() || b[i] == b'_') {
                    break;
                }
            }
            let shape = if nums.len() == 1 {
                vec![]
            } else {
                vec![nums.len()]
            };
            let is_float = nums
                .iter()
                .any(|x| x.contains(['.', 'e', 'E']) || x == "_" || x == "__");
            let data = if is_float {
                let v = nums
                    .iter()
                    .map(|s| match s.as_str() {
                        "_" => Ok(f64::INFINITY),
                        "__" => Ok(f64::NEG_INFINITY),
                        "_." => Ok(f64::NAN),
                        _ => s
                            .replace('_', "-")
                            .parse::<f64>()
                            .map_err(|_| Error::Unsupported(format!("numeric literal {s}"))),
                    })
                    .collect::<Result<Vec<_>>>()?;
                Data::Float(Arc::new(v))
            } else {
                let v = nums
                    .iter()
                    .map(|s| {
                        s.replace('_', "-")
                            .parse::<i64>()
                            .map_err(|_| Error::Unsupported(format!("numeric literal {s}")))
                    })
                    .collect::<Result<Vec<_>>>()?;
                if v.iter().all(|&n| n == 0 || n == 1) {
                    Data::Bool(Arc::new(v.into_iter().map(|n| n as u8).collect()))
                } else {
                    Data::Int(Arc::new(v))
                }
            };
            out.push(Token::Noun(Value::new(shape, data)?));
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
            out.push(Token::Name(name.into()));
            continue;
        }
        let t = match b[i] {
            b'(' => Token::Open,
            b')' => Token::Close,
            b'/' => Token::Slash,
            b'"' => Token::Rank,
            b'+' | b'-' | b'*' | b'%' | b'$' | b'#' | b',' | b'=' | b'<' | b'>' | b'{' | b'|' => {
                Token::Verb((b[i] as char).to_string())
            }
            _ => return Err(Error::Unsupported(format!("token at byte {i}"))),
        };
        i += 1;
        out.push(t);
    }
    Ok(out)
}

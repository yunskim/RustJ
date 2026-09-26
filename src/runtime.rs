use crate::{
    error::{Error, Result},
    kernels,
    syntax::{Token, lex},
    value::Value,
};
use std::collections::HashMap;

#[derive(Default)]
pub struct Engine {
    names: HashMap<String, Value>,
}

#[derive(Clone)]
struct Verb {
    name: String,
    reduce: bool,
    rank: Option<i64>,
}
enum Item {
    Noun(Value),
    Verb(Verb),
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn eval(&mut self, source: &str) -> Result<Option<Value>> {
        let tokens = lex(source)?;
        if tokens.is_empty() {
            return Ok(None);
        }
        let (name, expr) = if tokens.len() > 1 && matches!(tokens[1], Token::Assign) {
            let Token::Name(name) = &tokens[0] else {
                return Err(Error::Syntax("assignment target".into()));
            };
            (Some(name.clone()), &tokens[2..])
        } else {
            (None, tokens.as_slice())
        };
        let mut pos = 0;
        let value = self.expression(expr, &mut pos, false, 0)?;
        if pos != expr.len() {
            return Err(Error::Syntax("trailing tokens".into()));
        }
        if let Some(name) = name {
            self.names.insert(name, value);
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    fn expression(
        &self,
        tokens: &[Token],
        pos: &mut usize,
        nested: bool,
        depth: usize,
    ) -> Result<Value> {
        if depth > 128 {
            return Err(Error::Limit);
        }
        let mut items = Vec::new();
        while *pos < tokens.len() {
            match &tokens[*pos] {
                Token::Close => {
                    if nested {
                        break;
                    } else {
                        return Err(Error::Syntax("unexpected )".into()));
                    }
                }
                Token::Open => {
                    *pos += 1;
                    let v = self.expression(tokens, pos, true, depth + 1)?;
                    if !matches!(tokens.get(*pos), Some(Token::Close)) {
                        return Err(Error::Syntax("missing )".into()));
                    }
                    *pos += 1;
                    items.push(Item::Noun(v));
                }
                Token::Noun(v) => {
                    items.push(Item::Noun(v.clone()));
                    *pos += 1;
                }
                Token::Name(n) => {
                    items.push(Item::Noun(
                        self.names
                            .get(n)
                            .cloned()
                            .ok_or_else(|| Error::Value(n.clone()))?,
                    ));
                    *pos += 1;
                }
                Token::Verb(n) => {
                    let mut verb = Verb {
                        name: n.clone(),
                        reduce: false,
                        rank: None,
                    };
                    *pos += 1;
                    if matches!(tokens.get(*pos), Some(Token::Slash)) {
                        verb.reduce = true;
                        *pos += 1;
                    }
                    if matches!(tokens.get(*pos), Some(Token::Rank)) {
                        *pos += 1;
                        let Some(Token::Noun(v)) = tokens.get(*pos) else {
                            return Err(Error::Syntax("rank needs a scalar integer".into()));
                        };
                        if !v.shape().is_empty() {
                            return Err(Error::Unsupported("rank list".into()));
                        }
                        verb.rank = Some(v.int_at(0)?);
                        *pos += 1;
                    }
                    items.push(Item::Verb(verb));
                }
                _ => {
                    return Err(Error::Unsupported(
                        "assignment/modifier in expression".into(),
                    ));
                }
            }
        }
        let Some(Item::Noun(mut rhs)) = items.pop() else {
            return Err(Error::Syntax("expected right argument".into()));
        };
        while let Some(item) = items.pop() {
            let Item::Verb(v) = item else {
                return Err(Error::Syntax("adjacent nouns".into()));
            };
            if matches!(items.last(), Some(Item::Noun(_))) {
                let Some(Item::Noun(lhs)) = items.pop() else {
                    unreachable!()
                };
                if v.reduce || v.rank.is_some() {
                    return Err(Error::Unsupported("dyadic derived verb".into()));
                }
                rhs = kernels::dyad(&v.name, lhs, rhs)?;
            } else if let Some(rank) = v.rank {
                rhs = kernels::ranked(&v.name, v.reduce, rank, rhs)?;
            } else if v.reduce {
                rhs = kernels::reduce(&v.name, rhs)?;
            } else {
                rhs = kernels::monad(&v.name, rhs)?;
            }
        }
        Ok(rhs)
    }
}

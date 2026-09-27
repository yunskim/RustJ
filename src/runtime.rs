use crate::{
    error::{Error, Result},
    kernels,
    syntax::{Token, lex},
    value::Value,
};
use std::collections::HashMap;

pub struct Engine {
    names: HashMap<String, Value>,
    pool: crate::pool::OutputPool,
}

#[derive(Clone)]
struct Verb {
    name: &'static str,
    reduce: bool,
    rank: Option<[i64; 3]>,
}
enum Item {
    Noun(Value),
    Verb(Verb),
}

impl Default for Engine {
    fn default() -> Self {
        Self::with_output_cache_limit(64 * 1024 * 1024)
    }
}
impl Engine {
    /// Limits retained integer payload bytes. Zero disables caching.
    pub fn with_output_cache_limit(bytes: usize) -> Self {
        Self {
            names: HashMap::new(),
            pool: crate::pool::OutputPool::new(bytes),
        }
    }
    /// Retained payload capacity in bytes and cumulative reuse count.
    pub fn output_cache_stats(&self) -> (usize, usize) {
        self.pool.stats()
    }
    /// Return cached buffers to the allocator; RSS may not decrease.
    pub fn clear_output_cache(&mut self) {
        self.pool.clear();
    }

    pub fn new() -> Self {
        Self::default()
    }
    pub fn eval(&mut self, source: &str) -> Result<Option<Value>> {
        let mut tokens = lex(source)?;
        if tokens.is_empty() {
            return Ok(None);
        }
        let (name, expr) = if tokens.len() > 1 && matches!(tokens[1], Token::Assign) {
            let Token::Name(name) = &tokens[0] else {
                return Err(Error::Syntax("assignment target".into()));
            };
            (Some((*name).to_owned()), &mut tokens[2..])
        } else {
            (None, tokens.as_mut_slice())
        };
        let mut pos = 0;
        let value = self.expression(expr, &mut pos, false, 0)?;
        if pos != expr.len() {
            return Err(Error::Syntax("trailing tokens".into()));
        }
        if let Some(name) = name {
            if let Some(old) = self.names.insert(name, value.into_shared()) {
                self.pool.retire(old);
            }
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    fn expression(
        &mut self,
        tokens: &mut [Token<'_>],
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
                Token::Scalar(v) => {
                    items.push(Item::Noun(v.into_value()));
                    *pos += 1;
                }
                Token::Noun(_) => {
                    let Token::Noun(v) = std::mem::replace(&mut tokens[*pos], Token::Open) else {
                        unreachable!()
                    };
                    items.push(Item::Noun(*v));
                    *pos += 1;
                }
                Token::Name(n) => {
                    items.push(Item::Noun(
                        self.names
                            .get(*n)
                            .cloned()
                            .ok_or_else(|| Error::Value((*n).to_owned()))?,
                    ));
                    *pos += 1;
                }
                Token::Verb(n) => {
                    let mut verb = Verb {
                        name: n,
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
                        let v = match tokens.get(*pos) {
                            Some(Token::Scalar(v)) => v.into_value(),
                            Some(Token::Noun(v)) => (**v).clone(),
                            _ => return Err(Error::Syntax("rank needs a numeric literal".into())),
                        };
                        if v.is_empty() || v.len() > 3 {
                            return Err(Error::Length);
                        }
                        let at = |i| v.int_at(i);
                        verb.rank = Some(match v.len() {
                            1 => [at(0)?, at(0)?, at(0)?],
                            2 => [at(1)?, at(0)?, at(1)?],
                            _ => [at(0)?, at(1)?, at(2)?],
                        });
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
                if v.reduce {
                    return Err(Error::Unsupported("dyadic derived verb".into()));
                }
                if let Some(rank) = v.rank {
                    rhs = kernels::ranked_dyad_ranks(v.name, rank[1], rank[2], lhs, rhs)?;
                    continue;
                }
                rhs = match v.name {
                    "+" => kernels::atomic_with_pool(kernels::Op::Add, lhs, rhs, &mut self.pool)?,
                    "-" => kernels::atomic_with_pool(kernels::Op::Sub, lhs, rhs, &mut self.pool)?,
                    "*" => kernels::atomic_with_pool(kernels::Op::Mul, lhs, rhs, &mut self.pool)?,
                    _ => kernels::dyad(v.name, lhs, rhs)?,
                };
            } else if let Some(rank) = v.rank {
                rhs = kernels::ranked(v.name, v.reduce, rank[0], rhs)?;
            } else if v.reduce {
                rhs = kernels::reduce(v.name, rhs)?;
            } else {
                rhs = kernels::monad(v.name, rhs)?;
            }
        }
        Ok(rhs)
    }
}

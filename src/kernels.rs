use crate::{
    error::{Error, Result},
    value::{Data, Value, buffer, count},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Lt,
    Gt,
}

fn agreement(a: &Value, b: &Value) -> Result<(Vec<usize>, usize, usize)> {
    let (short, long) = if a.shape.len() <= b.shape.len() {
        (&a.shape, &b.shape)
    } else {
        (&b.shape, &a.shape)
    };
    if !long.starts_with(short) {
        return Err(Error::Length);
    }
    let ad = count(&long[a.shape.len()..])?;
    let bd = count(&long[b.shape.len()..])?;
    Ok((long.clone(), ad, bd))
}

fn integer(op: Op, a: i64, b: i64) -> Option<i64> {
    match op {
        Op::Add => a.checked_add(b),
        Op::Sub => a.checked_sub(b),
        Op::Mul => a.checked_mul(b),
        _ => None,
    }
}
fn real(op: Op, a: f64, b: f64) -> f64 {
    match op {
        Op::Add => a + b,
        Op::Sub => a - b,
        Op::Mul => {
            if a == 0.0 || b == 0.0 {
                0.0
            } else {
                a * b
            }
        }
        Op::Div => {
            if a == 0.0 && b == 0.0 {
                0.0
            } else {
                a / b
            }
        }
        _ => unreachable!(),
    }
}
fn near(a: f64, b: f64) -> bool {
    a == b
        || (a.is_finite()
            && b.is_finite()
            && (a - b).abs() <= 2f64.powi(-44) * a.abs().max(b.abs()))
}

fn int_pair<const OP: u8>(
    a: Value,
    b: Value,
    shape: Vec<usize>,
    ad: usize,
    bd: usize,
) -> Result<Value> {
    let (Data::Int(x), Data::Int(y)) = (a.data, b.data) else {
        unreachable!()
    };
    let data = crate::numeric::int::<OP>(x, y, count(&shape)?, ad, bd)?;
    Value::new(shape, data)
}
pub fn atomic(op: Op, a: Value, b: Value) -> Result<Value> {
    let (shape, ad, bd) = agreement(&a, &b)?;
    let n = count(&shape)?;
    if matches!(op, Op::Add | Op::Sub)
        && matches!((&a.data, &b.data), (Data::Float(_), Data::Float(_)))
    {
        let (Data::Float(x), Data::Float(y)) = (a.data, b.data) else {
            unreachable!()
        };
        let data = if matches!(op, Op::Sub) {
            crate::numeric::float::<true>(x, y, n, ad, bd)?
        } else {
            crate::numeric::float::<false>(x, y, n, ad, bd)?
        };
        return Value::new(shape, data);
    }
    if matches!((&a.data, &b.data), (Data::Int(_), Data::Int(_))) {
        match op {
            Op::Add => return int_pair::<0>(a, b, shape, ad, bd),
            Op::Sub => return int_pair::<1>(a, b, shape, ad, bd),
            Op::Mul => return int_pair::<2>(a, b, shape, ad, bd),
            _ => (),
        }
    }
    if matches!(op, Op::Eq | Op::Lt | Op::Gt) {
        let mut out = buffer(n)?;
        for i in 0..n {
            let (ai, bi) = (i / ad, i / bd);
            let (eq, lt, gt) = match (&a.data, &b.data) {
                (Data::Char(x), Data::Char(y)) => (x[ai] == y[bi], x[ai] < y[bi], x[ai] > y[bi]),
                (Data::Char(_), _) | (_, Data::Char(_)) => {
                    if matches!(op, Op::Eq) {
                        (false, false, false)
                    } else {
                        return Err(Error::Domain);
                    }
                }
                (Data::Float(_), _) | (_, Data::Float(_)) => {
                    let x = a.float_at(ai)?;
                    let y = b.float_at(bi)?;
                    let eq = near(x, y);
                    (eq, !eq && x < y, !eq && x > y)
                }
                _ => {
                    let x = a.int_at(ai)?;
                    let y = b.int_at(bi)?;
                    (x == y, x < y, x > y)
                }
            };
            out.push(match op {
                Op::Eq => eq,
                Op::Lt => lt,
                Op::Gt => gt,
                _ => unreachable!(),
            } as u8);
        }
        return Value::new(shape, Data::Bool(Arc::new(out)));
    }
    if matches!(a.data, Data::Char(_)) || matches!(b.data, Data::Char(_)) {
        return Err(Error::Domain);
    }
    let float = matches!(a.data, Data::Float(_))
        || matches!(b.data, Data::Float(_))
        || matches!(op, Op::Div);
    if !float {
        // Preflight avoids exposing partially mutated output on integer promotion.
        let overflow = (0..n)
            .any(|i| integer(op, a.int_at(i / ad).unwrap(), b.int_at(i / bd).unwrap()).is_none());
        if !overflow {
            if matches!(op, Op::Mul) && matches!((&a.data, &b.data), (Data::Bool(_), Data::Bool(_)))
            {
                let mut out = buffer(n)?;
                for i in 0..n {
                    out.push((a.int_at(i / ad)? * b.int_at(i / bd)?) as u8);
                }
                return Value::new(shape, Data::Bool(Arc::new(out)));
            }
            if ad == 1 && matches!(a.data, Data::Int(_)) {
                let Data::Int(storage) = a.data else {
                    unreachable!()
                };
                let mut out = match Arc::try_unwrap(storage) {
                    Ok(v) => v,
                    Err(v) => {
                        let mut out = buffer(n)?;
                        out.extend_from_slice(&v);
                        out
                    }
                };
                for (i, x) in out.iter_mut().enumerate() {
                    *x = integer(op, *x, b.int_at(i / bd)?).unwrap();
                }
                return Value::ints(shape, out);
            }
            let mut out = buffer(n)?;
            for i in 0..n {
                out.push(integer(op, a.int_at(i / ad)?, b.int_at(i / bd)?).unwrap());
            }
            return Value::ints(shape, out);
        }
    }
    if ad == 1 && matches!(a.data, Data::Float(_)) {
        let Data::Float(storage) = a.data else {
            unreachable!()
        };
        let mut out = match Arc::try_unwrap(storage) {
            Ok(v) => v,
            Err(v) => {
                let mut out = buffer(n)?;
                out.extend_from_slice(&v);
                out
            }
        };
        for (i, x) in out.iter_mut().enumerate() {
            *x = real(op, *x, b.float_at(i / bd)?);
        }
        Value::new(shape, Data::Float(Arc::new(out)))
    } else {
        let mut out = buffer(n)?;
        for i in 0..n {
            out.push(real(op, a.float_at(i / ad)?, b.float_at(i / bd)?));
        }
        Value::new(shape, Data::Float(Arc::new(out)))
    }
}

fn dimensions(v: &Value) -> Result<Vec<usize>> {
    if v.shape.len() > 1 {
        return Err(Error::Rank);
    }
    (0..v.len())
        .map(|i| usize::try_from(v.int_at(i)?).map_err(|_| Error::Domain))
        .collect()
}

pub fn monad(verb: &str, mut y: Value) -> Result<Value> {
    match verb {
        "+" => {
            if matches!(y.data, Data::Char(_)) {
                Err(Error::Domain)
            } else {
                Ok(y)
            }
        }
        "-" => atomic(Op::Sub, Value::scalar(0), y),
        "%" => atomic(Op::Div, Value::scalar(1), y),
        "*" | "|" => {
            if matches!(y.data, Data::Char(_)) {
                return Err(Error::Domain);
            }
            if verb == "|" {
                if matches!(y.data, Data::Bool(_)) {
                    return Ok(y);
                }
                if let Data::Int(v) = &y.data {
                    if v.iter().all(|&x| x != i64::MIN) {
                        let mut out = buffer(v.len())?;
                        out.extend(v.iter().map(|x| x.abs()));
                        return Value::ints(y.shape, out);
                    }
                }
                let mut out = buffer(y.len())?;
                for i in 0..y.len() {
                    out.push(y.float_at(i)?.abs());
                }
                Value::new(y.shape, Data::Float(Arc::new(out)))
            } else {
                let mut out = buffer(y.len())?;
                for i in 0..y.len() {
                    let x = y.float_at(i)?;
                    if x.is_nan() {
                        return Err(Error::Unsupported("signum of NaN".into()));
                    }
                    out.push(if x > 0.0 {
                        1
                    } else if x < 0.0 {
                        -1
                    } else {
                        0
                    });
                }
                Value::ints(y.shape, out)
            }
        }
        "$" => Value::ints(
            vec![y.shape.len()],
            y.shape.iter().map(|&d| d as i64).collect(),
        ),
        "#" => Ok(Value::scalar(y.shape.first().copied().unwrap_or(1) as i64)),
        "," => {
            y.shape = vec![y.len()];
            Ok(y)
        }
        "i." => {
            if y.shape.len() > 1 {
                return Err(Error::Rank);
            }
            let dims = (0..y.len())
                .map(|i| y.int_at(i))
                .collect::<Result<Vec<_>>>()?;
            let shape = dims
                .iter()
                .map(|x| {
                    x.checked_abs()
                        .and_then(|d| usize::try_from(d).ok())
                        .ok_or(Error::Limit)
                })
                .collect::<Result<Vec<_>>>()?;
            let n = count(&shape)?;
            if n > i64::MAX as usize {
                return Err(Error::Limit);
            }
            let mut out = buffer(n)?;
            for index in 0..n {
                let mut rem = index;
                let mut stride = 1;
                let mut mapped = 0;
                for (&d, &signed) in shape.iter().zip(&dims).rev() {
                    let pos = rem % d;
                    rem /= d;
                    mapped += (if signed < 0 { d - 1 - pos } else { pos }) * stride;
                    stride *= d;
                }
                out.push(mapped as i64);
            }
            Value::ints(shape, out)
        }
        _ => Err(Error::Unsupported(format!("monad {verb}"))),
    }
}

pub fn dyad(verb: &str, a: Value, mut b: Value) -> Result<Value> {
    let op = match verb {
        "+" => Some(Op::Add),
        "-" => Some(Op::Sub),
        "*" => Some(Op::Mul),
        "%" => Some(Op::Div),
        "=" => Some(Op::Eq),
        "<" => Some(Op::Lt),
        ">" => Some(Op::Gt),
        _ => None,
    };
    if let Some(op) = op {
        return atomic(op, a, b);
    }
    match verb {
        "$" => {
            let shape = dimensions(&a)?;
            let n = count(&shape)?;
            if n == b.len() {
                b.shape = shape;
                return Ok(b);
            }
            if b.is_empty() && n > 0 {
                return Err(Error::Length);
            }
            b.select(shape, (0..n).map(|i| i % b.len()))
        }
        "{" => {
            let items = b.shape.first().copied().unwrap_or(1);
            let cell_shape = if b.shape.is_empty() {
                &[][..]
            } else {
                &b.shape[1..]
            };
            let cell = count(cell_shape)?;
            let mut indices = buffer(a.len().checked_mul(cell).ok_or(Error::Limit)?)?;
            for i in 0..a.len() {
                let x = a.int_at(i)?;
                let x = if x < 0 {
                    (items as i64).checked_add(x).ok_or(Error::Index)?
                } else {
                    x
                };
                if x < 0 || x as usize >= items {
                    return Err(Error::Index);
                }
                indices.extend(x as usize * cell..(x as usize + 1) * cell);
            }
            let mut shape = a.shape.clone();
            shape.extend_from_slice(cell_shape);
            b.select(shape, indices)
        }
        "," => {
            if a.shape.len() > 1 || b.shape.len() > 1 {
                return Err(Error::Unsupported("catenate rank > 1".into()));
            }
            assemble(
                vec![a.len().checked_add(b.len()).ok_or(Error::Limit)?],
                vec![a, b],
            )
        }
        _ => Err(Error::Unsupported(format!("dyad {verb}"))),
    }
}

pub fn reduce(verb: &str, y: Value) -> Result<Value> {
    if !matches!(verb, "+" | "-" | "*" | "%") {
        return Err(Error::Unsupported(format!("reduction {verb}")));
    }
    if y.shape.is_empty() {
        return Ok(y);
    }
    let items = y.shape[0];
    let shape = y.shape[1..].to_vec();
    let cell = count(&shape)?;
    if items == 0 {
        if !matches!(verb, "+" | "*") {
            return Err(Error::Unsupported("empty reduction identity".into()));
        }
        let fill = if verb == "+" { 0 } else { 1 };
        let mut data = buffer(cell)?;
        data.resize(cell, fill);
        return Value::new(shape, Data::Bool(Arc::new(data)));
    }
    // Reference implementation: right fold. Specialized reductions come later.
    let mut out = y.select(shape.clone(), (items - 1) * cell..items * cell)?;
    for row in (0..items - 1).rev() {
        let lhs = y.select(shape.clone(), row * cell..(row + 1) * cell)?;
        out = dyad(verb, lhs, out)?;
    }
    Ok(out)
}

pub fn assemble(shape: Vec<usize>, cells: Vec<Value>) -> Result<Value> {
    let n = count(&shape)?;
    let chars = cells.iter().any(|x| matches!(x.data, Data::Char(_)));
    if chars {
        let mut out = buffer(n)?;
        for c in cells {
            if let Data::Char(v) = c.data {
                out.extend_from_slice(&v);
            } else {
                return Err(Error::Domain);
            }
        }
        return Value::new(shape, Data::Char(Arc::new(out)));
    }
    if cells.iter().any(|x| matches!(x.data, Data::Float(_))) {
        let mut out = buffer(n)?;
        for c in cells {
            for i in 0..c.len() {
                out.push(c.float_at(i)?);
            }
        }
        Value::new(shape, Data::Float(Arc::new(out)))
    } else if cells.iter().any(|x| matches!(x.data, Data::Int(_))) {
        let mut out = buffer(n)?;
        for c in cells {
            for i in 0..c.len() {
                out.push(c.int_at(i)?);
            }
        }
        Value::ints(shape, out)
    } else {
        let mut out = buffer(n)?;
        for c in cells {
            if let Data::Bool(v) = c.data {
                out.extend_from_slice(&v);
            }
        }
        Value::new(shape, Data::Bool(Arc::new(out)))
    }
}

pub fn ranked(verb: &str, reduction: bool, rank: i64, y: Value) -> Result<Value> {
    let yr = y.shape.len();
    let r = if rank < 0 {
        yr.saturating_sub(rank.unsigned_abs() as usize)
    } else {
        (rank as usize).min(yr)
    };
    let f = yr - r;
    let call = |v| {
        if reduction {
            reduce(verb, v)
        } else {
            monad(verb, v)
        }
    };
    if f == 0 {
        return call(y);
    }
    let frames = count(&y.shape[..f])?;
    if frames == 0 {
        return Err(Error::Unsupported(
            "rank over empty frame (prototype inference)".into(),
        ));
    }
    let cell_shape = y.shape[f..].to_vec();
    let size = count(&cell_shape)?;
    let mut cells = buffer(frames)?;
    for i in 0..frames {
        cells.push(call(
            y.select(cell_shape.clone(), i * size..(i + 1) * size)?,
        )?);
    }
    let result_shape = cells[0].shape.clone();
    if cells.iter().any(|v| v.shape != result_shape) {
        return Err(Error::Unsupported("rank result padding".into()));
    }
    let mut shape = y.shape[..f].to_vec();
    shape.extend(result_shape);
    assemble(shape, cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unique_storage_is_reused_and_shared_storage_is_not_mutated() {
        let x = Value::ints(vec![3], vec![2, 3, 4]).unwrap();
        let Data::Int(ref v) = x.data else { panic!() };
        let ptr = v.as_ptr();
        let result = atomic(Op::Add, x, Value::scalar(2)).unwrap();
        let Data::Int(ref v) = result.data else {
            panic!()
        };
        assert_eq!(ptr, v.as_ptr());
        let saved = result.clone();
        let changed = atomic(Op::Add, result, Value::scalar(10)).unwrap();
        assert_eq!(saved.display(), "4 5 6");
        assert_eq!(changed.display(), "14 15 16");
    }
}

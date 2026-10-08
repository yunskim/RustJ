//! Validated exact rational literal construction, independent of execution layout.
use crate::{
    Error, Result,
    types::{BigInt, Rational},
};

fn integer(source: &str) -> Result<BigInt> {
    source
        .strip_suffix('x')
        .unwrap_or(source)
        .replace('_', "-")
        .parse()
        .map_err(|_| Error::IllFormedNumber)
}
pub(crate) fn literal(source: &str) -> Result<Rational> {
    if matches!(source, "_" | "__") {
        return Rational::new(if source == "_" { 1 } else { -1 }.into(), 0.into());
    }
    if let Some((negative, rest)) = source
        .strip_prefix("__r")
        .map(|s| (true, s))
        .or_else(|| source.strip_prefix("_r").map(|s| (false, s)))
    {
        // numr flips the infinity sign from spelling, including _0.
        let negative = negative ^ rest.starts_with('_');
        return Rational::new(if negative { -1 } else { 1 }.into(), 0.into());
    }
    if let Some((numerator, denominator)) = source.split_once('r') {
        if matches!(denominator, "_" | "__") {
            return Rational::new(0.into(), 1.into());
        }
        return Rational::new(integer(numerator)?, integer(denominator)?);
    }
    Rational::new(integer(source)?, 1.into())
}
use crate::{
    Data, Value,
    kernels::Op,
    storage::{CpuStorage, Shape},
};
use num_rational::BigRational;
use std::{borrow::Cow, cmp::Ordering, sync::Arc};

fn sign(q: &Rational) -> Ordering {
    q.numerator().cmp(&BigInt::from(0))
}
fn finite(q: &Rational) -> BigRational {
    // Canonical inputs need no second gcd. Never pass infinity to num-rational.
    BigRational::new_raw(q.numerator().clone(), q.denominator().clone())
}
fn compare(a: &Rational, b: &Rational) -> Ordering {
    match (a.is_finite(), b.is_finite()) {
        (false, false) => sign(a).cmp(&sign(b)),
        (false, true) => sign(a),
        (true, false) => sign(b).reverse(),
        (true, true) => (a.numerator() * b.denominator()).cmp(&(b.numerator() * a.denominator())),
    }
}
fn calculate(op: Op, a: &Rational, b: &Rational) -> Result<Rational> {
    let (af, bf) = (a.is_finite(), b.is_finite());
    if !af || !bf {
        return match op {
            Op::Add | Op::Sub => {
                let bs = if matches!(op, Op::Sub) {
                    sign(b).reverse()
                } else {
                    sign(b)
                };
                if !af && !bf && sign(a) != bs {
                    return Err(Error::NaN);
                }
                let s = if !af { sign(a) } else { bs };
                Rational::new(if s == Ordering::Less { -1 } else { 1 }.into(), 0.into())
            }
            Op::Mul => Rational::new(a.numerator() * b.numerator(), 0.into()),
            Op::Div => {
                if !af && !bf {
                    return Err(Error::NaN);
                }
                if !bf {
                    return Rational::new(0.into(), 1.into());
                }
                // Pinned QdivQQ represents a non-finite numerator over zero.
                // The zero denominator loses the finite divisor sign in GMP;
                // fresh C engines retain the numerator infinity sign.
                let negative = sign(a) == Ordering::Less;
                Rational::new(if negative { -1 } else { 1 }.into(), 0.into())
            }
            _ => unreachable!(),
        };
    }
    if matches!(op, Op::Div) && sign(b) == Ordering::Equal {
        return Rational::new(a.numerator().clone(), 0.into());
    }
    let (a, b) = (finite(a), finite(b));
    let result = match op {
        Op::Add => a + b,
        Op::Sub => a - b,
        Op::Mul => a * b,
        Op::Div => a / b,
        _ => unreachable!(),
    };
    Ok(Rational::from_finite(result))
}
fn atom(value: &Value, index: usize) -> Result<Cow<'_, Rational>> {
    let integer = match value.data() {
        Data::Rational(v) => return Ok(Cow::Borrowed(v[index].as_ref())),
        Data::ExtendedInt(v) => v[index].as_ref().clone(),
        Data::Int(v) => BigInt::from(v[index]),
        Data::Bool(v) => BigInt::from(v[index]),
        _ => return Err(Error::Unsupported("rational mixed-type arithmetic".into())),
    };
    Ok(Cow::Owned(Rational::new(integer, 1.into())?))
}
pub(crate) fn atomic(
    op: Op,
    a: &Value,
    b: &Value,
    shape: Shape,
    ad: usize,
    bd: usize,
) -> Result<Value> {
    // Check capabilities even for empty arrays; never infer Float conversion.
    for v in [a, b] {
        if !matches!(
            v.data(),
            Data::Rational(_) | Data::ExtendedInt(_) | Data::Int(_) | Data::Bool(_)
        ) {
            return Err(Error::Unsupported("rational mixed-type arithmetic".into()));
        }
    }
    let n = crate::value::count(&shape)?;
    if matches!(op, Op::Eq | Op::Lt | Op::Gt) {
        let mut out = crate::value::buffer(n)?;
        for i in 0..n {
            let (a, b) = (atom(a, i / ad)?, atom(b, i / bd)?);
            let order = compare(&a, &b);
            out.push(match op {
                Op::Eq => order == Ordering::Equal,
                Op::Lt => order == Ordering::Less,
                Op::Gt => order == Ordering::Greater,
                _ => unreachable!(),
            } as u8);
        }
        return Value::new(shape, Data::Bool(CpuStorage::new(out)));
    }
    let mut out = crate::value::buffer(n)?;
    for i in 0..n {
        let (x, y) = (atom(a, i / ad)?, atom(b, i / bd)?);
        out.push(Arc::new(calculate(op, &x, &y)?));
    }
    Value::new(shape, Data::Rational(CpuStorage::new(out)))
}
pub(crate) fn unary(verb: &str, y: Value) -> Result<Value> {
    let Data::Rational(v) = y.data() else {
        unreachable!()
    };
    if verb == "*" {
        let mut out = crate::value::buffer(v.len())?;
        out.extend(v.iter().map(|q| {
            Arc::new(BigInt::from(match sign(q) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }))
        }));
        return Value::new(
            Shape::from(y.shape()),
            Data::ExtendedInt(CpuStorage::new(out)),
        );
    }
    let mut out = crate::value::buffer(v.len())?;
    for q in v.iter() {
        out.push(match verb {
            "|" if sign(q) != Ordering::Less => Arc::clone(q),
            "-" | "|" => Arc::new(Rational::new(-q.numerator(), q.denominator().clone())?),
            "%" => Arc::new(Rational::new(
                q.denominator().clone(),
                q.numerator().clone(),
            )?),
            _ => unreachable!(),
        });
    }
    Value::new(Shape::from(y.shape()), Data::Rational(CpuStorage::new(out)))
}

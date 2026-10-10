//! Finite arbitrary-precision CPU semantics; no Float conversion or device layout.
use crate::{
    Data, Error, Result, Value,
    storage::{CpuStorage, Shape},
    types::BigInt,
};
use num_integer::Integer;
use std::{borrow::Cow, sync::Arc};

fn atom(value: &Value, index: usize) -> Result<Cow<'_, BigInt>> {
    match value.data() {
        Data::ExtendedInt(v) => Ok(Cow::Borrowed(v[index].as_ref())),
        Data::Int(v) => Ok(Cow::Owned(BigInt::from(v[index]))),
        Data::Bool(v) => Ok(Cow::Owned(BigInt::from(v[index]))),
        _ => Err(Error::Unsupported("extended mixed-type arithmetic".into())),
    }
}

pub(crate) fn counts(
    shape: impl Into<Shape>,
    counts: impl IntoIterator<Item = usize>,
) -> Result<Value> {
    let shape = shape.into();
    let mut out = crate::value::buffer(crate::value::count(&shape)?)?;
    out.extend(counts.into_iter().map(|n| Arc::new(BigInt::from(n))));
    Value::new(shape, Data::ExtendedInt(CpuStorage::new(out)))
}

pub(crate) fn atomic(
    op: crate::kernels::Op,
    a: &Value,
    b: &Value,
    shape: Shape,
    ad: usize,
    bd: usize,
) -> Result<Value> {
    use crate::kernels::Op;
    if !matches!(
        a.data(),
        Data::ExtendedInt(_) | Data::Int(_) | Data::Bool(_)
    ) || !matches!(
        b.data(),
        Data::ExtendedInt(_) | Data::Int(_) | Data::Bool(_)
    ) {
        return Err(Error::Unsupported("extended mixed-type arithmetic".into()));
    }
    if matches!(op, Op::Div) {
        return divide(a, b, shape, ad, bd);
    }
    let n = crate::value::count(&shape)?;
    if matches!(op, Op::Eq | Op::Lt | Op::Gt) {
        let mut out = crate::value::buffer(n)?;
        for i in 0..n {
            let x = atom(a, i / ad)?;
            let y = atom(b, i / bd)?;
            out.push(match op {
                Op::Eq => x == y,
                Op::Lt => x < y,
                Op::Gt => x > y,
                _ => unreachable!(),
            } as u8);
        }
        return Value::new(shape, Data::Bool(CpuStorage::new(out)));
    }
    let mut out = crate::value::buffer(n)?;
    for i in 0..n {
        let x = atom(a, i / ad)?;
        let y = atom(b, i / bd)?;
        let result = match op {
            Op::Add => x.as_ref() + y.as_ref(),
            Op::Sub => x.as_ref() - y.as_ref(),
            Op::Mul => x.as_ref() * y.as_ref(),
            _ => unreachable!(),
        };
        out.push(Arc::new(result));
    }
    Value::new(shape, Data::ExtendedInt(CpuStorage::new(out)))
}

pub(crate) fn unary(verb: &str, y: Value) -> Result<Value> {
    let Data::ExtendedInt(v) = y.data() else {
        unreachable!()
    };
    let mut out = crate::value::buffer(v.len())?;
    let zero = BigInt::from(0);
    for x in v.iter() {
        let result = match verb {
            "-" => Arc::new(-x.as_ref()),
            "|" => {
                if x.as_ref() < &zero {
                    Arc::new(-x.as_ref())
                } else {
                    Arc::clone(x)
                }
            }
            "*" => Arc::new(BigInt::from(if x.as_ref() > &zero {
                1
            } else if x.as_ref() < &zero {
                -1
            } else {
                0
            })),
            _ => unreachable!(),
        };
        out.push(result);
    }
    Value::new(
        Shape::from(y.shape()),
        Data::ExtendedInt(CpuStorage::new(out)),
    )
}

// Preserve the C whole-result promotion policy without replaying earlier atoms.
fn divide(a: &Value, b: &Value, shape: Shape, ad: usize, bd: usize) -> Result<Value> {
    let n = crate::value::count(&shape)?;
    let zero = BigInt::from(0);
    let mut integers = crate::value::buffer(n)?;
    for i in 0..n {
        let (x, y) = (atom(a, i / ad)?, atom(b, i / bd)?);
        if x.as_ref() == &zero {
            integers.push(Arc::new(BigInt::from(0)));
            continue;
        }
        if y.as_ref() != &zero {
            let (quotient, remainder) = x.div_rem(&y);
            if remainder == zero {
                integers.push(Arc::new(quotient));
                continue;
            }
        }
        let mut rationals = crate::value::buffer(n)?;
        for quotient in integers {
            // These freshly constructed quotients have no external aliases.
            let quotient = Arc::try_unwrap(quotient).expect("private division quotient");
            rationals.push(Arc::new(crate::types::Rational::new(quotient, 1.into())?));
        }
        rationals.push(Arc::new(crate::types::Rational::new(
            x.into_owned(),
            y.into_owned(),
        )?));
        for j in i + 1..n {
            rationals.push(Arc::new(crate::types::Rational::new(
                atom(a, j / ad)?.into_owned(),
                atom(b, j / bd)?.into_owned(),
            )?));
        }
        return Value::new(shape, Data::Rational(CpuStorage::new(rationals)));
    }
    Value::new(shape, Data::ExtendedInt(CpuStorage::new(integers)))
}

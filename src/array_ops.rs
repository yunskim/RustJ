//! Dense array rearrangement. One output allocation, no index-array temporary.
use crate::{
    Data, Error, Result, Value,
    storage::{CpuStorage, Shape},
    value::count,
};

fn mapped(y: &Value, shape: Shape, index: impl Fn(usize) -> Option<usize>) -> Result<Value> {
    let n = count(&shape)?;
    let data = match &y.data {
        Data::Sparse(_) => return Err(Error::Unsupported("sparse rearrangement".into())),
        Data::Rational(v) => {
            let mut out = crate::value::buffer(n)?;
            let mut zero = None;
            for i in 0..n {
                out.push(match index(i) {
                    Some(j) => v[j].clone(),
                    None => zero
                        .get_or_insert_with(|| {
                            std::sync::Arc::new(
                                crate::types::Rational::new(0.into(), 1.into())
                                    .expect("canonical zero"),
                            )
                        })
                        .clone(),
                });
            }
            Data::Rational(CpuStorage::new(out))
        }
        Data::ExtendedInt(v) => {
            let mut out = crate::value::buffer(n)?;
            let mut zero = None;
            for i in 0..n {
                out.push(match index(i) {
                    Some(j) => v[j].clone(),
                    None => zero
                        .get_or_insert_with(|| std::sync::Arc::new(crate::types::BigInt::from(0)))
                        .clone(),
                });
            }
            Data::ExtendedInt(CpuStorage::new(out))
        }
        Data::Boxed(v) => {
            let mut out = crate::value::buffer(n)?;
            for i in 0..n {
                let j = index(i).ok_or_else(|| Error::Unsupported("boxed fill".into()))?;
                out.push(v[j].clone());
            }
            Data::Boxed(CpuStorage::new(out))
        }
        Data::Bool(v) => Data::Bool(CpuStorage::generate(n, |i| index(i).map_or(0, |j| v[j]))?),
        Data::Int(v) => Data::Int(CpuStorage::generate(n, |i| index(i).map_or(0, |j| v[j]))?),
        Data::Float(v) => Data::Float(CpuStorage::generate(n, |i| index(i).map_or(0.0, |j| v[j]))?),
        Data::Char(v) => Data::Char(CpuStorage::generate(n, |i| {
            index(i).map_or(b' ', |j| v[j])
        })?),
    };
    Value::new(shape, data)
}

pub(crate) fn reverse(y: Value) -> Result<Value> {
    if y.shape.is_empty() || y.is_empty() {
        return Ok(y);
    }
    let items = y.shape[0];
    let cell = y.len() / items;
    mapped(&y, y.shape.clone(), |i| {
        Some((items - 1 - i / cell) * cell + i % cell)
    })
}

pub(crate) fn transpose(y: Value) -> Result<Value> {
    if y.shape.len() < 2 || y.is_empty() {
        let mut shape = y.shape.to_vec();
        shape.reverse();
        return Value::new(shape, y.data);
    }
    let mut shape = y.shape.to_vec();
    shape.reverse();
    mapped(&y, shape.into(), |mut i| {
        let mut source = 0;
        // Output axes reverse the input axes; decode from the output's last axis.
        for &d in y.shape.iter() {
            source = source * d + i % d;
            i /= d;
        }
        Some(source)
    })
}

pub(crate) fn scalar_dyad(verb: &str, a: Value, y: Value) -> Result<Value> {
    if !a.shape.is_empty() {
        return Err(Error::Unsupported(format!("{verb} with axis-count list")));
    }
    let signed = a.int_at(0)?;
    let magnitude = usize::try_from(signed.unsigned_abs()).map_err(|_| Error::Limit)?;
    let items = y.shape.first().copied().unwrap_or(1);
    let cell = count(if y.shape.is_empty() {
        &[]
    } else {
        &y.shape[1..]
    })?;
    if verb == "|." {
        if y.shape.is_empty() || y.is_empty() {
            return Ok(y);
        }
        let k = magnitude % items;
        let shift = if signed < 0 && k != 0 { items - k } else { k };
        if shift == 0 {
            return Ok(y);
        }
        return mapped(&y, y.shape.clone(), |i| {
            let row = i / cell;
            let rotated = if row >= items - shift {
                row - (items - shift)
            } else {
                row + shift
            };
            Some(rotated * cell + i % cell)
        });
    }
    let out_items = if verb == "{." {
        magnitude
    } else {
        items.saturating_sub(magnitude)
    };
    if out_items > i64::MAX as usize {
        return Err(Error::Limit);
    }
    let mut shape = y.shape.to_vec();
    if shape.is_empty() {
        shape.push(out_items);
    } else {
        shape[0] = out_items;
    }
    let removed = magnitude.min(items);
    mapped(&y, shape.into(), |i| {
        let row = i / cell;
        let source = if verb == "}." {
            Some(if signed < 0 { row } else { row + removed })
        } else if signed >= 0 {
            (row < items).then_some(row)
        } else if magnitude <= items {
            Some(items - magnitude + row)
        } else {
            row.checked_sub(magnitude - items)
        };
        source.map(|row| row * cell + i % cell)
    })
}

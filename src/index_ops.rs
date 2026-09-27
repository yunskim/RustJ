//! Index generation and item search. Floating search preserves tolerant equality.
use crate::{
    Data, Error, Result, Value,
    storage::{CpuStorage, Shape},
    value::{buffer, count},
};
use std::collections::HashMap;

pub(crate) fn steps(y: Value) -> Result<Value> {
    if !y.shape.is_empty() {
        return Err(Error::Unsupported("i: over array (result padding)".into()));
    }
    let d = y.float_at(0)?;
    let steps = 2.0 * d.abs();
    if !steps.is_finite() || steps.fract() != 0.0 {
        return Err(Error::Domain);
    }
    if steps >= i64::MAX as f64 {
        return Err(Error::Limit);
    }
    let n = (steps as usize).checked_add(1).ok_or(Error::Limit)?;
    let data = if d.fract() == 0.0 {
        let start = -(d as i64);
        Data::Int(CpuStorage::generate(n, |i| {
            if d < 0.0 {
                start - i as i64
            } else {
                start + i as i64
            }
        })?)
    } else {
        Data::Float(CpuStorage::generate(n, |i| {
            if d < 0.0 {
                -d - i as f64
            } else {
                -d + i as f64
            }
        })?)
    };
    Value::new([n], data)
}

pub(crate) fn indices(y: Value) -> Result<Value> {
    if y.shape.len() > 1 {
        return Err(Error::Unsupported(
            "I. over higher rank (result padding)".into(),
        ));
    }
    let mut n = 0usize;
    for i in 0..y.len() {
        let k = usize::try_from(y.int_at(i)?).map_err(|_| Error::Domain)?;
        n = n.checked_add(k).ok_or(Error::Limit)?;
    }
    let mut out = buffer(n)?;
    for i in 0..y.len() {
        out.extend(std::iter::repeat_n(i as i64, y.int_at(i)? as usize));
    }
    Value::ints([n], out)
}

fn atom_eq(a: &Value, ai: usize, b: &Value, bi: usize) -> bool {
    match (&a.data, &b.data) {
        (Data::Char(x), Data::Char(y)) => x[ai] == y[bi],
        (Data::Char(_), _) | (_, Data::Char(_)) => false,
        (Data::Float(_), _) | (_, Data::Float(_)) => {
            crate::kernels::near(a.float_at(ai).unwrap(), b.float_at(bi).unwrap())
        }
        _ => a.int_at(ai).unwrap() == b.int_at(bi).unwrap(),
    }
}

pub(crate) fn index_of(a: Value, b: Value, last: bool) -> Result<Value> {
    let items = a.shape.first().copied().unwrap_or(1);
    let cell_shape = if a.shape.is_empty() {
        &[][..]
    } else {
        &a.shape[1..]
    };
    let frame = b.shape.len().saturating_sub(cell_shape.len());
    let shape = Shape::from(&b.shape[..frame]);
    let n = count(&shape)?;
    if b.shape.len() < cell_shape.len() || &b.shape[frame..] != cell_shape {
        return Value::new(shape, Data::Int(CpuStorage::generate(n, |_| items as i64)?));
    }
    let cell = count(cell_shape)?;
    // Exact integer/bool scalar items admit hashing without float tolerance issues.
    if cell_shape.is_empty()
        && matches!(a.data, Data::Int(_) | Data::Bool(_))
        && matches!(b.data, Data::Int(_) | Data::Bool(_))
    {
        let mut table = HashMap::new();
        table.try_reserve(items).map_err(|_| Error::Limit)?;
        for i in 0..items {
            let key = a.int_at(i)?;
            if last {
                table.insert(key, i);
            } else {
                table.entry(key).or_insert(i);
            }
        }
        return Value::new(
            shape,
            Data::Int(CpuStorage::generate(n, |i| {
                *table.get(&b.int_at(i).unwrap()).unwrap_or(&items) as i64
            })?),
        );
    }
    Value::new(
        shape,
        Data::Int(CpuStorage::generate(n, |q| {
            let matches = |i: usize| (0..cell).all(|k| atom_eq(&a, i * cell + k, &b, q * cell + k));
            if last {
                (0..items).rev().find(|&i| matches(i))
            } else {
                (0..items).find(|&i| matches(i))
            }
            .unwrap_or(items) as i64
        })?),
    )
}

pub(crate) fn member(a: Value, b: Value) -> Result<Value> {
    let items = b.shape.first().copied().unwrap_or(1) as i64;
    let positions = index_of(b, a, false)?;
    Value::new(
        positions.shape.clone(),
        Data::Bool(CpuStorage::generate(positions.len(), |i| {
            (positions.int_at(i).unwrap() < items) as u8
        })?),
    )
}

pub(crate) fn find(a: Value, b: Value) -> Result<Value> {
    if a.shape.len() > b.shape.len() {
        return Err(Error::Rank);
    }
    if a.shape.len() > 1 || b.shape.len() > 1 {
        return Err(Error::Unsupported("E. multidimensional pattern".into()));
    }
    let width = a.len();
    Value::new(
        b.shape.clone(),
        Data::Bool(CpuStorage::generate(b.len(), |i| {
            (width <= b.len() - i && (0..width).all(|k| atom_eq(&a, k, &b, i + k))) as u8
        })?),
    )
}

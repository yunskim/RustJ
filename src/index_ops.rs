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

pub(crate) fn atom_eq(a: &Value, ai: usize, b: &Value, bi: usize) -> bool {
    match (&a.data, &b.data) {
        (Data::Char(x), Data::Char(y)) => x[ai] == y[bi],
        (Data::Char(_), _) | (_, Data::Char(_)) => false,
        (Data::Float(_), _) | (_, Data::Float(_)) => {
            crate::kernels::near(a.float_at(ai).unwrap(), b.float_at(bi).unwrap())
        }
        _ => a.int_at(ai).unwrap() == b.int_at(bi).unwrap(),
    }
}


/// J Index-Of output intent; the search index is an implementation detail,
/// not a new J value or a new Graph IR operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LookupResult {
    First,
    Last,
    Membership,
}

/// A bounded direct-address table is profitable for narrow integer domains.
/// Other exact scalar domains still use a conventional hash table. Neither
/// route is legal for tolerant float/boxed comparisons.
enum ExactScalarIndex {
    Direct { minimum: i64, positions: Vec<usize> },
    Hashed(HashMap<i64, usize>),
}

impl ExactScalarIndex {
    fn find(&self, key: i64, missing: usize) -> usize {
        match self {
            Self::Direct { minimum, positions } => key
                .checked_sub(*minimum)
                .and_then(|delta| usize::try_from(delta).ok())
                .and_then(|offset| positions.get(offset))
                .copied()
                .unwrap_or(missing),
            Self::Hashed(entries) => entries.get(&key).copied().unwrap_or(missing),
        }
    }
}

/// Build a direct index only when table bytes are bounded and reasonably
/// proportional to the work. Use i128 when measuring the key span: subtracting
/// arbitrary i64 endpoints can overflow. An empty or tiny search needs no
/// indexing setup at all.
fn exact_scalar_index(
    values: &Value,
    items: usize,
    queries: usize,
    result: LookupResult,
) -> Result<Option<ExactScalarIndex>> {
    if items == 0 || queries == 0 || items.saturating_mul(queries) <= 32 {
        return Ok(None);
    }

    let mut minimum = i64::MAX;
    let mut maximum = i64::MIN;
    for i in 0..items {
        let key = values.int_at(i)?;
        minimum = minimum.min(key);
        maximum = maximum.max(key);
    }
    let span = i128::from(maximum) - i128::from(minimum) + 1;
    const MAX_DIRECT_ENTRIES: i128 = 65_536;
    let work_budget = items.saturating_add(queries).saturating_mul(4) as i128;
    let last = result == LookupResult::Last;

    if span <= MAX_DIRECT_ENTRIES && span <= work_budget {
        let width = span as usize;
        let mut positions = Vec::new();
        positions.try_reserve_exact(width).map_err(|_| Error::Limit)?;
        positions.resize(width, items);
        for i in 0..items {
            let offset = (i128::from(values.int_at(i)?) - i128::from(minimum)) as usize;
            if last || positions[offset] == items {
                positions[offset] = i;
            }
        }
        return Ok(Some(ExactScalarIndex::Direct { minimum, positions }));
    }

    let mut entries = HashMap::new();
    entries.try_reserve(items).map_err(|_| Error::Limit)?;
    for i in 0..items {
        let key = values.int_at(i)?;
        if last {
            entries.insert(key, i);
        } else {
            entries.entry(key).or_insert(i);
        }
    }
    Ok(Some(ExactScalarIndex::Hashed(entries)))
}

fn result_from_positions(
    shape: Shape,
    queries: usize,
    items: usize,
    result: LookupResult,
    mut position: impl FnMut(usize) -> usize,
) -> Result<Value> {
    match result {
        LookupResult::Membership => Value::new(
            shape,
            Data::Bool(CpuStorage::generate(queries, |q| (position(q) != items) as u8)?),
        ),
        LookupResult::First | LookupResult::Last => Value::new(
            shape,
            Data::Int(CpuStorage::generate(queries, |q| position(q) as i64)?),
        ),
    }
}

/// Common search contract: the first argument is the index domain, the second
/// is the query collection. Output mode controls materialization: membership
/// returns booleans directly, never a temporary index vector.
fn lookup(indexed: Value, queries: Value, result: LookupResult) -> Result<Value> {
    let items = indexed.shape.first().copied().unwrap_or(1);
    let cell_shape = if indexed.shape.is_empty() {
        &[][..]
    } else {
        &indexed.shape[1..]
    };
    let frame = queries.shape.len().saturating_sub(cell_shape.len());
    let shape = Shape::from(&queries.shape[..frame]);
    let n = count(&shape)?;

    if queries.shape.len() < cell_shape.len() || &queries.shape[frame..] != cell_shape {
        return result_from_positions(shape, n, items, result, |_| items);
    }

    let cell = count(cell_shape)?;
    let exact_scalar = cell_shape.is_empty()
        && matches!(indexed.data, Data::Int(_) | Data::Bool(_))
        && matches!(queries.data, Data::Int(_) | Data::Bool(_));
    let exact = if exact_scalar {
        exact_scalar_index(&indexed, items, n, result)?
    } else {
        None
    };
    result_from_positions(shape, n, items, result, |q| {
        if let Some(index) = &exact {
            return index.find(queries.int_at(q).unwrap(), items);
        }
        let equal = |i: usize| {
            (0..cell).all(|k| atom_eq(&indexed, i * cell + k, &queries, q * cell + k))
        };
        match result {
            LookupResult::Last => (0..items).rev().find(|&i| equal(i)),
            LookupResult::First | LookupResult::Membership => (0..items).find(|&i| equal(i)),
        }
        .unwrap_or(items)
    })
}

pub(crate) fn index_of(a: Value, b: Value, last: bool) -> Result<Value> {
    lookup(a, b, if last { LookupResult::Last } else { LookupResult::First })
}

pub(crate) fn member(a: Value, b: Value) -> Result<Value> {
    lookup(b, a, LookupResult::Membership)
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

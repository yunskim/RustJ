use crate::storage::{ArrayView, CpuStorage, CpuView, Shape};
use crate::{
    error::{Error, Result},
    value::{Data, Value, buffer, count},
};

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

fn agreement(a: &Value, b: &Value) -> Result<(Shape, usize, usize)> {
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
pub(crate) fn near(a: f64, b: f64) -> bool {
    // One semantic comparator identity for equality and all search modes.
    // Pinned J default CCT; dynamic 9!:19 and Fit remain unsupported.
    crate::comparison_policy::ComparisonPolicySnapshot::pinned_j_default_cct().float_equal(a, b)
}

fn int_pair<const OP: u8>(
    a: Value,
    b: Value,
    shape: Shape,
    ad: usize,
    bd: usize,
    pool: &mut crate::pool::OutputPool,
) -> Result<Value> {
    let (Data::Int(x), Data::Int(y)) = (a.data, b.data) else {
        unreachable!()
    };
    let data = crate::numeric::int::<OP>(x, y, count(&shape)?, ad, bd, pool)?;
    Value::new(shape, data)
}
pub fn atomic(op: Op, a: Value, b: Value) -> Result<Value> {
    atomic_with_pool(op, a, b, &mut crate::pool::OutputPool::default())
}
pub(crate) fn atomic_with_pool(
    op: Op,
    a: Value,
    b: Value,
    pool: &mut crate::pool::OutputPool,
) -> Result<Value> {
    if a.is_sparse() || b.is_sparse() {
        return Err(Error::Unsupported("sparse atomic operation".into()));
    }
    if matches!(a.data, Data::Boxed(_)) || matches!(b.data, Data::Boxed(_)) {
        return Err(Error::Unsupported("boxed atomic operation".into()));
    }
    let (shape, ad, bd) = agreement(&a, &b)?;
    let n = count(&shape)?;
    if n == 1 && matches!(op, Op::Add | Op::Sub | Op::Mul | Op::Div) {
        return arithmetic_views(op, a.view(), b.view());
    }
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
            Op::Add => return int_pair::<0>(a, b, shape, ad, bd, pool),
            Op::Sub => return int_pair::<1>(a, b, shape, ad, bd, pool),
            Op::Mul => return int_pair::<2>(a, b, shape, ad, bd, pool),
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
        return Value::new(shape, Data::Bool(CpuStorage::new(out)));
    }
    if matches!(a.data, Data::Char(_) | Data::Boxed(_))
        || matches!(b.data, Data::Char(_) | Data::Boxed(_))
    {
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
                return Value::new(shape, Data::Bool(CpuStorage::new(out)));
            }
            if ad == 1 && matches!(a.data, Data::Int(_)) {
                let Data::Int(storage) = a.data else {
                    unreachable!()
                };
                let mut out = match storage.try_into_vec() {
                    Ok(v) => v,
                    Err(v) => {
                        return Value::new(
                            shape,
                            Data::Int(CpuStorage::generate(n, |i| {
                                integer(op, v[i], b.int_at(i / bd).unwrap()).unwrap()
                            })?),
                        );
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
        let mut out = match storage.try_into_vec() {
            Ok(v) => v,
            Err(v) => {
                return Value::new(
                    shape,
                    Data::Float(CpuStorage::generate(n, |i| {
                        real(op, v[i], b.float_at(i / bd).unwrap())
                    })?),
                );
            }
        };
        for (i, x) in out.iter_mut().enumerate() {
            *x = real(op, *x, b.float_at(i / bd)?);
        }
        Value::new(shape, Data::Float(CpuStorage::new(out)))
    } else {
        let mut out = buffer(n)?;
        for i in 0..n {
            out.push(real(op, a.float_at(i / ad)?, b.float_at(i / bd)?));
        }
        Value::new(shape, Data::Float(CpuStorage::new(out)))
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
    if verb == "[:" {
        return Err(Error::Valence);
    }
    if verb == "$." {
        return crate::sparse::monad(y);
    }
    if y.is_sparse() && !matches!(verb, "$" | "#") {
        return Err(Error::Unsupported(format!("sparse monad {verb}")));
    }
    match verb {
        "<" => Ok(Value::boxed(y)),
        ">" => {
            if let Data::Boxed(v) = &y.data {
                if y.shape.is_empty() {
                    return Ok((*v[0]).clone());
                }
                let Some(first) = v.first() else {
                    // C jtope returns an empty boxed noun unchanged.
                    return Ok(y);
                };
                if v.iter().any(|cell| cell.shape() != first.shape()) {
                    return Err(Error::Unsupported("open with cell padding".into()));
                }
                let mut shape = y.shape.to_vec();
                shape.extend_from_slice(first.shape());
                let n = count(&shape)?;
                let mut builder = crate::assembly::CellBuilder::new(first, n)?;
                for cell in &v[1..] {
                    builder.push(cell)?;
                }
                return Value::new(shape, builder.finish());
            }
            Ok(y)
        }
        "i:" => crate::index_ops::steps(y),
        "I." => crate::index_ops::indices(y),
        "|." => crate::array_ops::reverse(y),
        "|:" => crate::array_ops::transpose(y),
        "+" => {
            if matches!(y.data, Data::Char(_) | Data::Boxed(_)) {
                Err(Error::Domain)
            } else {
                Ok(y)
            }
        }
        "-" => atomic(Op::Sub, Value::scalar(0), y),
        "%" => atomic(Op::Div, Value::scalar(1), y),
        "*" | "|" => {
            if matches!(y.data, Data::Char(_) | Data::Boxed(_)) {
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
                Value::new(y.shape, Data::Float(CpuStorage::new(out)))
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
            y.shape = Shape::from([y.len()]);
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
    if verb == "[:" {
        return Err(Error::Valence);
    }
    if verb == "$." {
        return crate::sparse::dyad(a, b);
    }
    if a.is_sparse() || b.is_sparse() {
        return Err(Error::Unsupported(format!("sparse dyad {verb}")));
    }
    if matches!(verb, "i." | "i:" | "e." | "E.")
        && (matches!(a.data, Data::Boxed(_)) || matches!(b.data, Data::Boxed(_)))
    {
        return Err(Error::Unsupported("boxed search".into()));
    }
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
        "e." => crate::index_ops::member(a, b),
        "E." => crate::index_ops::find(a, b),
        "i." | "i:" => crate::index_ops::index_of(a, b, verb == "i:"),
        "|." | "{." | "}." => crate::array_ops::scalar_dyad(verb, a, b),
        "$" => {
            let shape = dimensions(&a)?;
            let n = count(&shape)?;
            if n == b.len() {
                b.shape = shape.into();
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
    if y.is_sparse() {
        return Err(Error::Unsupported("sparse reduction".into()));
    }
    if !matches!(verb, "+" | "-" | "*" | "%") {
        return Err(Error::Unsupported(format!("reduction {verb}")));
    }
    if y.shape.is_empty() {
        return Ok(y);
    }
    reduce_view(verb, y.view())
}

fn reduce_view(verb: &str, y: ArrayView<'_>) -> Result<Value> {
    if matches!(y.data, CpuView::Sparse(_)) {
        return Err(Error::Unsupported("sparse reduction".into()));
    }
    if !matches!(verb, "+" | "-" | "*" | "%") {
        return Err(Error::Unsupported(format!("reduction {verb}")));
    }
    if y.shape.is_empty() {
        return y.to_owned();
    }
    let items = y.shape[0];
    let shape = Shape::from(&y.shape[1..]);
    let cell = count(&shape)?;
    if items == 0 {
        // ai.c iden: subtraction shares additive zero; division shares
        // multiplicative one. This is a right-fold identity, not reassociation.
        let fill = if matches!(verb, "+" | "-") { 0 } else { 1 };
        let mut data = buffer(cell)?;
        data.resize(cell, fill);
        return Value::new(shape, Data::Bool(CpuStorage::new(data)));
    }
    // Reference implementation: right fold. Specialized reductions come later.
    let mut out = y.cell(shape.len(), items - 1)?.to_owned()?;
    for row in (0..items - 1).rev() {
        let lhs = y.cell(shape.len(), row)?;
        let op = match verb {
            "+" => Op::Add,
            "-" => Op::Sub,
            "*" => Op::Mul,
            _ => Op::Div,
        };
        out = reduction_step(op, lhs, out)?;
    }
    Ok(out)
}

fn reduction_step(op: Op, lhs: ArrayView<'_>, rhs: Value) -> Result<Value> {
    if matches!(op, Op::Add | Op::Sub)
        && matches!((lhs.data, &rhs.data), (CpuView::Int(_), Data::Int(_)))
    {
        let CpuView::Int(left) = lhs.data else {
            unreachable!()
        };
        let Data::Int(right) = rhs.data else {
            unreachable!()
        };
        let data = if matches!(op, Op::Add) {
            crate::numeric::int_accumulate::<0>(left, right)?
        } else {
            crate::numeric::int_accumulate::<1>(left, right)?
        };
        return Value::new(rhs.shape, data);
    }
    arithmetic_views(op, lhs, rhs.view())
}

// Right-fold and rank read their inputs through lifetime-bound views. Output
// storage is owned, so no borrowed cell can escape into the evaluator.
fn arithmetic_views(op: Op, a: ArrayView<'_>, b: ArrayView<'_>) -> Result<Value> {
    if matches!(
        a.data,
        CpuView::Char(_) | CpuView::Boxed(_) | CpuView::Sparse(_)
    ) || matches!(
        b.data,
        CpuView::Char(_) | CpuView::Boxed(_) | CpuView::Sparse(_)
    ) {
        return Err(Error::Domain);
    }
    let (short, shape) = if a.shape.len() <= b.shape.len() {
        (a.shape, b.shape)
    } else {
        (b.shape, a.shape)
    };
    if !shape.starts_with(short) {
        return Err(Error::Length);
    }
    let n = count(shape)?;
    let ad = count(&shape[a.shape.len()..])?;
    let bd = count(&shape[b.shape.len()..])?;
    let float = matches!(a.data, CpuView::Float(_))
        || matches!(b.data, CpuView::Float(_))
        || matches!(op, Op::Div);
    if !float {
        if matches!(op, Op::Mul) && matches!((a.data, b.data), (CpuView::Bool(_), CpuView::Bool(_)))
        {
            return Value::new(
                Shape::from(shape),
                Data::Bool(CpuStorage::generate(n, |i| {
                    (a.int_at(i / ad).unwrap() * b.int_at(i / bd).unwrap()) as u8
                })?),
            );
        }
        let mut overflow = false;
        let out = CpuStorage::generate(n, |i| {
            match integer(op, a.int_at(i / ad).unwrap(), b.int_at(i / bd).unwrap()) {
                Some(x) => x,
                None => {
                    overflow = true;
                    0
                }
            }
        })?;
        if !overflow {
            return Value::new(Shape::from(shape), Data::Int(out));
        }
    }
    Value::new(
        Shape::from(shape),
        Data::Float(CpuStorage::generate(n, |i| {
            real(op, a.float_at(i / ad).unwrap(), b.float_at(i / bd).unwrap())
        })?),
    )
}

fn monad_view(verb: &str, y: ArrayView<'_>) -> Result<Value> {
    match verb {
        "#" => Ok(Value::scalar(y.shape.first().copied().unwrap_or(1) as i64)),
        "$" => Value::new(
            [y.shape.len()],
            Data::Int(CpuStorage::generate(y.shape.len(), |i| y.shape[i] as i64)?),
        ),
        "-" | "%" => {
            let x = Value::scalar(if verb == "-" { 0 } else { 1 });
            arithmetic_views(if verb == "-" { Op::Sub } else { Op::Div }, x.view(), y)
        }
        "*" | "|" => {
            if matches!(
                y.data,
                CpuView::Char(_) | CpuView::Boxed(_) | CpuView::Sparse(_)
            ) {
                return Err(Error::Domain);
            }
            let n = y.len();
            if verb == "*" {
                if (0..n).any(|i| y.float_at(i).unwrap().is_nan()) {
                    return Err(Error::Unsupported("signum of NaN".into()));
                }
                Value::new(
                    Shape::from(y.shape),
                    Data::Int(CpuStorage::generate(n, |i| {
                        let x = y.float_at(i).unwrap();
                        if x > 0.0 {
                            1
                        } else if x < 0.0 {
                            -1
                        } else {
                            0
                        }
                    })?),
                )
            } else if matches!(y.data, CpuView::Bool(_)) {
                y.to_owned()
            } else if matches!(y.data,CpuView::Int(v) if v.iter().all(|&x|x != i64::MIN)) {
                Value::new(
                    Shape::from(y.shape),
                    Data::Int(CpuStorage::generate(n, |i| y.int_at(i).unwrap().abs())?),
                )
            } else {
                Value::new(
                    Shape::from(y.shape),
                    Data::Float(CpuStorage::generate(n, |i| y.float_at(i).unwrap().abs())?),
                )
            }
        }
        // Identity/ravel need an owned output; i. uses its small dimension input.
        _ => monad(verb, y.to_owned()?),
    }
}

pub fn assemble(shape: Vec<usize>, cells: Vec<Value>) -> Result<Value> {
    if cells.iter().any(Value::is_sparse) {
        return Err(Error::Unsupported("sparse assembly".into()));
    }
    let n = count(&shape)?;
    if cells.iter().any(|x| matches!(x.data, Data::Boxed(_))) {
        let mut out = buffer(n)?;
        for c in cells {
            if let Data::Boxed(v) = c.data {
                out.extend_from_slice(&v);
            } else {
                return Err(Error::Domain);
            }
        }
        return Value::new(shape, Data::Boxed(CpuStorage::new(out)));
    }
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
        return Value::new(shape, Data::Char(CpuStorage::new(out)));
    }
    if cells.iter().any(|x| matches!(x.data, Data::Float(_))) {
        let mut out = buffer(n)?;
        for c in cells {
            for i in 0..c.len() {
                out.push(c.float_at(i)?);
            }
        }
        Value::new(shape, Data::Float(CpuStorage::new(out)))
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
        Value::new(shape, Data::Bool(CpuStorage::new(out)))
    }
}

/// Apply a dyad to cells selected by a scalar rank. Shorter frames repeat
/// across the unmatched suffix of the longer frame (J prefix agreement).
/// Empty-frame prototypes and differently shaped result cells remain unsupported.
pub fn ranked_dyad(verb: &str, rank: i64, a: Value, b: Value) -> Result<Value> {
    ranked_dyad_ranks(verb, rank, rank, a, b)
}

/// Independent left and right cell ranks, with J prefix frame agreement.
pub fn ranked_dyad_ranks(verb: &str, left: i64, right: i64, a: Value, b: Value) -> Result<Value> {
    let cell_rank = |r: usize, rank: i64| {
        if rank < 0 {
            r.saturating_sub(rank.unsigned_abs() as usize)
        } else {
            r.min(rank as usize)
        }
    };
    let ar = cell_rank(a.shape.len(), left);
    let br = cell_rank(b.shape.len(), right);
    let af = &a.shape[..a.shape.len() - ar];
    let bf = &b.shape[..b.shape.len() - br];
    if af.is_empty() && bf.is_empty() {
        return dyad(verb, a, b);
    }
    let (short, frame) = if af.len() <= bf.len() {
        (af, bf)
    } else {
        (bf, af)
    };
    if !frame.starts_with(short) {
        return Err(Error::Length);
    }
    let frames = count(frame)?;
    if frames == 0 {
        return Err(Error::Unsupported(
            "dyadic rank over empty frame (prototype inference)".into(),
        ));
    }
    let ad = count(&frame[af.len()..])?;
    let bd = count(&frame[bf.len()..])?;
    let evaluate = |i| {
        let x = a.view().cell(ar, i / ad)?;
        let y = b.view().cell(br, i / bd)?;
        match verb {
            "+" => arithmetic_views(Op::Add, x, y),
            "-" => arithmetic_views(Op::Sub, x, y),
            "*" => arithmetic_views(Op::Mul, x, y),
            "%" => arithmetic_views(Op::Div, x, y),
            _ => dyad(verb, x.to_owned()?, y.to_owned()?),
        }
    };
    let first = evaluate(0)?;
    let result_shape = first.shape.clone();
    let mut shape = Shape::from(frame);
    shape.extend_from_slice(&result_shape);
    let mut out = crate::assembly::CellBuilder::new(&first, count(&shape)?)?;
    drop(first);
    let mut mismatch = false;
    let mut error = None;
    for i in 1..frames {
        let cell = evaluate(i)?;
        mismatch |= cell.shape != result_shape;
        if !mismatch && error.is_none() {
            if let Err(e) = out.push(&cell) {
                error = Some(e);
            }
        }
    }
    if mismatch {
        return Err(Error::Unsupported("rank result padding".into()));
    }
    if let Some(e) = error {
        return Err(e);
    }
    Value::new(shape, out.finish())
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
        if verb == "," && !reduction && !y.is_sparse() {
            // Ravel is pure and preserves atom type/order. Its prototype shape
            // follows from the cell shape without invoking an unknown verb.
            let atoms = count(&y.shape[f..])?;
            let mut shape = y.shape[..f].to_vec();
            shape.push(atoms);
            return y.select(shape, std::iter::empty());
        }
        return Err(Error::Unsupported(
            "rank over empty frame (prototype inference)".into(),
        ));
    }
    let evaluate_cell = |i| {
        let cell = y.view().cell(r, i)?;
        if reduction {
            reduce_view(verb, cell)
        } else {
            monad_view(verb, cell)
        }
    };
    let first = evaluate_cell(0)?;
    let result_shape = first.shape.clone();
    let mut shape = Shape::from(&y.shape[..f]);
    shape.extend_from_slice(&result_shape);
    let mut out = crate::assembly::CellBuilder::new(&first, count(&shape)?)?;
    drop(first);
    let mut shape_mismatch = false;
    let mut assembly_error = None;
    for i in 1..frames {
        let cell = evaluate_cell(i)?;
        if cell.shape != result_shape {
            shape_mismatch = true;
        }
        // As before, evaluate every cell before reporting assembly errors.
        // A later cell's domain error must not be masked by an earlier shape.
        if !shape_mismatch && assembly_error.is_none() {
            if let Err(error) = out.push(&cell) {
                assembly_error = Some(error);
            }
        }
    }
    if shape_mismatch {
        return Err(Error::Unsupported("rank result padding".into()));
    }
    if let Some(error) = assembly_error {
        return Err(error);
    }
    Value::new(shape, out.finish())
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
        let result = result.into_shared();
        let saved = result.clone();
        let changed = atomic(Op::Add, result, Value::scalar(10)).unwrap();
        assert_eq!(saved.display(), "4 5 6");
        assert_eq!(changed.display(), "14 15 16");
    }
}

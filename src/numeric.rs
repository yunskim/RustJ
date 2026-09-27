use crate::pool::OutputPool;
// Typed, one-pass dense kernels. Inspired by jsrc/ve.c's overflow aggregation
// and exceptional repair strategy; no C code or C layout is used here.
use crate::storage::CpuStorage;
use crate::{
    error::Result,
    value::{Data, buffer, finish_initialized, generate},
};

#[inline(always)]
fn step<const OP: u8>(a: i64, b: i64, overflow: &mut u64) -> i64 {
    match OP {
        0 => {
            let r = a.wrapping_add(b);
            *overflow |= (!(a ^ b) & (a ^ r)) as u64;
            r
        }
        1 => {
            let r = a.wrapping_sub(b);
            *overflow |= ((a ^ b) & (a ^ r)) as u64;
            r
        }
        _ => {
            let (r, bad) = a.overflowing_mul(b);
            *overflow |= (bad as u64) << 63;
            r
        }
    }
}
#[inline(always)]
fn real<const OP: u8>(a: i64, b: i64) -> f64 {
    match OP {
        0 => a as f64 + b as f64,
        1 => a as f64 - b as f64,
        _ => a as f64 * b as f64,
    }
}

fn fresh<const OP: u8>(
    n: usize,
    a: impl Fn(usize) -> i64,
    b: impl Fn(usize) -> i64,
    pool: &mut OutputPool,
) -> Result<Data> {
    if n == 1 {
        let (x, y) = (a(0), b(0));
        let mut bad = 0;
        let z = step::<OP>(x, y, &mut bad);
        return Ok(if (bad as i64) < 0 {
            Data::Float(CpuStorage::Inline(real::<OP>(x, y)))
        } else {
            Data::Int(CpuStorage::Inline(z))
        });
    }
    let mut overflow = 0u64;
    let mut out = pool.take(n)?;
    out.extend((0..n).map(|i| step::<OP>(a(i), b(i), &mut overflow)));
    if (overflow as i64) < 0 {
        // Original inputs have not been modified. Promote the complete array.
        drop(out);
        Ok(Data::Float(CpuStorage::new(generate(n, |i| {
            real::<OP>(a(i), b(i))
        })?)))
    } else {
        Ok(Data::Int(CpuStorage::new(out)))
    }
}

#[allow(unsafe_code)]
fn fresh_pair<const OP: u8>(n: usize, x: &[i64], y: &[i64], pool: &mut OutputPool) -> Result<Data> {
    if n >= 64 && (OP < 2 || x.len() == 1 || y.len() == 1) && crate::simd::available() {
        let mut out = pool.take(n)?;
        let slots = &mut out.spare_capacity_mut()[..n];
        let simd = if OP == 2 && y.len() == 1 {
            crate::simd::mul_scalar(x, y[0], slots)
        } else if OP == 2 && x.len() == 1 {
            crate::simd::mul_scalar(y, x[0], slots)
        } else {
            crate::simd::fill::<OP>(x, y, slots)
        };
        if let Some(bad) = simd {
            if bad {
                drop(out);
                return Ok(Data::Float(CpuStorage::new(generate(n, |i| {
                    real::<OP>(
                        x[if x.len() == 1 { 0 } else { i }],
                        y[if y.len() == 1 { 0 } else { i }],
                    )
                })?)));
            }
            // SAFETY: simd::fill returned Some, guaranteeing n initialized slots.
            return Ok(Data::Int(CpuStorage::new(unsafe {
                finish_initialized(out, n)
            })));
        }
    }
    if x.len() == 1 {
        let a = x[0];
        fresh::<OP>(n, |_| a, |i| y[i], pool)
    } else if y.len() == 1 {
        let b = y[0];
        fresh::<OP>(n, |i| x[i], |_| b, pool)
    } else {
        fresh::<OP>(n, |i| x[i], |i| y[i], pool)
    }
}

fn reuse<const OP: u8, const SWAP: bool>(mut out: Vec<i64>, other: &[i64]) -> Result<Data> {
    let mut overflow = 0u64;
    let bad = crate::simd::inplace::<OP, SWAP>(&mut out, other);
    let other = |i| other[if other.len() == 1 { 0 } else { i }];
    if bad.is_none() {
        for (i, x) in out.iter_mut().enumerate() {
            *x = if SWAP {
                step::<OP>(other(i), *x, &mut overflow)
            } else {
                step::<OP>(*x, other(i), &mut overflow)
            };
        }
    }
    if !bad.unwrap_or((overflow as i64) < 0) {
        return Ok(Data::Int(CpuStorage::new(out)));
    }
    // Add/sub are reversible modulo 2^64. Recover the overwritten operand
    // exactly, then convert both original integers separately, as J does.
    // Multiplication never enters this path: it is not generally reversible.
    debug_assert!(OP < 2);
    let floats = generate(out.len(), |i| {
        let b = other(i);
        let r = out[i];
        let original = if OP == 0 {
            r.wrapping_sub(b)
        } else if SWAP {
            b.wrapping_sub(r)
        } else {
            r.wrapping_add(b)
        };
        if SWAP {
            real::<OP>(b, original)
        } else {
            real::<OP>(original, b)
        }
    })?;
    Ok(Data::Float(CpuStorage::new(floats)))
}

/// A right-fold accumulator is owned; its next left input is a borrowed cell.
/// Add/sub can recover its original elements if the entire cell must promote.
pub(crate) fn int_accumulate<const OP: u8>(left: &[i64], right: CpuStorage<i64>) -> Result<Data> {
    if left.is_empty() {
        return Ok(Data::Int(right));
    }
    if OP < 2 {
        match right.try_into_vec() {
            Ok(out) => reuse::<OP, true>(out, left),
            Err(right) => fresh_pair::<OP>(left.len(), left, &right, &mut OutputPool::default()),
        }
    } else {
        fresh_pair::<OP>(left.len(), left, &right, &mut OutputPool::default())
    }
}

pub(crate) fn int<const OP: u8>(
    x: CpuStorage<i64>,
    y: CpuStorage<i64>,
    n: usize,
    ad: usize,
    bd: usize,
    pool: &mut OutputPool,
) -> Result<Data> {
    if n == 0 {
        return Ok(Data::Int(CpuStorage::new(Vec::new())));
    }
    if ad == 1 && bd == 1 {
        if OP < 2 {
            match x.try_into_vec() {
                Ok(out) => return reuse::<OP, false>(out, &y),
                Err(x) => return fresh_pair::<OP>(n, &x, &y, pool),
            }
        }
        return fresh_pair::<OP>(n, &x, &y, pool);
    }
    if y.len() == 1 {
        if OP < 2 {
            match x.try_into_vec() {
                Ok(out) => return reuse::<OP, false>(out, &y),
                Err(x) => return fresh_pair::<OP>(n, &x, &y, pool),
            }
        }
        return fresh_pair::<OP>(n, &x, &y, pool);
    }
    if x.len() == 1 {
        if OP < 2 {
            match y.try_into_vec() {
                Ok(out) => return reuse::<OP, true>(out, &x),
                Err(y) => return fresh_pair::<OP>(n, &x, &y, pool),
            }
        }
        return fresh_pair::<OP>(n, &x, &y, pool);
    }
    fresh::<OP>(n, |i| x[i / ad], |i| y[i / bd], pool)
}

// Float add/sub need no integer promotion pass. Preserve the J-specific
// multiply/divide handling in the general implementation for now.
#[allow(unsafe_code)]
pub(crate) fn float<const SUB: bool>(
    x: CpuStorage<f64>,
    y: CpuStorage<f64>,
    n: usize,
    ad: usize,
    bd: usize,
) -> Result<Data> {
    let op = |a: f64, b: f64| if SUB { a - b } else { a + b };
    if n == 1 {
        return Ok(Data::Float(CpuStorage::Inline(op(x[0], y[0]))));
    }
    if n == 0 {
        return Ok(Data::Float(CpuStorage::new(Vec::new())));
    }
    if n >= 64
        && crate::simd::available()
        && ((x.len() == n || x.len() == 1) && (y.len() == n || y.len() == 1))
    {
        let mut out = buffer::<f64>(n)?;
        if crate::simd::float_fill::<SUB>(&x, &y, &mut out.spare_capacity_mut()[..n]) {
            // SAFETY: true means all n output slots were written by float_fill.
            return Ok(Data::Float(CpuStorage::new(unsafe {
                finish_initialized(out, n)
            })));
        }
    }
    let out = if ad == 1 && bd == 1 {
        generate(n, |i| op(x[i], y[i]))?
    } else if y.len() == 1 {
        let b = y[0];
        generate(n, |i| op(x[i], b))?
    } else if x.len() == 1 {
        let a = x[0];
        generate(n, |i| op(a, y[i]))?
    } else {
        generate(n, |i| op(x[i / ad], y[i / bd]))?
    };
    Ok(Data::Float(CpuStorage::new(out)))
}

//! Safe length-checked entry points to a small AVX2 implementation.
//! Generic builds retain a portable fallback; no global target-cpu requirement.
use std::mem::MaybeUninit;

pub(crate) fn available() -> bool {
    if cfg!(feature = "portable") {
        return false;
    }
    #[cfg(target_arch = "x86_64")]
    {
        std::is_x86_feature_detected!("avx2")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

pub(crate) fn float_fill<const SUB: bool>(
    a: &[f64],
    b: &[f64],
    out: &mut [MaybeUninit<f64>],
) -> bool {
    let n = out.len();
    if n < 64
        || !available()
        || !((a.len() == n || a.len() == 1) && (b.len() == n || b.len() == 1))
        || (a.len() == 1 && b.len() == 1)
    {
        return false;
    }
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: features, lengths and disjoint output are established above.
        unsafe {
            if a.len() == 1 {
                float_run::<SUB, 2>(a.as_ptr(), b.as_ptr(), out.as_mut_ptr().cast(), n);
            } else if b.len() == 1 {
                float_run::<SUB, 1>(a.as_ptr(), b.as_ptr(), out.as_mut_ptr().cast(), n);
            } else {
                float_run::<SUB, 0>(a.as_ptr(), b.as_ptr(), out.as_mut_ptr().cast(), n);
            }
        }
        true
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn float_run<const SUB: bool, const MODE: u8>(
    a: *const f64,
    b: *const f64,
    out: *mut f64,
    n: usize,
) {
    use std::arch::x86_64::*;
    // SAFETY: n elements or scalar are available, and stores cover initialized
    // output slots only. Floating operations do not reassociate or use FMA.
    unsafe {
        let ax = if MODE == 2 {
            _mm256_set1_pd(*a)
        } else {
            _mm256_setzero_pd()
        };
        let by = if MODE == 1 {
            _mm256_set1_pd(*b)
        } else {
            _mm256_setzero_pd()
        };
        let mut i = 0;
        while i + 16 <= n {
            for lane in 0..4 {
                let j = i + lane * 4;
                let x = if MODE == 2 {
                    ax
                } else {
                    _mm256_loadu_pd(a.add(j))
                };
                let y = if MODE == 1 {
                    by
                } else {
                    _mm256_loadu_pd(b.add(j))
                };
                let z = if SUB {
                    _mm256_sub_pd(x, y)
                } else {
                    _mm256_add_pd(x, y)
                };
                _mm256_storeu_pd(out.add(j), z);
            }
            i += 16;
        }
        while i < n {
            let x = *a.add(if MODE == 2 { 0 } else { i });
            let y = *b.add(if MODE == 1 { 0 } else { i });
            *out.add(i) = if SUB { x - y } else { x + y };
            i += 1;
        }
    }
}

pub(crate) fn mul_scalar(a: &[i64], b: i64, out: &mut [MaybeUninit<i64>]) -> Option<bool> {
    if a.len() != out.len() || a.len() < 64 || !available() {
        return None;
    }
    let (lo, hi) = if b == 0 {
        (i64::MIN, i64::MAX)
    } else {
        let b = b as i128;
        let (lo, hi) = if b > 0 {
            (i64::MIN as i128 / b, i64::MAX as i128 / b)
        } else {
            (i64::MAX as i128 / b, i64::MIN as i128 / b)
        };
        (
            lo.clamp(i64::MIN as i128, i64::MAX as i128) as i64,
            hi.clamp(i64::MIN as i128, i64::MAX as i128) as i64,
        )
    };
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: feature/length checks and safe borrows establish disjointness.
        Some(unsafe { mul_run(a.as_ptr(), b, out.as_mut_ptr().cast(), a.len(), lo, hi) })
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (lo, hi);
        None
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn mul_run(a: *const i64, b: i64, out: *mut i64, n: usize, lo: i64, hi: i64) -> bool {
    use std::arch::x86_64::*;
    // SAFETY: all loads/stores bounded by n. Low 64-bit product is assembled
    // from 32-bit unsigned partial products; signed bounds detect overflow.
    unsafe {
        let bv = _mm256_set1_epi64x(b);
        let bhi = _mm256_srli_epi64(bv, 32);
        let low = _mm256_set1_epi64x(lo);
        let high = _mm256_set1_epi64x(hi);
        let mut flags = _mm256_setzero_si256();
        let mut i = 0;
        while i + 4 <= n {
            let x = _mm256_loadu_si256(a.add(i).cast());
            flags = _mm256_or_si256(
                flags,
                _mm256_or_si256(_mm256_cmpgt_epi64(low, x), _mm256_cmpgt_epi64(x, high)),
            );
            let ll = _mm256_mul_epu32(x, bv);
            let cross = _mm256_add_epi64(
                _mm256_mul_epu32(_mm256_srli_epi64(x, 32), bv),
                _mm256_mul_epu32(x, bhi),
            );
            let z = _mm256_add_epi64(ll, _mm256_slli_epi64(cross, 32));
            _mm256_storeu_si256(out.add(i).cast(), z);
            i += 4;
        }
        let mut bad = _mm256_movemask_pd(_mm256_castsi256_pd(flags)) != 0;
        while i < n {
            let (r, o) = (*a.add(i)).overflowing_mul(b);
            *out.add(i) = r;
            bad |= o;
            i += 1;
        }
        bad
    }
}

pub(crate) fn fill<const OP: u8>(
    a: &[i64],
    b: &[i64],
    out: &mut [MaybeUninit<i64>],
) -> Option<bool> {
    let n = out.len();
    if OP > 1 || n < 64 || !available() {
        return None;
    }
    if !((a.len() == n || a.len() == 1) && (b.len() == n || b.len() == 1)) {
        return None;
    }
    if a.len() == 1 && b.len() == 1 {
        return None;
    }
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: CPU checked; inputs cover n elements or are scalar; output
        // covers n slots. Safe references make fresh output disjoint from input.
        Some(unsafe {
            dispatch::<OP>(
                a.as_ptr(),
                b.as_ptr(),
                out.as_mut_ptr().cast(),
                n,
                a.len() == 1,
                b.len() == 1,
            )
        })
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        None
    }
}

pub(crate) fn inplace<const OP: u8, const SWAP: bool>(
    out: &mut [i64],
    other: &[i64],
) -> Option<bool> {
    let n = out.len();
    if OP > 1 || n < 64 || !available() || !(other.len() == n || other.len() == 1) {
        return None;
    }
    #[cfg(target_arch = "x86_64")]
    {
        let p = out.as_mut_ptr();
        let (a, b, ascalar, bscalar) = if SWAP {
            (other.as_ptr(), p.cast_const(), other.len() == 1, false)
        } else {
            (p.cast_const(), other.as_ptr(), false, other.len() == 1)
        };
        // SAFETY: CPU/lengths checked; one input aliases output exactly, the
        // other cannot alias via safe references. Each vector is loaded before
        // its corresponding store; no shifted overlap or retained pointers.
        Some(unsafe { dispatch::<OP>(a, b, p, n, ascalar, bscalar) })
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        None
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn dispatch<const OP: u8>(
    a: *const i64,
    b: *const i64,
    out: *mut i64,
    n: usize,
    ascalar: bool,
    bscalar: bool,
) -> bool {
    // SAFETY: caller establishes AVX2 and pointer/aliasing preconditions.
    unsafe {
        if ascalar {
            run::<OP, 2>(a, b, out, n)
        } else if bscalar {
            run::<OP, 1>(a, b, out, n)
        } else {
            run::<OP, 0>(a, b, out, n)
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn run<const OP: u8, const MODE: u8>(
    a: *const i64,
    b: *const i64,
    out: *mut i64,
    n: usize,
) -> bool {
    use std::arch::x86_64::*;
    // SAFETY: entry wrappers check features and lengths. All loads/stores are
    // unaligned operations bounded by n; scalar inputs only read element zero.
    unsafe {
        let zero = _mm256_setzero_si256();
        let ax = if MODE == 2 {
            _mm256_set1_epi64x(*a)
        } else {
            zero
        };
        let by = if MODE == 1 {
            _mm256_set1_epi64x(*b)
        } else {
            zero
        };
        let mut flags = [zero; 4];
        let mut i = 0;
        while i + 16 <= n {
            for (lane, flag) in flags.iter_mut().enumerate() {
                let j = i + lane * 4;
                let x = if MODE == 2 {
                    ax
                } else {
                    _mm256_loadu_si256(a.add(j).cast())
                };
                let y = if MODE == 1 {
                    by
                } else {
                    _mm256_loadu_si256(b.add(j).cast())
                };
                let z = if OP == 0 {
                    _mm256_add_epi64(x, y)
                } else {
                    _mm256_sub_epi64(x, y)
                };
                let xy = _mm256_xor_si256(x, y);
                let xz = _mm256_xor_si256(x, z);
                let overflow = if OP == 0 {
                    _mm256_andnot_si256(xy, xz)
                } else {
                    _mm256_and_si256(xy, xz)
                };
                *flag = _mm256_or_si256(*flag, overflow);
                _mm256_storeu_si256(out.add(j).cast(), z);
            }
            i += 16;
        }
        let mut bad = _mm256_movemask_pd(_mm256_castsi256_pd(_mm256_or_si256(
            _mm256_or_si256(flags[0], flags[1]),
            _mm256_or_si256(flags[2], flags[3]),
        ))) != 0;
        while i < n {
            let x = *a.add(if MODE == 2 { 0 } else { i });
            let y = *b.add(if MODE == 1 { 0 } else { i });
            let (r, overflow) = if OP == 0 {
                x.overflowing_add(y)
            } else {
                x.overflowing_sub(y)
            };
            *out.add(i) = r;
            bad |= overflow;
            i += 1;
        }
        bad
    }
}

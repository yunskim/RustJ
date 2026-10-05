//! Numeric-word recognition from wn.c. Validation does not construct exact,
//! complex, based or precision payloads and never invents a J error for unknowns.
use crate::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Check {
    Valid,
    Invalid,
    Unknown,
}
impl Check {
    fn and(self, other: Self) -> Self {
        if self == Self::Invalid || other == Self::Invalid {
            Self::Invalid
        } else if self == Self::Unknown || other == Self::Unknown {
            Self::Unknown
        } else {
            Self::Valid
        }
    }
}
#[derive(Clone, Copy)]
enum Mode {
    Real,
    Extended,
    Rational,
    Complex,
    Quad,
    ReferenceBoundary,
}

fn mode(source: &str) -> Mode {
    let bytes = source.as_bytes();
    let complex = source.contains(['j', 'a']);
    let mut based = source.contains(['b', 'p']);
    let (mut half_single, mut quad) = (false, false);
    for i in 1..bytes.len().saturating_sub(1) {
        if bytes[i] == b'f'
            && (bytes[i - 1].is_ascii_digit()
                || (i > 1 && bytes[i - 1] == b'.' && bytes[i - 2].is_ascii_digit()))
        {
            half_single |= matches!(bytes[i + 1], b'h' | b's');
            quad |= bytes[i + 1] == b'q';
        }
    }
    based |= (half_single || quad) && source.contains('x');
    if half_single || (quad && based) {
        return Mode::ReferenceBoundary;
    }
    if !complex && !based && !quad {
        let has_x = source.contains('x');
        let has_r = source.contains('r');
        let inexact = source.contains(['.', 'e'])
            || source.split_ascii_whitespace().any(|part| {
                part.bytes()
                    .enumerate()
                    .any(|(i, b)| b == b'x' && i + 1 < part.len())
            });
        if !inexact {
            if has_r
                || (has_x
                    && source
                        .split_ascii_whitespace()
                        .any(|s| s == "_" || s == "__"))
            {
                return Mode::Rational;
            }
            if has_x {
                return Mode::Extended;
            }
        }
        if has_x {
            return Mode::Complex;
        }
    }
    if complex || based {
        Mode::Complex
    } else if quad {
        Mode::Quad
    } else {
        Mode::Real
    }
}

fn integer(s: &str, suffix: bool) -> Check {
    let s = if suffix {
        s.strip_suffix('x').unwrap_or(s)
    } else {
        s
    };
    let s = s.strip_prefix('_').unwrap_or(s);
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        Check::Valid
    } else {
        Check::Invalid
    }
}

// Match the hexadecimal part of Windows strtod without constructing a value.
fn hex_prefix(s: &str) -> Option<(usize, &str, &str)> {
    let unsigned = s
        .strip_prefix('_')
        .or_else(|| s.strip_prefix('+'))
        .unwrap_or(s);
    let sign = s.len() - unsigned.len();
    let body = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))?;
    let bytes = body.as_bytes();
    let mut i = 0;
    let mut digits = 0;
    while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
        i += 1;
        digits += 1;
    }
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    let mantissa = &body[..i];
    let mut exponent = "0";
    if matches!(bytes.get(i), Some(b'p' | b'P')) {
        let marker = i;
        i += 1;
        let start = i;
        if matches!(bytes.get(i), Some(b'_' | b'+')) {
            i += 1;
        }
        let first_digit = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == first_digit {
            i = marker;
        } else {
            exponent = &body[start..i];
        }
    }
    Some((sign + 2 + i, mantissa, exponent))
}

// A negative hex magnitude is invalid for polar input only when its leading
// bit and sticky remainder prove its sign after default round-to-nearest-even.
// This validates the polar sign, not the rounded numeric payload.
fn hex_nonnegative(s: &str) -> Check {
    let Some((_, mantissa, exponent)) = hex_prefix(s) else {
        return Check::Unknown;
    };
    if !s.starts_with('_') {
        return Check::Valid;
    }
    let integer_digits = mantissa.split('.').next().unwrap().len();
    let first = mantissa
        .bytes()
        .filter(|b| *b != b'.')
        .enumerate()
        .find(|(_, b)| *b != b'0');
    let Some((index, digit)) = first else {
        return Check::Valid;
    };
    // The prefix scanner has already proved the exponent's decimal grammar.
    // Saturation preserves its sign relative to the zero-rounding threshold:
    // any usize-sized mantissa offset is far smaller than i128's range on
    // supported 64-bit hosts. This is not a numeric payload conversion.
    let exponent = exponent
        .replace('_', "-")
        .parse::<i128>()
        .unwrap_or_else(|_| {
            if exponent.starts_with('_') {
                i128::MIN
            } else {
                i128::MAX
            }
        });
    let (Ok(integer_digits), Ok(index)) = (i128::try_from(integer_digits), i128::try_from(index))
    else {
        return Check::Unknown;
    };
    let nibble = (digit as char).to_digit(16).expect("validated hex digit");
    let bit = i128::from(31 - nibble.leading_zeros());
    let leading = integer_digits
        .checked_sub(index)
        .and_then(|n| n.checked_sub(1))
        .and_then(|n| n.checked_mul(4))
        .and_then(|n| n.checked_add(bit))
        .map(|n| n.saturating_add(exponent));
    match leading {
        Some(n) if n < -1075 => Check::Valid,
        Some(-1075) => {
            // Half the minimum subnormal rounds to even zero. Any lower set
            // bit puts the magnitude above that midpoint, rounding away from zero.
            let exact_power = nibble.is_power_of_two()
                && mantissa
                    .bytes()
                    .filter(|b| *b != b'.')
                    .skip(index as usize + 1)
                    .all(|b| b == b'0');
            if exact_power {
                Check::Valid
            } else {
                Check::Invalid
            }
        }
        Some(_) => Check::Invalid,
        None => Check::Unknown,
    }
}

fn decimal(s: &str, window: &str) -> Check {
    // Parenthesized NaN payloads require a separate word-formation audit.
    if s.contains(['(', ')']) {
        return Check::Unknown;
    }
    let unsigned = s
        .strip_prefix('_')
        .or_else(|| s.strip_prefix('+'))
        .unwrap_or(s);
    if unsigned.starts_with("0x") || unsigned.starts_with("0X") {
        return match hex_prefix(window) {
            Some((consumed, _, _)) if consumed >= s.len() => Check::Valid,
            _ => Check::Invalid,
        };
    }
    if s.replace('_', "-").parse::<f64>().is_ok() {
        Check::Valid
    } else {
        Check::Invalid
    }
}
fn real(s: &str, window: &str) -> Check {
    if matches!(s, "_" | "__" | "_.") {
        return Check::Valid;
    }
    if let Some((n, d)) = s.split_once('r') {
        (if n.is_empty() {
            Check::Valid
        } else {
            decimal(n, window)
        })
        .and(decimal(d, &window[n.len() + 1..]))
    } else {
        decimal(s, window)
    }
}
fn real_value(s: &str) -> Option<f64> {
    match s {
        "_" => return Some(f64::INFINITY),
        "__" => return Some(f64::NEG_INFINITY),
        "_." => return Some(f64::NAN),
        _ => {}
    }
    if let Some((n, d)) = s.split_once('r') {
        let n = if n.is_empty() {
            0.0
        } else {
            n.replace('_', "-").parse::<f64>().ok()?
        };
        let d = d.replace('_', "-").parse::<f64>().ok()?;
        if d == 0.0 {
            let sign = if n.is_sign_negative() ^ d.is_sign_negative() {
                -1.0
            } else {
                1.0
            };
            Some(if n == 0.0 {
                0.0_f64.copysign(sign)
            } else {
                f64::INFINITY.copysign(sign)
            })
        } else {
            Some(n / d)
        }
    } else {
        s.replace('_', "-").parse().ok()
    }
}
fn rational(s: &str) -> Check {
    if s == "_" || s == "__" {
        return Check::Valid;
    }
    if let Some(rest) = s.strip_prefix("__r").or_else(|| s.strip_prefix("_r")) {
        let rest =
            if rest.starts_with('_') && rest.as_bytes().get(1).is_some_and(u8::is_ascii_digit) {
                &rest[1..]
            } else {
                rest
            };
        return if rest.bytes().all(|b| b.is_ascii_digit()) {
            Check::Valid
        } else {
            Check::Invalid
        };
    }
    if let Some((n, d)) = s.split_once('r') {
        let denominator = if matches!(d, "_" | "__") {
            Check::Valid
        } else {
            integer(d, false)
        };
        // numr explicitly rejects a trailing x on the denominator.
        integer(n, true).and(denominator)
    } else {
        integer(s, true)
    }
}
fn complex(s: &str, window: &str) -> Check {
    if let Some((x, y)) = s.split_once('j') {
        return real(x, window).and(real(y, &window[x.len() + 1..]));
    }
    if let Some((m, angle)) = s.split_once('a') {
        let Some(angle) = angle.strip_prefix('d').or_else(|| angle.strip_prefix('r')) else {
            return Check::Invalid;
        };
        let check = real(m, window).and(real(angle, &window[m.len() + 2..]));
        if check != Check::Valid {
            return check;
        }
        return match real_value(m) {
            Some(x) if x >= 0.0 => Check::Valid,
            Some(_) => Check::Invalid,
            None if !m.contains('r') => hex_nonnegative(window),
            None => Check::Unknown,
        };
    }
    real(s, window)
}
fn based(s: &str, window: &str) -> Check {
    if let Some((base, digits)) = s.split_once('b') {
        let digits = digits.strip_prefix('_').unwrap_or(digits);
        let mut dots = 0;
        let digit_check = if !digits.is_empty()
            && digits != "."
            && digits.bytes().all(|b| {
                if b == b'.' {
                    dots += 1;
                    dots <= 1
                } else {
                    b.is_ascii_digit() || b.is_ascii_lowercase()
                }
            }) {
            Check::Valid
        } else {
            Check::Invalid
        };
        return based(base, window).and(digit_check);
    }
    if let Some((x, y)) = s.split_once('p').or_else(|| s.split_once('x')) {
        return complex(x, x).and(complex(y, &window[x.len() + 1..]));
    }
    complex(s, window)
}
fn quad(s: &str) -> Check {
    let s = s.strip_suffix("fq").unwrap_or(s);
    if matches!(s, "_" | "__" | "_.") {
        return Check::Valid;
    }
    let s = s.strip_prefix('_').unwrap_or(s);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return Check::Invalid;
    }
    let mut fraction = 0;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        fraction = i - start;
    }
    let exponent = if i == bytes.len() {
        0
    } else {
        if bytes.get(i) != Some(&b'e') {
            return Check::Invalid;
        }
        let exponent = s[i + 1..].strip_prefix('+').unwrap_or(&s[i + 1..]);
        if integer(exponent, false) != Check::Valid {
            return Check::Invalid;
        }
        let Ok(exponent) = exponent.replace('_', "-").parse::<i64>() else {
            return Check::Invalid;
        };
        exponent
    };
    // numfq combines fractional scale and a machine-integer exponent. Keep
    // unproved signed-overflow/resource behavior out of lexical error claims.
    let Ok(fraction) = i64::try_from(fraction) else {
        return Check::Unknown;
    };
    if exponent.checked_sub(fraction).is_none() {
        Check::Unknown
    } else {
        Check::Valid
    }
}

pub(crate) fn validate(source: &str) -> Result<()> {
    // Keep ordinary decimal/integer nouns on the existing constructor path;
    // no extra float parsing or temporary normalized strings for these words.
    if source.bytes().all(|b| {
        b.is_ascii_digit() || b.is_ascii_whitespace() || matches!(b, b'_' | b'.' | b'e' | b'E')
    }) {
        return Ok(());
    }
    let mode = mode(source);
    if matches!(mode, Mode::ReferenceBoundary) {
        return Err(Error::Unsupported(
            "C reference precision conversion boundary".into(),
        ));
    }
    let mut check = Check::Valid;
    for part in source.split_ascii_whitespace() {
        let next = match mode {
            Mode::Real => real(part, part),
            Mode::Extended => integer(part, true),
            Mode::Rational => rational(part),
            Mode::Complex => based(part, part),
            Mode::Quad => quad(part),
            Mode::ReferenceBoundary => unreachable!(),
        };
        check = check.and(next);
    }
    match check {
        Check::Invalid => Err(Error::IllFormedNumber),
        Check::Unknown => Err(Error::Unsupported(
            "numeric recognition requires additional grammar facts".into(),
        )),
        Check::Valid if !matches!(mode, Mode::Real) => Err(Error::Unsupported(
            "validated numeric family payload construction".into(),
        )),
        Check::Valid => Ok(()),
    }
}

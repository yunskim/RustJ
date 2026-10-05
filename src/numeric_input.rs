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

// Rust's decimal parser consumes the whole field. C strtod additionally accepts
// platform-dependent hex/NaN-payload forms: keep these explicitly unknown.
fn decimal(s: &str) -> Check {
    if s.contains(['X', '(', ')']) || s.to_ascii_lowercase().contains("0x") {
        return Check::Unknown;
    }
    if s.replace('_', "-").parse::<f64>().is_ok() {
        Check::Valid
    } else {
        Check::Invalid
    }
}
fn real(s: &str) -> Check {
    if matches!(s, "_" | "__" | "_.") {
        return Check::Valid;
    }
    if let Some((n, d)) = s.split_once('r') {
        (if n.is_empty() {
            Check::Valid
        } else {
            decimal(n)
        })
        .and(decimal(d))
    } else {
        decimal(s)
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
fn complex(s: &str) -> Check {
    if let Some((x, y)) = s.split_once('j') {
        return real(x).and(real(y));
    }
    if let Some((m, angle)) = s.split_once('a') {
        let Some(angle) = angle.strip_prefix('d').or_else(|| angle.strip_prefix('r')) else {
            return Check::Invalid;
        };
        let check = real(m).and(real(angle));
        if check != Check::Valid {
            return check;
        }
        return match real_value(m) {
            Some(x) if x >= 0.0 => Check::Valid,
            Some(_) => Check::Invalid,
            None => Check::Unknown,
        };
    }
    real(s)
}
fn based(s: &str) -> Check {
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
        return based(base).and(digit_check);
    }
    if let Some((x, y)) = s.split_once('p').or_else(|| s.split_once('x')) {
        return complex(x).and(complex(y));
    }
    complex(s)
}
fn quad(s: &str) -> Check {
    // Dedicated numfq grammar and its i64 exponent/resource boundaries are
    // a follow-up. Do not treat an unvalidated precision literal as invalid.
    let _ = s;
    Check::Unknown
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
            Mode::Real => real(part),
            Mode::Extended => integer(part, true),
            Mode::Rational => rational(part),
            Mode::Complex => based(part),
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

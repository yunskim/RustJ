//! Logical atom types, independent of array storage and kernel availability.
use std::sync::Arc;

pub use num_bigint::BigInt;
use num_rational::BigRational;

use crate::{Data, Error, Result, Value, storage::CpuStorage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DType {
    Bool,
    Int,
    Float,
    Char,
    Complex,
    ExtendedInt,
    Rational,
    Symbol,
    Boxed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

/// Canonical J rational: reduced finite n/d (d>0), zero 0/1, infinities +/-1/0.
/// Non-finite values never enter num-rational's finite arithmetic routines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rational {
    numerator: BigInt,
    denominator: BigInt,
}
impl Rational {
    pub fn new(numerator: BigInt, denominator: BigInt) -> Result<Self> {
        let zero = BigInt::from(0);
        let (numerator, denominator) = if numerator == zero {
            (zero, BigInt::from(1))
        } else if denominator == zero {
            (BigInt::from(if numerator < zero { -1 } else { 1 }), zero)
        } else {
            BigRational::new(numerator, denominator).into_raw()
        };
        Ok(Self {
            numerator,
            denominator,
        })
    }
    /// Integer promotion needs no GCD or sign normalization.
    pub(crate) fn from_integer(numerator: BigInt) -> Self {
        Self {
            numerator,
            denominator: BigInt::from(1),
        }
    }
    pub(crate) fn from_finite(value: BigRational) -> Self {
        let (numerator, denominator) = value.into_raw();
        Self {
            numerator,
            denominator,
        }
    }
    pub fn numerator(&self) -> &BigInt {
        &self.numerator
    }
    pub fn denominator(&self) -> &BigInt {
        &self.denominator
    }
    pub fn is_finite(&self) -> bool {
        self.denominator != BigInt::from(0)
    }
}
impl std::fmt::Display for Rational {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.is_finite() {
            f.write_str(if self.numerator < BigInt::from(0) {
                "__"
            } else {
                "_"
            })
        } else if self.denominator == BigInt::from(1) {
            f.write_str(&self.numerator.to_string().replace('-', "_"))
        } else {
            write!(
                f,
                "{}r{}",
                self.numerator.to_string().replace('-', "_"),
                self.denominator
            )
        }
    }
}

/// Symbol identity is its text, not a process-local pointer or numeric ID.
/// Interning can subsequently accelerate equality without changing this contract.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Symbol(Arc<str>);
impl Symbol {
    pub fn new(text: impl Into<Arc<str>>) -> Self {
        Self(text.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Compact atoms. Variable-sized immutable payloads are shared on clone.
/// Bool is the logical bit type; packed-bit arrays are a separate storage choice.
/// Boxed contains an entire noun, including its shape, rather than just an atom.
#[derive(Clone, Debug)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    Char(u8),
    Complex(Complex),
    ExtendedInt(Arc<BigInt>),
    Rational(Arc<Rational>),
    Symbol(Symbol),
    Boxed(Arc<Value>),
}
impl Scalar {
    pub fn dtype(&self) -> DType {
        match self {
            Self::Bool(_) => DType::Bool,
            Self::Int(_) => DType::Int,
            Self::Float(_) => DType::Float,
            Self::Char(_) => DType::Char,
            Self::Complex(_) => DType::Complex,
            Self::ExtendedInt(_) => DType::ExtendedInt,
            Self::Rational(_) => DType::Rational,
            Self::Symbol(_) => DType::Symbol,
            Self::Boxed(_) => DType::Boxed,
        }
    }

    /// Lower to the currently implemented CPU array representation.
    /// Never silently round exact values, discard imaginary parts or unbox nouns.
    pub fn into_value(self) -> Result<Value> {
        let data = match self {
            Self::Bool(x) => Data::Bool(CpuStorage::Inline(x as u8)),
            Self::Int(x) => Data::Int(CpuStorage::Inline(x)),
            Self::Float(x) => Data::Float(CpuStorage::Inline(x)),
            Self::Char(x) => Data::Char(CpuStorage::Inline(x)),
            Self::Rational(x) => Data::Rational(CpuStorage::Inline(x)),
            Self::ExtendedInt(x) => Data::ExtendedInt(CpuStorage::Inline(x)),
            Self::Boxed(x) => Data::Boxed(CpuStorage::Inline(x)),
            other => {
                return Err(Error::Unsupported(format!(
                    "{:?} array storage",
                    other.dtype()
                )));
            }
        };
        Value::new([], data)
    }
}

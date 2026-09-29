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

/// A finite, reduced fraction with a positive denominator.
/// J's non-finite rational semantics still require an explicit implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rational(BigRational);
impl Rational {
    pub fn new(numerator: BigInt, denominator: BigInt) -> Result<Self> {
        if denominator == BigInt::from(0) {
            return Err(Error::Unsupported("non-finite rational".into()));
        }
        Ok(Self(BigRational::new(numerator, denominator)))
    }
    pub fn numerator(&self) -> &BigInt {
        self.0.numer()
    }
    pub fn denominator(&self) -> &BigInt {
        self.0.denom()
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

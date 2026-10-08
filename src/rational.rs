//! Validated exact rational literal construction, independent of execution layout.
use crate::{
    Error, Result,
    types::{BigInt, Rational},
};

fn integer(source: &str) -> Result<BigInt> {
    source
        .strip_suffix('x')
        .unwrap_or(source)
        .replace('_', "-")
        .parse()
        .map_err(|_| Error::IllFormedNumber)
}
pub(crate) fn literal(source: &str) -> Result<Rational> {
    if matches!(source, "_" | "__") {
        return Rational::new(if source == "_" { 1 } else { -1 }.into(), 0.into());
    }
    if let Some((negative, rest)) = source
        .strip_prefix("__r")
        .map(|s| (true, s))
        .or_else(|| source.strip_prefix("_r").map(|s| (false, s)))
    {
        // numr flips the infinity sign from spelling, including _0.
        let negative = negative ^ rest.starts_with('_');
        return Rational::new(if negative { -1 } else { 1 }.into(), 0.into());
    }
    if let Some((numerator, denominator)) = source.split_once('r') {
        if matches!(denominator, "_" | "__") {
            return Rational::new(0.into(), 1.into());
        }
        return Rational::new(integer(numerator)?, integer(denominator)?);
    }
    Rational::new(integer(source)?, 1.into())
}

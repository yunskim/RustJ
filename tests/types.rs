use rustj::{
    Error, Value,
    syntax::Token,
    types::{BigInt, Complex, DType, Rational, Scalar, Symbol},
};
use std::sync::Arc;

#[test]
fn exact_atoms_preserve_precision_and_share_large_payloads() {
    let n: BigInt = "12345678901234567890123456789012345678901234567890"
        .parse()
        .unwrap();
    let atom = Scalar::ExtendedInt(Arc::new(n.clone()));
    let Scalar::ExtendedInt(copy) = atom.clone() else {
        panic!()
    };
    let Scalar::ExtendedInt(original) = &atom else {
        panic!()
    };
    assert!(Arc::ptr_eq(original, &copy));
    assert_eq!(copy.to_string(), n.to_string());
    let fraction = Rational::new(n.clone() * 2, BigInt::from(-6)).unwrap();
    assert_eq!(fraction.numerator(), &(-n / 3));
    assert_eq!(fraction.denominator(), &BigInt::from(1));
    assert!(matches!(
        Rational::new(1.into(), 0.into()),
        Err(Error::Unsupported(_))
    ));
    let fraction = Rational::new(2.into(), (-4).into()).unwrap();
    assert_eq!(fraction.numerator(), &BigInt::from(-1));
    assert_eq!(fraction.denominator(), &BigInt::from(2));
}

#[test]
fn extended_types_cannot_silently_lower_to_existing_kernels() {
    let atoms = [
        (
            Scalar::Complex(Complex { re: 1.0, im: 2.0 }),
            DType::Complex,
        ),
        (
            Scalar::Rational(Arc::new(Rational::new(1.into(), 2.into()).unwrap())),
            DType::Rational,
        ),
        (Scalar::Symbol(Symbol::new("alpha")), DType::Symbol),
    ];
    for (atom, dtype) in atoms {
        assert_eq!(atom.dtype(), dtype);
        assert!(matches!(atom.into_value(), Err(Error::Unsupported(_))));
    }
    assert_eq!(Symbol::new(String::from("alpha")), Symbol::new("alpha"));
    assert!(std::mem::size_of::<Token<'_>>() < std::mem::size_of::<Value>());
}

#[test]
fn boxing_preserves_the_entire_noun_and_shares_it() {
    let mut engine = rustj::Engine::new();
    let noun = engine.eval("i. 2 3").unwrap().unwrap();
    let boxed = Scalar::Boxed(Arc::new(noun));
    let Scalar::Boxed(copy) = boxed.clone() else {
        panic!()
    };
    let Scalar::Boxed(original) = boxed else {
        panic!()
    };
    assert!(Arc::ptr_eq(&original, &copy));
    assert_eq!(copy.shape(), &[2, 3]);
    assert_eq!(copy.int_at(5).unwrap(), 5);
}

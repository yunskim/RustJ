//! Conservative semantic contracts, independent of physical device/layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Valence {
    Monad,
    Dyad,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Pure,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationClass {
    Map,
    Structural,
    Gather,
    Search,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueRule {
    JAtomic,
    PrimitiveSpecific,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overflow {
    WholeResultPromotion,
    PrimitiveSpecific,
    Unknown,
}
/// Shape rules describe successful results; they do not prove error freedom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeRule {
    Unknown,
    PreserveRight,
    PrefixAgreement,
    Ravel,
    ReverseAxes,
    ShapeOf,
    Tally,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contract {
    pub effect: Effect,
    pub shape_rule: ShapeRule,
    pub class: OperationClass,
    pub value_rule: ValueRule,
    pub overflow: Overflow,
    pub may_error: bool,
    pub preserve_evaluation_order: bool,
    pub alias_requires_proof: bool,
    pub allow_reassociation: bool,
}
/// Unknown names are barriers. Purity alone never grants fusion/reordering.
pub fn lookup(name: &str, valence: Valence) -> Contract {
    match crate::primitive::PrimitiveId::from_spelling(name) {
        Some(id) => for_primitive(id, valence),
        None => unknown(),
    }
}

pub fn unknown() -> Contract {
    Contract {
        effect: Effect::Unknown,
        shape_rule: ShapeRule::Unknown,
        class: OperationClass::Unknown,
        value_rule: ValueRule::Unknown,
        overflow: Overflow::Unknown,
        may_error: true,
        preserve_evaluation_order: true,
        alias_requires_proof: true,
        allow_reassociation: false,
    }
}

pub fn for_primitive(id: crate::primitive::PrimitiveId, valence: Valence) -> Contract {
    use crate::primitive::PrimitiveId::*;
    let class = match (id, valence) {
        (Add | Subtract | Multiply | Divide | Equal | Less | Greater, Valence::Dyad) => {
            OperationClass::Map
        }
        (Add | Subtract | Multiply | Divide | Magnitude, Valence::Monad) => OperationClass::Map,
        (IndexOf | Steps | Member | Find, Valence::Dyad) => OperationClass::Search,
        (From, Valence::Dyad) => OperationClass::Gather,
        (Shape | Ravel | Reverse | Take | Drop, Valence::Dyad) => OperationClass::Structural,
        (
            Shape | Tally | Ravel | IndexOf | Steps | Indices | Reverse | Transpose,
            Valence::Monad,
        ) => OperationClass::Structural,
        _ => OperationClass::Unknown,
    };
    let known = class != OperationClass::Unknown;
    Contract {
        shape_rule: match (id, valence) {
            (Add | Subtract | Multiply | Divide | Equal | Less | Greater, Valence::Dyad) => {
                ShapeRule::PrefixAgreement
            }
            (Add | Subtract | Multiply | Divide | Magnitude | Reverse, Valence::Monad) => {
                ShapeRule::PreserveRight
            }
            (Ravel, Valence::Monad) => ShapeRule::Ravel,
            (Transpose, Valence::Monad) => ShapeRule::ReverseAxes,
            (Shape, Valence::Monad) => ShapeRule::ShapeOf,
            (Tally, Valence::Monad) => ShapeRule::Tally,
            _ => ShapeRule::Unknown,
        },
        effect: if known { Effect::Pure } else { Effect::Unknown },
        class,
        value_rule: if !known {
            ValueRule::Unknown
        } else if class == OperationClass::Map && valence == Valence::Dyad {
            ValueRule::JAtomic
        } else {
            ValueRule::PrimitiveSpecific
        },
        overflow: if matches!(id, Add | Subtract | Multiply) && valence == Valence::Dyad {
            Overflow::WholeResultPromotion
        } else if known {
            Overflow::PrimitiveSpecific
        } else {
            Overflow::Unknown
        },
        may_error: true,
        preserve_evaluation_order: true,
        alias_requires_proof: true,
        allow_reassociation: false,
    }
}

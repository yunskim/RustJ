//! Conservative semantic contracts, independent of physical device/layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Valence {
    Monad,
    Dyad,
}
/// Semantic J rank, independent of jsource's integer RMAX sentinel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RankSpec {
    Infinite,
    Absolute(usize),
    /// Negative source rank resolved relative to the actual argument rank.
    Relative(i64),
}
impl RankSpec {
    pub fn from_integer(value: i64) -> Self {
        if value < 0 {
            Self::Relative(value)
        } else {
            Self::Absolute(usize::try_from(value).unwrap_or(usize::MAX))
        }
    }

    pub fn resolve(self, argument_rank: usize) -> usize {
        match self {
            Self::Infinite => argument_rank,
            Self::Absolute(rank) => rank.min(argument_rank),
            Self::Relative(delta) => {
                let raw = argument_rank as i128 + delta as i128;
                if raw <= 0 {
                    0
                } else {
                    usize::try_from(raw)
                        .unwrap_or(usize::MAX)
                        .min(argument_rank)
                }
            }
        }
    }
}

/// Monad / dyad-left / dyad-right rank contract carried by a callable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RankContract {
    pub monad: RankSpec,
    pub left: RankSpec,
    pub right: RankSpec,
}
impl RankContract {
    pub const fn new(monad: RankSpec, left: RankSpec, right: RankSpec) -> Self {
        Self { monad, left, right }
    }

    pub const fn all(rank: RankSpec) -> Self {
        Self::new(rank, rank, rank)
    }
}

impl RankContract {
    pub fn from_normalized(ranks: [i64; 3]) -> Self {
        let at = |rank| {
            if rank == 63 {
                RankSpec::Infinite
            } else {
                RankSpec::from_integer(rank)
            }
        };
        Self::new(at(ranks[0]), at(ranks[1]), at(ranks[2]))
    }
}
/// Consume the authoritative primitive registry rather than duplicating its table.
pub fn innate_rank(id: crate::primitive::PrimitiveId) -> RankContract {
    RankContract::from_normalized(id.innate_ranks())
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
    Scalar,
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
        (Sparse, _) => OperationClass::Structural,
        (Add | Subtract | Multiply | Divide | Equal | Less | Greater, Valence::Dyad) => {
            OperationClass::Map
        }
        (Add | Subtract | Multiply | Divide | Magnitude, Valence::Monad) => OperationClass::Map,
        (IndexOf | Steps | Member | Find, Valence::Dyad) => OperationClass::Search,
        (From, Valence::Dyad) => OperationClass::Gather,
        (Shape | Ravel | Reverse | Take | Drop, Valence::Dyad) => OperationClass::Structural,
        (
            Shape | Tally | Ravel | IndexOf | Steps | Indices | Reverse | Transpose | Less
            | Greater,
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
            (Add | Subtract | Multiply | Divide | Magnitude | Reverse | Sparse, Valence::Monad) => {
                ShapeRule::PreserveRight
            }
            (Ravel, Valence::Monad) => ShapeRule::Ravel,
            (Transpose, Valence::Monad) => ShapeRule::ReverseAxes,
            (Shape, Valence::Monad) => ShapeRule::ShapeOf,
            (Tally, Valence::Monad) => ShapeRule::Tally,
            (Less, Valence::Monad) => ShapeRule::Scalar,
            (Find, Valence::Dyad) => ShapeRule::PreserveRight,
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

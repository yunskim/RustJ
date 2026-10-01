//! Canonical IDs for the currently recognized primitive spellings.
//! IDs denote a J symbol, not a valence or a physical kernel.

pub const REGISTRY_VERSION: u32 = 3;
macro_rules! primitives {
    ($($id:ident => $spelling:literal),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum PrimitiveId { $($id),+ }
        impl PrimitiveId {
            pub const ALL: &'static [Self] = &[$(Self::$id),+];
            pub fn from_spelling(s: &str) -> Option<Self> {
                match s { $($spelling => Some(Self::$id),)+ _ => None }
            }
            pub const fn spelling(self) -> &'static str {
                match self { $(Self::$id => $spelling),+ }
            }
        }
    }
}
primitives! {
    Add => "+",
    Subtract => "-",
    Multiply => "*",
    Divide => "%",
    Shape => "$",
    Sparse => "$.",
    Tally => "#",
    Ravel => ",",
    Equal => "=",
    Less => "<",
    Greater => ">",
    From => "{",
    Magnitude => "|",
    IndexOf => "i.",
    Reverse => "|.",
    Transpose => "|:",
    Take => "{.",
    Drop => "}.",
    Steps => "i:",
    Indices => "I.",
    Member => "e.",
    Find => "E.",
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdverbId {
    Insert,
    /// J prefix/infix adverb `\`. The derived verb keeps the adverb
    /// identity; monadic prefix vs dyadic infix is resolved at application.
    PrefixInfix,
}
impl AdverbId {
    pub fn from_spelling(s: &str) -> Option<Self> {
        match s {
            "/" => Some(Self::Insert),
            "\\" => Some(Self::PrefixInfix),
            _ => None,
        }
    }
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Insert => "/",
            Self::PrefixInfix => "\\",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConjunctionId {
    Rank,
    Atop,
}
impl ConjunctionId {
    pub fn from_spelling(s: &str) -> Option<Self> {
        match s {
            "\"" => Some(Self::Rank),
            "@:" => Some(Self::Atop),
            _ => None,
        }
    }
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Rank => "\"",
            Self::Atop => "@:",
        }
    }
}

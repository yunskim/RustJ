//! Canonical IDs for the currently recognized primitive spellings.
//! IDs denote a J symbol, not a valence or a physical kernel.

pub const REGISTRY_VERSION: u32 = 7;
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
    OperandU => "u.",
    OperandV => "v.",
    Cap => "[:",
}

impl PrimitiveId {
    /// C t.c's intrinsic (monad, dyad-left, dyad-right) header ranks.
    /// 63 is J's unbounded-rank sentinel, not an actual argument rank.
    pub const fn innate_ranks(self) -> [i64; 3] {
        use PrimitiveId::*;
        match self {
            Add | Subtract | Multiply | Divide | Greater | Magnitude => [0, 0, 0],
            Equal | Less => [63, 0, 0],
            Shape | Tally | Reverse | Transpose | Take | Drop => [63, 1, 63],
            From => [1, 0, 63],
            IndexOf | Indices => [1, 63, 63],
            Steps | Find => [0, 63, 63],
            Sparse | Ravel | Member | OperandU | OperandV | Cap => [63; 3],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdverbId {
    Insert,
    /// J prefix/infix adverb `\`. The derived verb keeps the adverb
    /// identity; monadic prefix vs dyadic infix is resolved at application.
    PrefixInfix,
    /// Ident `]:` returns its noun or verb operand without invoking it.
    Ident,
}
impl AdverbId {
    pub fn from_spelling(s: &str) -> Option<Self> {
        match s {
            "/" => Some(Self::Insert),
            "\\" => Some(Self::PrefixInfix),
            "]:" => Some(Self::Ident),
            _ => None,
        }
    }
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Insert => "/",
            Self::PrefixInfix => "\\",
            Self::Ident => "]:",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConjunctionId {
    Rank,
    /// Historical internal name: spelling `@:` is NuVoc At, not rank-sensitive `@` Atop.
    Atop,
    Lev,
    Dex,
}
impl ConjunctionId {
    pub fn from_spelling(s: &str) -> Option<Self> {
        match s {
            "\"" => Some(Self::Rank),
            "@:" => Some(Self::Atop),
            "[." => Some(Self::Lev),
            "]." => Some(Self::Dex),
            _ => None,
        }
    }
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Rank => "\"",
            Self::Atop => "@:",
            Self::Lev => "[.",
            Self::Dex => "].",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveSemanticId {
    Verb(PrimitiveId),
    Adverb(AdverbId),
    Conjunction(ConjunctionId),
    Extension(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitivePartOfSpeech {
    Verb,
    Adverb,
    Conjunction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveSourceOrigin {
    Core,
    Extension,
}

/// Target-independent lowering identity.  Backends resolve this key only after
/// semantic parsing/lowering; the enqueuer never selects a hardware route.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoweringKey {
    Core(PrimitiveSemanticId),
    Extension(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PrimitiveSemanticInfo {
    pub registry_version: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PrimitiveHandle {
    pub semantic_id: PrimitiveSemanticId,
    pub source_origin: PrimitiveSourceOrigin,
    pub result_pos: PrimitivePartOfSpeech,
    pub semantic_info: PrimitiveSemanticInfo,
    pub lowering_key: LoweringKey,
}

impl PrimitiveHandle {
    fn core(semantic_id: PrimitiveSemanticId) -> Self {
        let result_pos = match semantic_id {
            PrimitiveSemanticId::Verb(_) => PrimitivePartOfSpeech::Verb,
            PrimitiveSemanticId::Adverb(_) => PrimitivePartOfSpeech::Adverb,
            PrimitiveSemanticId::Conjunction(_) => PrimitivePartOfSpeech::Conjunction,
            PrimitiveSemanticId::Extension(_) => {
                unreachable!("extension handles must declare their parser part of speech")
            }
        };
        Self {
            semantic_id,
            source_origin: PrimitiveSourceOrigin::Core,
            result_pos,
            semantic_info: PrimitiveSemanticInfo {
                registry_version: REGISTRY_VERSION,
            },
            lowering_key: LoweringKey::Core(semantic_id),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ExtensionPrimitive {
    pub spelling: &'static str,
    pub handle: PrimitiveHandle,
}

/// Semantic primitive lookup fixed for one compilation profile.
///
/// Core J spellings always win. Enabled extensions participate through the same
/// handle interface but cannot shadow core J semantics.
#[derive(Clone, Debug, Default)]
pub struct PrimitiveResolver {
    extensions: Vec<ExtensionPrimitive>,
}

impl PrimitiveResolver {
    pub fn core() -> Self {
        Self::default()
    }

    pub fn with_extensions(extensions: impl IntoIterator<Item = ExtensionPrimitive>) -> Self {
        Self {
            extensions: extensions.into_iter().collect(),
        }
    }

    /// Resolve only spellings that are J core primitives at enqueue time.
    ///
    /// Alphabetic extension names must remain ordinary NAMEs. Their current
    /// binding and part of speech are resolved by the parser/name environment.
    pub fn resolve_core_for_enqueue(&self, spelling: &str) -> Option<PrimitiveHandle> {
        if let Some(id) = PrimitiveId::from_spelling(spelling) {
            return Some(PrimitiveHandle::core(PrimitiveSemanticId::Verb(id)));
        }
        if let Some(id) = AdverbId::from_spelling(spelling) {
            return Some(PrimitiveHandle::core(PrimitiveSemanticId::Adverb(id)));
        }
        ConjunctionId::from_spelling(spelling)
            .map(|id| PrimitiveHandle::core(PrimitiveSemanticId::Conjunction(id)))
    }

    /// Lookup for the parser/name-binding layer after a word has entered as NAME.
    pub fn resolve_extension_binding(&self, name: &str) -> Option<PrimitiveHandle> {
        self.extensions
            .iter()
            .find(|extension| extension.spelling == name)
            .map(|extension| extension.handle)
    }
}

#[derive(Clone, Debug)]
pub struct PrimitiveContext {
    resolver: PrimitiveResolver,
}

impl PrimitiveContext {
    pub fn core() -> Self {
        Self {
            resolver: PrimitiveResolver::core(),
        }
    }

    pub fn new(resolver: PrimitiveResolver) -> Self {
        Self { resolver }
    }

    pub fn resolve_core_for_enqueue(&self, spelling: &str) -> Option<PrimitiveHandle> {
        self.resolver.resolve_core_for_enqueue(spelling)
    }

    pub fn resolve_extension_binding(&self, name: &str) -> Option<PrimitiveHandle> {
        self.resolver.resolve_extension_binding(name)
    }
}

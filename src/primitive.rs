//! Canonical IDs for the currently recognized primitive spellings.
//! IDs denote a J symbol, not a valence or a physical kernel.

pub const REGISTRY_VERSION: u32 = 10;
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
    Left => "[",
    Right => "]",
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
            Sparse | Ravel | Member | OperandU | OperandV | Cap | Left | Right => [63; 3],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdverbId {
    Insert,
    /// J `/.`: a verb-derived noun group/oblique function. Source identity
    /// is constructed independently of its monadic/dyadic runtime semantics.
    Key,
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
            "/." => Some(Self::Key),
            "\\" => Some(Self::PrefixInfix),
            "]:" => Some(Self::Ident),
            _ => None,
        }
    }
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Insert => "/",
            Self::Key => "/.",
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

/// Recognized core function identity whose execution/construction is pending.
/// Fields are private: descriptors come only from the reviewed core catalog.
/// POS recognition does not assert rank, effects, lowering or runtime support.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VocabularyPrimitive {
    spelling: &'static str,
    part_of_speech: PrimitivePartOfSpeech,
}
impl VocabularyPrimitive {
    pub const ALL: &'static [Self] = &[
        Self {
            spelling: "!",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "!.",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "!:",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "\".",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "\":",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "#.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "#:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "$:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "$::",
            part_of_speech: PrimitivePartOfSpeech::Adverb,
        },
        Self {
            spelling: "%.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "%:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "&",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "&.",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "&.:",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "&:",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "*.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "*:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "+.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "+:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: ",.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: ",:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "-.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "-:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: ".",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "/..",
            part_of_speech: PrimitivePartOfSpeech::Adverb,
        },
        Self {
            spelling: "/:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "0:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "1:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "2:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "3:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "4:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "5:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "6:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "7:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "8:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "9:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: ":",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: ":.",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "::",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: ";",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: ";.",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: ";:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "<.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "<:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: ">.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: ">:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "?",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "?.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "@",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "@.",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "A.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "C.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "F.",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "F..",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "F.:",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "F:",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "F:.",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "F::",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "H.",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "L.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "L:",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "M.",
            part_of_speech: PrimitivePartOfSpeech::Adverb,
        },
        Self {
            spelling: "S:",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "T.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "Z:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "\\.",
            part_of_speech: PrimitivePartOfSpeech::Adverb,
        },
        Self {
            spelling: "\\:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "^",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "^.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "^:",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "_1:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "_2:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "_3:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "_4:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "_5:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "_6:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "_7:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "_8:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "_9:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "_:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "__:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "`",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "`:",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "b.",
            part_of_speech: PrimitivePartOfSpeech::Adverb,
        },
        Self {
            spelling: "c.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "f.",
            part_of_speech: PrimitivePartOfSpeech::Adverb,
        },
        Self {
            spelling: "f:",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "j.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "m.",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "o.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "p.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "p..",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "p:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "q:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "r.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "t.",
            part_of_speech: PrimitivePartOfSpeech::Conjunction,
        },
        Self {
            spelling: "u:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "x:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "{:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "{::",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "}",
            part_of_speech: PrimitivePartOfSpeech::Adverb,
        },
        Self {
            spelling: "}:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "~",
            part_of_speech: PrimitivePartOfSpeech::Adverb,
        },
        Self {
            spelling: "~.",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
        Self {
            spelling: "~:",
            part_of_speech: PrimitivePartOfSpeech::Verb,
        },
    ];
    pub fn from_spelling(spelling: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|id| id.spelling == spelling)
    }
    pub const fn spelling(self) -> &'static str {
        self.spelling
    }
    pub const fn part_of_speech(self) -> PrimitivePartOfSpeech {
        self.part_of_speech
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveSemanticId {
    Verb(PrimitiveId),
    Adverb(AdverbId),
    Conjunction(ConjunctionId),
    Vocabulary(VocabularyPrimitive),
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
            PrimitiveSemanticId::Vocabulary(id) => id.part_of_speech(),
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

    /// Admit only consistent extension identities at a compile-profile boundary.
    ///
    /// A descriptor is metadata, not proof of executable semantics. Even an
    /// admitted name stays a normal J NAME until parser-time binding lookup.
    pub fn with_extensions(
        extensions: impl IntoIterator<Item = ExtensionPrimitive>,
    ) -> Result<Self, String> {
        let mut admitted: Vec<ExtensionPrimitive> = Vec::new();
        for extension in extensions {
            let spelling = extension.spelling;
            // Consult the existing J word classifier, not a second spelling
            // grammar. Explicit locatives are not registry-owned base names.
            let words = crate::enqueuer::enqueue(spelling)
                .map_err(|_| format!("invalid extension NAME: {spelling}"))?;
            if !matches!(words.as_slice(), [word]
                if word.class == crate::enqueuer::EnqueueClass::Name
                    && word.span == (0..spelling.len())
                    && !word.flags.name_form.is_locative())
            {
                return Err(format!(
                    "extension must be a single ordinary J NAME: {spelling}"
                ));
            }
            if admitted
                .iter()
                .any(|previous| previous.spelling == spelling)
            {
                return Err(format!("duplicate extension NAME: {spelling}"));
            }
            let handle = extension.handle;
            let PrimitiveSemanticId::Extension(identity) = handle.semantic_id else {
                return Err(format!(
                    "extension has non-extension semantic identity: {spelling}"
                ));
            };
            if handle.source_origin != PrimitiveSourceOrigin::Extension
                || handle.lowering_key != LoweringKey::Extension(identity)
                || handle.semantic_info.registry_version != REGISTRY_VERSION
            {
                return Err(format!("inconsistent extension handle: {spelling}"));
            }
            admitted.push(extension);
        }
        Ok(Self {
            extensions: admitted,
        })
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
        if let Some(id) = ConjunctionId::from_spelling(spelling) {
            return Some(PrimitiveHandle::core(PrimitiveSemanticId::Conjunction(id)));
        }
        VocabularyPrimitive::from_spelling(spelling)
            .map(|id| PrimitiveHandle::core(PrimitiveSemanticId::Vocabulary(id)))
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

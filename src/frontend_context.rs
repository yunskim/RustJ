//! Immutable parser occurrences and semantic-result provenance.
//!
//! Program remains the semantic authority during storage migration. This index
//! is emitted by its parser actions, not reconstructed from spans or executed
//! array values. It is not independently executable or a binding proof.
use crate::{
    enqueuer::{EnqueueClass, EnqueueEnvironment, EnqueueFlags},
    parser::{ParseClass, ParseRow, match_parse_row},
    semantic::FunctionEntity,
};
use std::{ops::Range, sync::Arc};

macro_rules! ids {
    ($($name:ident),*) => {$(
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub usize);
    )*};
}
ids!(WordId, ItemId, NodeId, ReductionId, NameUseId);

pub const FRONTEND_CONTEXT_SCHEMA: u32 = 3;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrontendUnitId(pub u64);

#[derive(Clone, Debug)]
pub struct WordRecord {
    pub span: Range<usize>,
    pub class: EnqueueClass,
    pub flags: EnqueueFlags,
    pub environment: EnqueueEnvironment,
    /// NAME spelling survives enqueue expansion even when its mapped span is shared.
    pub name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemProducer {
    Word(WordId),
    NameUse(NameUseId),
    Reduction(ReductionId),
    FrontMark,
}

#[derive(Clone, Debug)]
pub struct ItemRecord {
    pub producer: ItemProducer,
    pub class: ParseClass,
    pub semantic: Option<NodeId>,
    pub word_range: Range<usize>,
    pub blame_word: Option<WordId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamePolicy {
    CaptureAtRead,
    CaptureAndAbandon,
    LateAtCall,
    ResolveAtConstruction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameEvidence {
    /// Unbound diagnostic parsing historically assumes noun; never a POS proof.
    DiagnosticAssumption,
    CatalogClass,
    RuntimeClass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameResolution {
    NounValue,
    FunctionValue,
    FunctionReference,
}

#[derive(Clone, Debug)]
pub struct NameUseRecord {
    pub word: WordId,
    pub input: ItemId,
    pub output: ItemId,
    pub result_class: ParseClass,
    pub policy: NamePolicy,
    pub resolution: NameResolution,
    pub evidence: NameEvidence,
    /// A catalog/runtime observation, not a complete frame/locale lookup witness.
    pub binding_version: Option<crate::semantic::NameVersion>,
    pub lookup: Option<LookupObservation>,
}

/// Unique namespace/frame instance; neither a lexical declaration nor a version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopeInstanceId(pub u64);

/// Never reused, including when a symbol is removed and recreated. Unlike a
/// per-name version, this identity cannot accidentally accept an ABA rebinding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BindingGeneration(pub u64);

impl BindingGeneration {
    pub(crate) fn fresh() -> Self {
        Self(ScopeInstanceId::fresh().0)
    }
}

impl ScopeInstanceId {
    pub(crate) fn fresh() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        // Keep unique, non-wrapping identities while supporting Rust 1.85.
        let mut id = NEXT.load(Ordering::Relaxed);
        loop {
            let next = id
                .checked_add(1)
                .expect("scope instance identity exhausted");
            match NEXT.compare_exchange_weak(id, next, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => return Self(id),
                Err(current) => id = current,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeSearch {
    /// A direct locative resolved in the selected locale's own noun table.
    DirectLocaleOnly(ScopeInstanceId),
    /// A named locale's own table missed; its default z table supplied a noun.
    /// This is an observation, not a path-epoch optimization guard.
    NamedDefaultZ {
        start: ScopeInstanceId,
        z: ScopeInstanceId,
    },
    /// Empty direct locale selects base explicitly, bypassing local search.
    BaseLocaleOnly,
    /// An explicit base locative missed base and read a noun in the default z table.
    BaseDefaultZ {
        z: ScopeInstanceId,
    },
    GlobalOnly,
    CurrentFrameThenGlobal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalLookupState {
    /// A frame exists but this locative does not search it.
    Bypassed,
    NoFrame,
    Bound,
    DeclaredUnbound,
    Absent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoundScope {
    Locale(ScopeInstanceId),
    Local(ScopeInstanceId),
    Global(ScopeInstanceId),
    Extension,
    Missing,
}

/// Actual simple-name or bounded base-noun lookup observation. Locale/path/epoch guards are not
/// supplied by this record; it must not be promoted to a full LookupWitness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookupObservation {
    pub engine: ScopeInstanceId,
    pub frame: Option<ScopeInstanceId>,
    pub search: ScopeSearch,
    pub local_state: LocalLookupState,
    pub found: FoundScope,
    pub binding_version: Option<crate::semantic::NameVersion>,
    pub binding_generation: Option<BindingGeneration>,
    pub binding_class: Option<ParseClass>,
}

/// Bounded recipe: repeat the current simple-name search, then compare frame,
/// scope, binding generation, version and POS. No locale/path assumptions are
/// implicit. A successful check is only a point-in-time observation: it is not
/// permission to hoist reads, freeze a late verb or replay effects.
#[derive(Clone, Debug)]
pub struct SimpleNameGuard {
    pub(crate) name: String,
    pub(crate) expected: LookupObservation,
    origin: (FrontendUnitId, NameUseId),
}

impl SimpleNameGuard {
    pub fn from_name_use(context: &FrontendContext, id: NameUseId) -> Result<Self, String> {
        context.verify()?;
        if !context.complete {
            return Err("incomplete parser context is not guard admission".into());
        }
        let usage = context
            .name_uses
            .get(id.0)
            .ok_or("NAME use out of bounds")?;
        let name = context.words[usage.word.0]
            .name
            .as_ref()
            .ok_or("missing NAME spelling")?;
        // Locatives and by-value forms require a different search recipe.
        if context.words[usage.word.0].flags.abandon_name
            || !name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || name.ends_with('_')
            || name.contains("__")
        {
            return Err("guard requires an ordinary simple name".into());
        }
        let expected = usage
            .lookup
            .as_ref()
            .ok_or("guard requires a runtime lookup")?;
        if usage.evidence != NameEvidence::RuntimeClass
            || !matches!(expected.found, FoundScope::Local(_) | FoundScope::Global(_))
            || expected.binding_generation.is_none()
            || expected.binding_class != Some(usage.result_class)
        {
            return Err("guard requires a bound runtime simple name".into());
        }
        Ok(Self {
            name: name.clone(),
            expected: expected.clone(),
            origin: (context.unit, id),
        })
    }

    pub fn origin(&self) -> (FrontendUnitId, NameUseId) {
        self.origin
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameGuardCheck {
    ValidAtCheck,
    EngineChanged,
    FrameChanged,
    LookupChanged,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ParseRealization {
    #[default]
    Deferred,
    Observed,
}

/// Descriptor of semantic structure, not a duplicate array payload or executor.
#[derive(Clone, Debug)]
pub enum NodeKind {
    Literal,
    ReadNoun(NameUseId),
    TakeName(NameUseId),
    Function(Arc<FunctionEntity>),
    Monad {
        function: NodeId,
        argument: NodeId,
    },
    Dyad {
        function: NodeId,
        left: NodeId,
        right: NodeId,
    },
    Construct {
        row: ParseRow,
        inputs: Vec<NodeId>,
        function: Option<Arc<FunctionEntity>>,
    },
    WriteName {
        target: ItemId,
        copula: WordId,
        value: NodeId,
    },
}

#[derive(Clone, Debug)]
pub struct NodeRecord {
    pub kind: NodeKind,
    pub class: ParseClass,
}

#[derive(Clone, Debug)]
pub struct NodeOrigin {
    pub node: NodeId,
    pub items: Vec<ItemId>,
    pub reductions: Vec<ReductionId>,
    pub name_uses: Vec<NameUseId>,
}

#[derive(Clone, Debug)]
pub struct ReductionRecord {
    pub row: ParseRow,
    pub window: [Option<ItemId>; 4],
    pub consumed: Vec<ItemId>,
    pub produced: ItemId,
}

/// Logical order, not a machine schedule. Failed attempts have no fake output.
#[derive(Clone, Debug)]
pub enum ParseStep {
    Stack { queued: ItemId, resolved: ItemId },
    FrontMark(ItemId),
    Reduce(ReductionId),
}

#[derive(Clone, Debug)]
pub struct PendingAction {
    pub row: ParseRow,
    pub window: [Option<ItemId>; 4],
}

#[derive(Clone, Debug)]
pub struct FrontendContext {
    pub schema: u32,
    pub unit: FrontendUnitId,
    pub realization: ParseRealization,
    pub source: Arc<str>,
    pub source_origin: Option<crate::source::SourceOrigin>,
    pub words: Vec<WordRecord>,
    pub items: Vec<ItemRecord>,
    pub nodes: Vec<NodeRecord>,
    pub origins: Vec<NodeOrigin>,
    pub name_uses: Vec<NameUseRecord>,
    pub reductions: Vec<ReductionRecord>,
    pub steps: Vec<ParseStep>,
    pub pending: Option<PendingAction>,
    pub root: Option<ItemId>,
    pub complete: bool,
}

impl Default for FrontendContext {
    fn default() -> Self {
        Self {
            schema: FRONTEND_CONTEXT_SCHEMA,
            unit: FrontendUnitId(ScopeInstanceId::fresh().0),
            realization: ParseRealization::Deferred,
            source: Arc::from(""),
            source_origin: None,
            words: Vec::new(),
            items: Vec::new(),
            nodes: Vec::new(),
            origins: Vec::new(),
            name_uses: Vec::new(),
            reductions: Vec::new(),
            steps: Vec::new(),
            pending: None,
            root: None,
            complete: false,
        }
    }
}

/// A failed parse retains its successfully emitted prefix and pending window.
#[derive(Debug)]
pub struct FrontendFailure {
    pub error: crate::Error,
    pub context: Box<FrontendContext>,
}

/// A3 operations already retain their J Graph origin. This closes that chain
/// back to parser occurrences without introducing a second executable IR.
#[derive(Clone, Debug)]
pub struct ParserProvenance {
    pub context: Arc<FrontendContext>,
    pub graph_nodes: Vec<Vec<NodeId>>,
}

pub(crate) fn action_slots(row: ParseRow, classes: [ParseClass; 4]) -> (usize, usize) {
    match row {
        ParseRow::MonadEdge | ParseRow::Adverb => (1, 2),
        ParseRow::MonadVVN => (2, 2),
        ParseRow::DyadNVN | ParseRow::Conjunction | ParseRow::Fork => (1, 3),
        ParseRow::Hook => (
            1,
            if matches!(
                classes[3],
                ParseClass::Noun | ParseClass::Verb | ParseClass::Adverb | ParseClass::Conjunction
            ) {
                3
            } else {
                2
            },
        ),
        ParseRow::Assignment | ParseRow::Parenthesis => (0, 3),
    }
}

impl FrontendContext {
    pub(crate) fn node(&mut self, kind: NodeKind, class: ParseClass) -> NodeId {
        let node = NodeId(self.nodes.len());
        self.nodes.push(NodeRecord { kind, class });
        self.origins.push(NodeOrigin {
            node,
            items: Vec::new(),
            reductions: Vec::new(),
            name_uses: Vec::new(),
        });
        node
    }

    pub(crate) fn item(&mut self, record: ItemRecord) -> ItemId {
        let id = ItemId(self.items.len());
        if let Some(node) = record.semantic {
            self.origins[node.0].items.push(id);
            match record.producer {
                ItemProducer::Reduction(reduction) => {
                    self.origins[node.0].reductions.push(reduction)
                }
                ItemProducer::NameUse(name_use) => self.origins[node.0].name_uses.push(name_use),
                _ => {}
            }
        }
        self.items.push(record);
        id
    }

    fn classes(&self, window: [Option<ItemId>; 4]) -> Result<[ParseClass; 4], String> {
        let mut classes = [ParseClass::Mark; 4];
        for (i, item) in window.iter().enumerate() {
            if let Some(item) = item {
                classes[i] = self
                    .items
                    .get(item.0)
                    .ok_or("window item out of bounds")?
                    .class;
            }
        }
        Ok(classes)
    }

    /// Verify IDs, inverse links, row choice and actual queue/stack transitions.
    /// This does not prove constructor semantics or authorize NAME specialization.
    pub fn verify(&self) -> Result<(), String> {
        let fail = |message: &str| Err(message.to_owned());
        if self.source_origin.as_ref().is_some_and(|origin| {
            origin.text() != self.source.as_ref()
                || origin.root_span(0..self.source.len()).is_none()
        }) {
            return fail("frontend source provenance mismatch");
        }
        if self.schema != FRONTEND_CONTEXT_SCHEMA || self.unit.0 == 0 {
            return fail("unsupported frontend context schema/unit");
        }
        for word in &self.words {
            if self.source.get(word.span.clone()).is_none() {
                return fail("invalid source span");
            }
        }
        if self.nodes.len() != self.origins.len() {
            return fail("missing node origins");
        }
        for (i, item) in self.items.iter().enumerate() {
            if item.word_range.start > item.word_range.end || item.word_range.end > self.words.len()
            {
                return fail("invalid item word coverage");
            }
            if let Some(word) = item.blame_word {
                if !item.word_range.contains(&word.0) {
                    return fail("blame outside item coverage");
                }
            }
            if let Some(node) = item.semantic {
                let record = self
                    .nodes
                    .get(node.0)
                    .ok_or("semantic node out of bounds")?;
                if record.class != item.class {
                    return fail("item/node POS mismatch");
                }
                if !self.origins[node.0].items.contains(&ItemId(i)) {
                    return fail("missing inverse item origin");
                }
                match item.producer {
                    ItemProducer::Reduction(id)
                        if !self.origins[node.0].reductions.contains(&id) =>
                    {
                        return fail("missing reduction origin");
                    }
                    ItemProducer::NameUse(id) if !self.origins[node.0].name_uses.contains(&id) => {
                        return fail("missing NAME origin");
                    }
                    _ => {}
                }
            } else if matches!(
                item.class,
                ParseClass::Noun | ParseClass::Verb | ParseClass::Adverb | ParseClass::Conjunction
            ) {
                return fail("semantic item has no result node");
            }
            let valid = match item.producer {
                ItemProducer::Word(word) => {
                    self.words.get(word.0).is_some() && item.word_range == (word.0..word.0 + 1)
                }
                ItemProducer::NameUse(id) => self
                    .name_uses
                    .get(id.0)
                    .is_some_and(|n| n.output == ItemId(i)),
                ItemProducer::Reduction(id) => self
                    .reductions
                    .get(id.0)
                    .is_some_and(|r| r.produced == ItemId(i)),
                ItemProducer::FrontMark => {
                    item.class == ParseClass::Mark && item.semantic.is_none()
                }
            };
            if !valid {
                return fail("invalid producer/output link");
            }
        }
        for (i, origin) in self.origins.iter().enumerate() {
            if origin.node != NodeId(i) || origin.items.is_empty() {
                return fail("invalid node origin");
            }
            for item in &origin.items {
                if self.items.get(item.0).and_then(|item| item.semantic) != Some(NodeId(i)) {
                    return fail("invalid origin/item link");
                }
            }
            for reduction in &origin.reductions {
                let record = self
                    .reductions
                    .get(reduction.0)
                    .ok_or("origin reduction out of bounds")?;
                if self
                    .items
                    .get(record.produced.0)
                    .and_then(|item| item.semantic)
                    != Some(NodeId(i))
                {
                    return fail("invalid reduction origin");
                }
            }
            for name_use in &origin.name_uses {
                let record = self
                    .name_uses
                    .get(name_use.0)
                    .ok_or("origin name use out of bounds")?;
                if self
                    .items
                    .get(record.output.0)
                    .and_then(|item| item.semantic)
                    != Some(NodeId(i))
                {
                    return fail("invalid NAME origin");
                }
            }
            let mut children = Vec::new();
            match &self.nodes[i].kind {
                NodeKind::Monad { function, argument } => {
                    children.extend([*function, *argument]);
                    if self.nodes.get(function.0).map(|n| n.class) != Some(ParseClass::Verb)
                        || self.nodes.get(argument.0).map(|n| n.class) != Some(ParseClass::Noun)
                    {
                        return fail("invalid monad edges");
                    }
                }
                NodeKind::Dyad {
                    function,
                    left,
                    right,
                } => {
                    children.extend([*function, *left, *right]);
                    if self.nodes.get(function.0).map(|n| n.class) != Some(ParseClass::Verb)
                        || [left, right]
                            .iter()
                            .any(|n| self.nodes.get(n.0).map(|n| n.class) != Some(ParseClass::Noun))
                    {
                        return fail("invalid dyad edges");
                    }
                }
                NodeKind::Construct { inputs, .. } => children.extend(inputs),
                NodeKind::WriteName {
                    target,
                    copula,
                    value,
                } => {
                    children.push(*value);
                    let target = self
                        .items
                        .get(target.0)
                        .ok_or("missing assignment target")?;
                    if target.class == ParseClass::Noun {
                        children.push(target.semantic.ok_or("noun target without semantic node")?);
                    }
                    if !matches!(target.class, ParseClass::Name | ParseClass::Noun)
                        || self.words.get(copula.0).map(|word| word.class)
                            != Some(EnqueueClass::Assignment)
                    {
                        return fail("invalid assignment target/copula");
                    }
                }
                NodeKind::ReadNoun(name_use) | NodeKind::TakeName(name_use) => {
                    if self.name_uses.get(name_use.0).map(|n| n.result_class)
                        != Some(ParseClass::Noun)
                    {
                        return fail("invalid noun NAME use");
                    }
                }
                NodeKind::Function(function) => {
                    if ParseClass::from(function.result_pos) != self.nodes[i].class {
                        return fail("function POS mismatch");
                    }
                }
                NodeKind::Literal => {}
            }
            if children.iter().any(|node| node.0 >= i) {
                return fail("noncausal semantic edge");
            }
        }
        for (i, name_use) in self.name_uses.iter().enumerate() {
            let input = self
                .items
                .get(name_use.input.0)
                .ok_or("NAME input out of bounds")?;
            let output = self
                .items
                .get(name_use.output.0)
                .ok_or("NAME output out of bounds")?;
            if input.class != ParseClass::Name
                || name_use.input.0 >= name_use.output.0
                || output.producer != ItemProducer::NameUse(NameUseId(i))
                || output.class != name_use.result_class
                || output.word_range != input.word_range
                || input.producer != ItemProducer::Word(name_use.word)
            {
                return fail("invalid NAME substitution");
            }
            if !self
                .words
                .get(name_use.word.0)
                .is_some_and(|word| word.class == EnqueueClass::Name && word.flags.lookup_name)
            {
                return fail("assignment target incorrectly recorded as a NAME read");
            }
            let kind = &self.nodes[output
                .semantic
                .ok_or("NAME output without semantic node")?
                .0]
                .kind;
            let policy = match (name_use.resolution, kind) {
                (NameResolution::NounValue, NodeKind::TakeName(id)) if *id == NameUseId(i) => {
                    NamePolicy::CaptureAndAbandon
                }
                (NameResolution::NounValue, NodeKind::ReadNoun(id)) if *id == NameUseId(i) => {
                    NamePolicy::CaptureAtRead
                }
                (NameResolution::FunctionValue, NodeKind::Function(function)) => {
                    if let crate::semantic::FunctionHead::TakeName { name, single_word } =
                        &function.head
                        && (self.realization != ParseRealization::Deferred
                            || !self.words[name_use.word.0].flags.abandon_name
                            || self.words[name_use.word.0].name.as_ref() != Some(name)
                            || *single_word != (self.words.len() == 1))
                    {
                        return fail("invalid deferred function abandon context");
                    }
                    if self.words[name_use.word.0].flags.abandon_name {
                        NamePolicy::CaptureAndAbandon
                    } else {
                        NamePolicy::CaptureAtRead
                    }
                }
                (NameResolution::FunctionReference, NodeKind::Function(function)) => {
                    match (&function.head, function.result_pos) {
                        (
                            crate::semantic::FunctionHead::NameRef(_),
                            crate::semantic::FunctionPartOfSpeech::Verb,
                        ) => NamePolicy::LateAtCall,
                        (crate::semantic::FunctionHead::NameRef(_), _) => {
                            NamePolicy::ResolveAtConstruction
                        }
                        _ => return fail("NAME reference without a NameRef entity"),
                    }
                }
                _ => return fail("invalid NAME result kind"),
            };
            if policy != name_use.policy {
                return fail("invalid NAME timing policy");
            }
            if (policy == NamePolicy::CaptureAndAbandon)
                != self.words[name_use.word.0].flags.abandon_name
            {
                return fail("abandon flag/policy mismatch");
            }
            if let Some(lookup) = &name_use.lookup {
                if name_use.evidence != NameEvidence::RuntimeClass
                    || lookup.binding_version != name_use.binding_version
                {
                    return fail("invalid runtime NAME observation");
                }
                if matches!(
                    lookup.search,
                    ScopeSearch::GlobalOnly | ScopeSearch::CurrentFrameThenGlobal
                ) && self.words[name_use.word.0].flags.name_form
                    != crate::enqueuer::NameForm::Simple
                {
                    return fail("simple NAME search requires a simple lexical name");
                }
                let frame_valid = match (
                    lookup.frame,
                    lookup.search,
                    lookup.local_state,
                    lookup.found,
                ) {
                    (frame, ScopeSearch::DirectLocaleOnly(start), state, found) => {
                        self.words[name_use.word.0].flags.name_form
                            == crate::enqueuer::NameForm::DirectLocative
                            && lookup.binding_class == Some(ParseClass::Noun)
                            && self.words[name_use.word.0]
                                .name
                                .as_deref()
                                .is_some_and(|name| {
                                    // Only the literal base alias selects the engine table.
                                    // Distinct named locales cannot forge the base resource ID.
                                    name.ends_with("_base_") == (start == lookup.engine)
                                })
                            && match found {
                                FoundScope::Locale(hit) => hit == start && hit != lookup.engine,
                                FoundScope::Global(hit) => hit == start && hit == lookup.engine,
                                _ => false,
                            }
                            && match frame {
                                None => state == LocalLookupState::NoFrame,
                                Some(frame) => {
                                    frame != start
                                        && frame != lookup.engine
                                        && state == LocalLookupState::Bypassed
                                }
                            }
                    }
                    (
                        frame,
                        ScopeSearch::NamedDefaultZ { start, z },
                        state,
                        FoundScope::Locale(hit),
                    ) => {
                        self.words[name_use.word.0].flags.name_form
                            == crate::enqueuer::NameForm::DirectLocative
                            && lookup.binding_class == Some(ParseClass::Noun)
                            && self.words[name_use.word.0]
                                .name
                                .as_deref()
                                .is_some_and(|name| {
                                    !name.ends_with("_base_") && !name.ends_with("_z_")
                                })
                            && start != lookup.engine
                            && z != lookup.engine
                            && start != z
                            && hit == z
                            && match frame {
                                None => state == LocalLookupState::NoFrame,
                                Some(frame) => {
                                    frame != start
                                        && frame != z
                                        && frame != lookup.engine
                                        && state == LocalLookupState::Bypassed
                                }
                            }
                    }
                    (frame, ScopeSearch::BaseDefaultZ { z }, state, FoundScope::Locale(hit)) => {
                        let word = &self.words[name_use.word.0];
                        (word.flags.name_form == crate::enqueuer::NameForm::BaseLocative
                            || (word.flags.name_form == crate::enqueuer::NameForm::DirectLocative
                                && word
                                    .name
                                    .as_deref()
                                    .is_some_and(|name| name.ends_with("_base_"))))
                            && lookup.binding_class == Some(ParseClass::Noun)
                            && z != lookup.engine
                            && hit == z
                            && match frame {
                                None => state == LocalLookupState::NoFrame,
                                Some(frame) => {
                                    frame != z
                                        && frame != lookup.engine
                                        && state == LocalLookupState::Bypassed
                                }
                            }
                    }
                    (frame, ScopeSearch::BaseLocaleOnly, state, FoundScope::Global(_)) => {
                        self.words[name_use.word.0].flags.name_form
                            == crate::enqueuer::NameForm::BaseLocative
                            && lookup.binding_class == Some(ParseClass::Noun)
                            && match frame {
                                None => state == LocalLookupState::NoFrame,
                                Some(frame) => {
                                    frame != lookup.engine && state == LocalLookupState::Bypassed
                                }
                            }
                    }
                    (
                        None,
                        ScopeSearch::GlobalOnly,
                        LocalLookupState::NoFrame,
                        FoundScope::Global(_) | FoundScope::Extension | FoundScope::Missing,
                    ) => true,
                    (
                        Some(frame),
                        ScopeSearch::CurrentFrameThenGlobal,
                        LocalLookupState::Bound,
                        FoundScope::Local(found),
                    ) => frame == found && frame != lookup.engine,
                    (
                        Some(frame),
                        ScopeSearch::CurrentFrameThenGlobal,
                        LocalLookupState::DeclaredUnbound | LocalLookupState::Absent,
                        FoundScope::Global(_) | FoundScope::Extension | FoundScope::Missing,
                    ) => frame != lookup.engine,
                    _ => false,
                };
                if !frame_valid
                    || matches!(lookup.found, FoundScope::Global(scope) if scope != lookup.engine)
                    || (matches!(
                        lookup.found,
                        FoundScope::Local(_) | FoundScope::Global(_) | FoundScope::Locale(_)
                    ) != lookup.binding_version.is_some())
                    || lookup.binding_version.is_some() != lookup.binding_generation.is_some()
                    || lookup.binding_version.is_some() != lookup.binding_class.is_some()
                    || lookup
                        .binding_class
                        .is_some_and(|class| class != name_use.result_class)
                {
                    return fail("inconsistent local/global NAME lookup");
                }
            }
        }
        let mut stack = Vec::new();
        let mut next_word = self.words.len();
        let mut next_reduction = 0;
        for step in &self.steps {
            match *step {
                ParseStep::Stack { queued, resolved } => {
                    next_word = next_word.checked_sub(1).ok_or("extra stack entry")?;
                    if self.items.get(queued.0).map(|item| item.producer)
                        != Some(ItemProducer::Word(WordId(next_word)))
                    {
                        return fail("queue order mismatch");
                    }
                    if queued != resolved
                        && !self
                            .name_uses
                            .iter()
                            .any(|n| n.input == queued && n.output == resolved)
                    {
                        return fail("unrecorded stack substitution");
                    }
                    if self.items.get(resolved.0).is_none() {
                        return fail("stack item out of bounds");
                    }
                    stack.insert(0, resolved);
                }
                ParseStep::FrontMark(item) => {
                    if next_word != 0
                        || self.items.get(item.0).map(|i| i.producer)
                            != Some(ItemProducer::FrontMark)
                    {
                        return fail("invalid FRONT MARK timing");
                    }
                    stack.insert(0, item);
                }
                ParseStep::Reduce(id) => {
                    if id.0 != next_reduction {
                        return fail("reduction order mismatch");
                    }
                    next_reduction += 1;
                    let reduction = self.reductions.get(id.0).ok_or("reduction out of bounds")?;
                    let window = std::array::from_fn(|i| stack.get(i).copied());
                    if reduction.window != window {
                        return fail("reduction window mismatch");
                    }
                    let classes = self.classes(window)?;
                    if match_parse_row(classes) != Some(reduction.row) {
                        return fail("incorrect parsing row");
                    }
                    let (start, count) = action_slots(reduction.row, classes);
                    let consumed = stack
                        .get(start..start + count)
                        .ok_or("short reduction stack")?;
                    if consumed != reduction.consumed {
                        return fail("incorrect consumed items");
                    }
                    let output = self
                        .items
                        .get(reduction.produced.0)
                        .ok_or("reduction output out of bounds")?;
                    let first = &self.items[consumed[0].0];
                    let last = &self.items[consumed[count - 1].0];
                    if output.word_range != (first.word_range.start..last.word_range.end)
                        || consumed.iter().any(|item| item.0 >= reduction.produced.0)
                    {
                        return fail("invalid result coverage/causality");
                    }
                    let result = output.semantic.ok_or("reduction without semantic result")?;
                    let semantic = &self.nodes[result.0].kind;
                    let node = |index: usize| self.items[consumed[index].0].semantic;
                    let correct = match (reduction.row, semantic) {
                        (ParseRow::Parenthesis, _) => Some(result) == node(1),
                        (
                            ParseRow::MonadEdge | ParseRow::MonadVVN,
                            NodeKind::Monad { function, argument },
                        ) => Some(*function) == node(0) && Some(*argument) == node(1),
                        (
                            ParseRow::DyadNVN,
                            NodeKind::Dyad {
                                function,
                                left,
                                right,
                            },
                        ) => {
                            Some(*function) == node(1)
                                && Some(*left) == node(0)
                                && Some(*right) == node(2)
                        }
                        (
                            ParseRow::Assignment,
                            NodeKind::WriteName {
                                target,
                                copula,
                                value,
                            },
                        ) => {
                            *target == consumed[0]
                                && copula.0 == self.items[consumed[1].0].word_range.start
                                && Some(*value) == node(2)
                        }
                        (
                            row @ (ParseRow::Adverb
                            | ParseRow::Conjunction
                            | ParseRow::Fork
                            | ParseRow::Hook),
                            NodeKind::Construct {
                                row: constructor_row,
                                inputs,
                                function,
                            },
                        ) => {
                            row == *constructor_row
                                && *inputs
                                    == consumed
                                        .iter()
                                        .filter_map(|item| self.items[item.0].semantic)
                                        .collect::<Vec<_>>()
                                && function
                                    .as_ref()
                                    .is_none_or(|f| ParseClass::from(f.result_pos) == output.class)
                        }
                        _ => false,
                    };
                    if !correct {
                        return fail("semantic result does not match parser operands");
                    }
                    stack.splice(start..start + count, [reduction.produced]);
                }
            }
        }
        if next_reduction != self.reductions.len() {
            return fail("unvisited reduction");
        }
        if let Some(pending) = &self.pending {
            if pending.window != std::array::from_fn(|i| stack.get(i).copied())
                || match_parse_row(self.classes(pending.window)?) != Some(pending.row)
            {
                return fail("invalid pending action");
            }
        }
        if self.complete {
            if next_word != 0 || self.pending.is_some() {
                return fail("incomplete parser marked complete");
            }
            if stack
                .first()
                .is_some_and(|id| self.items[id.0].class == ParseClass::Mark)
            {
                stack.remove(0);
            }
            match (self.root, stack.as_slice()) {
                (None, []) => {}
                (Some(root), [item]) if root == *item => {}
                _ => return fail("invalid completed root"),
            }
        }
        Ok(())
    }
}

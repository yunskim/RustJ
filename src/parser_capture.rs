//! Optional parser execution provenance. No input/intermediate array snapshots.
//! Function entities still own their intrinsic noun operands, via shared identity.
//! This is an observation log, not a second canonical IR or an executable plan.
use crate::{
    error::ErrorContext,
    j_graph_ir::GraphFacts,
    parser::ParseRow,
    semantic::{FunctionEntity, NameVersion},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OccurrenceId(pub usize);

#[derive(Clone, Debug)]
pub enum CaptureEvent {
    /// Pre-action lookup and whether C's context actually deletes this binding.
    Abandon {
        name: String,
        lookup: crate::frontend_context::LookupObservation,
        deleted: bool,
        span: Range<usize>,
    },
    FunctionNameRank {
        snapshot: crate::semantic::NameRankSnapshot,
    },
    /// Direct constructor-time lookup; does not freeze ordinary namerefs.
    ForkNameResolved {
        row: ParseRow,
        read: crate::semantic::NameUse,
        capped: bool,
    },
    ModifierStacked {
        snapshot: crate::semantic::ModifierSnapshot,
    },
    Input {
        id: OccurrenceId,
        name: Option<String>,
        version: Option<NameVersion>,
        span: Range<usize>,
        facts: GraphFacts,
        /// Expanded enqueue index; several generated words may share a DD span.
        word_index: usize,
    },
    ApplyAttempt {
        id: OccurrenceId,
        function: Arc<FunctionEntity>,
        left: Option<OccurrenceId>,
        right: OccurrenceId,
        span: Range<usize>,
        word_index: usize,
    },
    ApplySuccess {
        id: OccurrenceId,
        facts: GraphFacts,
    },
    ApplyFailure {
        id: OccurrenceId,
        kind: String,
        context: Option<ErrorContext>,
    },
    ConstructionAttempt {
        row: ParseRow,
        noun_inputs: Vec<OccurrenceId>,
        span: Range<usize>,
    },
    ModifierResolved {
        binding: ModifierBinding,
    },
    GerundNameResolved {
        row: ParseRow,
        read: GerundNameRead,
    },
    ConstructorApply {
        row: ParseRow,
        call: ConstructorCall,
    },
    /// Execution boundary: body dependencies are not a flattened outer graph.
    ExplicitModifierApply {
        row: ParseRow,
        function: Arc<FunctionEntity>,
        span: Range<usize>,
    },
    ConstructionNounSuccess {
        row: ParseRow,
        id: OccurrenceId,
        /// Existing noun occurrence transported by a selector, never a body result.
        selected_input: Option<OccurrenceId>,
        facts: GraphFacts,
        span: Range<usize>,
    },
    ConstructionSuccess {
        row: ParseRow,
        function: Arc<FunctionEntity>,
        span: Range<usize>,
    },
    ConstructionFailure {
        row: ParseRow,
        kind: String,
        span: Range<usize>,
    },
    FunctionResult {
        function: Arc<FunctionEntity>,
        span: Range<usize>,
    },
    Commit {
        name: String,
        version: NameVersion,
        previous: Option<NameVersion>,
        span: Range<usize>,
        value: Option<OccurrenceId>,
        class: crate::parser::ParseClass,
        function: Option<Arc<FunctionEntity>>,
        source: crate::parser::AssignmentSource,
        final_assignment: bool,
    },
}

#[derive(Clone, Debug)]
pub struct ModifierBinding {
    pub name: String,
    pub version: NameVersion,
    pub expected: crate::semantic::FunctionPartOfSpeech,
    pub row: ParseRow,
    pub function: Arc<FunctionEntity>,
    pub span: Range<usize>,
}

/// One completed runtime call required by a constructor, not a replayable plan.
#[derive(Clone, Debug)]
pub struct ConstructorCall {
    pub function: Arc<FunctionEntity>,
    pub left: Option<GraphFacts>,
    pub right: GraphFacts,
    pub span: Range<usize>,
    pub outcome: ConstructorCallOutcome,
}
#[derive(Clone, Debug)]
pub enum ConstructorCallOutcome {
    Success(GraphFacts),
    Failure {
        kind: String,
        context: Option<ErrorContext>,
    },
}

/// Constructor-time observation, not a cache guard or a retained noun payload.
#[derive(Clone, Debug)]
pub struct GerundNameRead {
    pub name: String,
    pub version: Option<NameVersion>,
    pub class: crate::parser::ParseClass,
    pub facts: Option<GraphFacts>,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct ParseCapture {
    source: String,
    /// Same parser-owned context as Program, including the completed prefix on
    /// failure. Not reconstructed from events and not an executable continuation.
    pub frontend: Option<Arc<crate::frontend_context::FrontendContext>>,
    pub events: Vec<CaptureEvent>,
    pub result: Option<OccurrenceId>,
    /// Terminal enqueue/parse/runtime failure, including errors with no apply.
    pub failure: Option<CaptureFailure>,
    next_id: usize,
}

#[derive(Clone, Debug)]
pub struct CaptureFailure {
    pub kind: String,
    pub context: Option<ErrorContext>,
}

impl ParseCapture {
    pub fn source(&self) -> &str {
        &self.source
    }
    pub(crate) fn set_source(&mut self, source: &str) {
        self.source = source.to_owned();
    }

    pub(crate) fn next(&mut self) -> OccurrenceId {
        let id = OccurrenceId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Existing J Graph has one pending outer write, not ordered runtime effects.
    pub fn requires_ordered_effect_graph(&self) -> bool {
        self.events.iter().any(|event| {
            if matches!(event, CaptureEvent::Abandon { .. }) {
                return true;
            }
            matches!(
                event,
                CaptureEvent::Commit {
                    final_assignment: false,
                    ..
                }
            ) || matches!(event, CaptureEvent::Commit { source, .. } if source.selection.is_some())
        })
    }

    /// Check associations and attempt/outcome ordering, including partial failure.
    pub fn verify(&self) -> std::result::Result<(), &'static str> {
        let mut ready = BTreeSet::new();
        let mut attempts = BTreeMap::new();
        let mut next = 0;
        let mut construction = None;
        let mut construction_inputs = Vec::new();
        for (event_index, event) in self.events.iter().enumerate() {
            match event {
                CaptureEvent::Abandon {
                    name,
                    lookup,
                    span,
                    deleted,
                    ..
                } => {
                    if name.is_empty()
                        || self.source.get(span.clone()).is_none()
                        || lookup.binding_version.is_none()
                        || lookup.binding_generation.is_none()
                        || !(matches!(
                            lookup.found,
                            crate::frontend_context::FoundScope::Local(_)
                                | crate::frontend_context::FoundScope::Global(_)
                        ) || matches!(
                            (lookup.search, lookup.found),
                            (crate::frontend_context::ScopeSearch::SimpleDefaultZ { z },
                             crate::frontend_context::FoundScope::Locale(hit))
                                if z == hit && z != lookup.engine
                                    && *deleted
                                    && lookup.binding_class == Some(crate::parser::ParseClass::Noun)
                                    && match lookup.frame {
                                        None => lookup.local_state == crate::frontend_context::LocalLookupState::NoFrame,
                                        Some(frame) => frame != z && frame != lookup.engine
                                            && matches!(lookup.local_state,
                                                crate::frontend_context::LocalLookupState::Absent
                                                | crate::frontend_context::LocalLookupState::DeclaredUnbound),
                                    }
                        ))
                    {
                        return Err("invalid abandon observation");
                    }
                }
                CaptureEvent::FunctionNameRank { snapshot } => {
                    if !attempts.is_empty()
                        || snapshot.name.is_empty()
                        || snapshot.version.is_some_and(|v| v.0 == 0)
                        || snapshot
                            .ranks
                            .is_some_and(|ranks| ranks.iter().any(|r| !(0..=63).contains(r)))
                        || self.source.get(snapshot.span.clone()).is_none()
                    {
                        return Err("invalid stacked function rank snapshot");
                    }
                }
                CaptureEvent::ForkNameResolved { row, read, .. } => {
                    if construction != Some(*row)
                        || !matches!(
                            row,
                            ParseRow::Fork | ParseRow::Adverb | ParseRow::Conjunction
                        )
                        || !attempts.is_empty()
                        || read.name.is_empty()
                        || read.version.0 == 0
                        || read.span.start >= read.span.end
                        || self.source.get(read.span.clone()).is_none()
                    {
                        return Err("invalid fork construction binding witness");
                    }
                }
                CaptureEvent::ModifierStacked { snapshot } => {
                    if !attempts.is_empty()
                        || construction.is_some()
                        || snapshot.version.0 == 0
                        || snapshot.name.is_empty()
                        || snapshot.function.result_pos != snapshot.expected
                        || !snapshot.function.is_nameless_modifier()
                        || self.source.get(snapshot.span.clone()).is_none()
                    {
                        return Err("invalid stacked modifier snapshot");
                    }
                }
                CaptureEvent::Input { id, .. } => {
                    if id.0 != next || !attempts.is_empty() || construction.is_some() {
                        return Err("input occurrence is not sequential");
                    }
                    next += 1;
                    ready.insert(*id);
                }
                CaptureEvent::ApplyAttempt {
                    id, left, right, ..
                } => {
                    if id.0 != next || !attempts.is_empty() || construction.is_some() {
                        return Err("invalid apply attempt order");
                    }
                    if !ready.contains(right) || left.is_some_and(|id| !ready.contains(&id)) {
                        return Err("apply input is unavailable");
                    }
                    next += 1;
                    attempts.insert(*id, ());
                }
                CaptureEvent::ApplySuccess { id, .. } => {
                    if attempts.remove(id).is_none() {
                        return Err("success without attempt");
                    }
                    ready.insert(*id);
                }
                CaptureEvent::ApplyFailure { id, .. } => {
                    if attempts.remove(id).is_none() {
                        return Err("failure without attempt");
                    }
                }
                CaptureEvent::ConstructionAttempt {
                    row, noun_inputs, ..
                } => {
                    if construction.replace(*row).is_some() || !attempts.is_empty() {
                        return Err("invalid construction attempt order");
                    }
                    construction_inputs.clone_from(noun_inputs);
                    if noun_inputs.iter().any(|id| !ready.contains(id)) {
                        return Err("constructor input is unavailable");
                    }
                }
                CaptureEvent::ModifierResolved { binding } => {
                    let valid_phase = match binding.row {
                        ParseRow::Assignment => construction.is_none() && attempts.is_empty(),
                        ParseRow::Adverb | ParseRow::Conjunction => {
                            construction == Some(binding.row)
                        }
                        _ => false,
                    };
                    if !valid_phase || binding.function.result_pos != binding.expected {
                        return Err("invalid modifier resolution witness");
                    }
                }
                CaptureEvent::GerundNameResolved { row, read } => {
                    if construction != Some(*row)
                        || !matches!(row, ParseRow::Adverb | ParseRow::Conjunction)
                        || !matches!(
                            read.class,
                            crate::parser::ParseClass::Noun
                                | crate::parser::ParseClass::Verb
                                | crate::parser::ParseClass::Adverb
                                | crate::parser::ParseClass::Conjunction
                        )
                        || (read.class == crate::parser::ParseClass::Noun) != read.facts.is_some()
                        || (read.class == crate::parser::ParseClass::Noun && read.version.is_none())
                        || read.name.is_empty()
                        || read.span.start >= read.span.end
                        || self.source.get(read.span.clone()).is_none()
                        || read.version.is_some_and(|v| v.0 == 0)
                    {
                        return Err("invalid gerund name observation");
                    }
                }
                CaptureEvent::ConstructorApply { row, call } => {
                    if construction != Some(*row)
                        || !matches!(row, ParseRow::Adverb | ParseRow::Conjunction)
                        || call.function.result_pos != crate::semantic::FunctionPartOfSpeech::Verb
                        || call.span.start >= call.span.end
                        || self.source.get(call.span.clone()).is_none()
                    {
                        return Err("invalid constructor call observation");
                    }
                }
                CaptureEvent::ExplicitModifierApply {
                    row,
                    function,
                    span,
                } => {
                    if construction != Some(*row)
                        || !matches!(row, ParseRow::Adverb | ParseRow::Conjunction)
                        || !matches!(
                            function.head,
                            crate::semantic::FunctionHead::ExplicitDefinition(_)
                        )
                        || function.result_pos
                            != if *row == ParseRow::Adverb {
                                crate::semantic::FunctionPartOfSpeech::Adverb
                            } else {
                                crate::semantic::FunctionPartOfSpeech::Conjunction
                            }
                        || span.start >= span.end
                        || self.source.get(span.clone()).is_none()
                    {
                        return Err("invalid explicit modifier invocation");
                    }
                }
                CaptureEvent::ConstructionNounSuccess {
                    row,
                    id,
                    selected_input,
                    span,
                    ..
                } => {
                    if construction.take() != Some(*row)
                        || !matches!(row, ParseRow::Adverb | ParseRow::Conjunction)
                        || selected_input.is_some_and(|input| !construction_inputs.contains(&input))
                        || id.0 != next
                        || !ready.insert(*id)
                        || span.start >= span.end
                        || self.source.get(span.clone()).is_none()
                    {
                        return Err("invalid noun construction outcome");
                    }
                    next += 1;
                }
                CaptureEvent::ConstructionSuccess { row, .. }
                | CaptureEvent::ConstructionFailure { row, .. } => {
                    if construction.take() != Some(*row) {
                        return Err("construction outcome without matching attempt");
                    }
                }
                CaptureEvent::Commit {
                    value,
                    class,
                    function,
                    version,
                    previous,
                    source,
                    final_assignment,
                    ..
                } => {
                    let valid_value = match class {
                        crate::parser::ParseClass::Noun => value.is_some() && function.is_none(),
                        crate::parser::ParseClass::Verb
                        | crate::parser::ParseClass::Adverb
                        | crate::parser::ParseClass::Conjunction => {
                            value.is_none()
                                && function.as_ref().is_some_and(|f| {
                                    crate::parser::ParseClass::from(f.result_pos) == *class
                                })
                        }
                        _ => false,
                    };
                    if !valid_value {
                        return Err("commit result POS does not match value");
                    }
                    if source.selection.is_some() && *class != crate::parser::ParseClass::Noun {
                        return Err("item assignment requires a noun RHS");
                    }
                    if previous.map_or(Some(1), |v| v.0.checked_add(1)) != Some(version.0) {
                        return Err("invalid commit binding version");
                    }
                    if source.flags.global_assignment == source.flags.local_assignment {
                        return Err("invalid commit copula scope");
                    }
                    if *final_assignment && event_index + 1 != self.events.len() {
                        return Err("events after final commit");
                    }
                    if value.is_some_and(|id| !ready.contains(&id)) {
                        return Err("commit value is unavailable");
                    }
                    if !attempts.is_empty() || construction.is_some() {
                        return Err("commit before action outcome");
                    }
                }
                CaptureEvent::FunctionResult { .. } => {
                    if !attempts.is_empty() || construction.is_some() {
                        return Err("commit before action outcome");
                    }
                }
            }
        }
        if !attempts.is_empty() || construction.is_some() {
            return Err("unfinished attempt");
        }
        if self.result.is_some_and(|id| !ready.contains(&id)) {
            return Err("result is unavailable");
        }
        Ok(())
    }
}

/// Association sidecars around the existing canonical J Graph, not another IR.
#[derive(Clone, Debug)]
pub struct CapturedGraph {
    pub modifier_stack_snapshots: Vec<crate::semantic::ModifierSnapshot>,
    pub graph: crate::j_graph_ir::Plan,
    pub occurrences: Vec<(OccurrenceId, crate::j_graph_ir::ValueId)>,
    /// Runtime observations are separate from inferred facts and reuse guards.
    pub observed_facts: Vec<(crate::j_graph_ir::ValueId, GraphFacts)>,
    pub constructors: Vec<ConstructorOrigin>,
    pub modifier_bindings: Vec<ModifierBinding>,
    pub gerund_name_reads: Vec<GerundNameRead>,
    pub constructor_calls: Vec<ConstructorCall>,
}

#[derive(Clone, Debug)]
pub struct ConstructorOrigin {
    pub row: ParseRow,
    pub function: Arc<FunctionEntity>,
    pub noun_inputs: Vec<crate::j_graph_ir::ValueId>,
    pub span: Range<usize>,
}

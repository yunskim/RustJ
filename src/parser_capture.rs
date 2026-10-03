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
    Input {
        id: OccurrenceId,
        name: Option<String>,
        version: Option<NameVersion>,
        span: Range<usize>,
        facts: GraphFacts,
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
    },
}

#[derive(Clone, Debug, Default)]
pub struct ParseCapture {
    source: String,
    pub events: Vec<CaptureEvent>,
    pub result: Option<OccurrenceId>,
    next_id: usize,
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

    /// Check associations and attempt/outcome ordering, including partial failure.
    pub fn verify(&self) -> std::result::Result<(), &'static str> {
        let mut ready = BTreeSet::new();
        let mut attempts = BTreeMap::new();
        let mut next = 0;
        let mut construction = None;
        for event in &self.events {
            match event {
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
                    if noun_inputs.iter().any(|id| !ready.contains(id)) {
                        return Err("constructor input is unavailable");
                    }
                }
                CaptureEvent::ConstructionSuccess { row, .. }
                | CaptureEvent::ConstructionFailure { row, .. } => {
                    if construction.take() != Some(*row) {
                        return Err("construction outcome without matching attempt");
                    }
                }
                CaptureEvent::Commit { .. } | CaptureEvent::FunctionResult { .. } => {
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
    pub graph: crate::j_graph_ir::Plan,
    pub occurrences: Vec<(OccurrenceId, crate::j_graph_ir::ValueId)>,
    /// Runtime observations are separate from inferred facts and reuse guards.
    pub observed_facts: Vec<(crate::j_graph_ir::ValueId, GraphFacts)>,
    pub constructors: Vec<ConstructorOrigin>,
}

#[derive(Clone, Debug)]
pub struct ConstructorOrigin {
    pub row: ParseRow,
    pub function: Arc<FunctionEntity>,
    pub noun_inputs: Vec<crate::j_graph_ir::ValueId>,
    pub span: Range<usize>,
}

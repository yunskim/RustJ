//! Schema-driven, non-executing J graph and logical-memory analysis.
//!
//! Input declarations provide J noun POS and facts, never array contents
//! or kernel execution. This report is not an executable plan or proof that a call is error-free.
use crate::{
    Error, Result,
    enqueuer::{EnqueueClass, EnqueueFlags, EnqueuedPayload, enqueue},
    error::DiagnosticPhase,
    j_graph_ir::{GraphAnalyzability, GraphFacts, Plan, ValueId},
    j_graph_memory::StaticMemoryAnalysis,
    j_graph_resource::GraphResourceSummary,
    parser::{ParserNameBinding, parse_analysis},
    semantic::{FunctionPartOfSpeech, NameVersion, bind},
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogKind {
    Noun(GraphFacts),
    /// POS is known, but callable identity remains a J NameRef.
    Function(FunctionPartOfSpeech),
    /// Known core construction behavior, not only the operator's input POS.
    PrimitiveModifier(crate::primitive::PrimitiveSemanticId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogBinding {
    pub name: String,
    pub kind: CatalogKind,
    /// Catalog-local declaration version, NOT a runtime workspace witness.
    pub version: NameVersion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryReason {
    UnknownFacts,
    FunctionSpecialization,
    RuntimeSemantics,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisBoundary {
    pub value: ValueId,
    pub reason: BoundaryReason,
}

/// Owned source provenance for downstream analysis; literal arrays are omitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceWord {
    pub span: std::ops::Range<usize>,
    pub word_index: usize,
    pub class: EnqueueClass,
    pub flags: EnqueueFlags,
}

#[derive(Clone, Debug)]
pub struct StaticAnalysis {
    pub graph: Plan,
    pub source_words: Vec<SourceWord>,
    pub reductions: Vec<crate::parser::ParseReduction>,
    pub assignment_source: Option<crate::parser::AssignmentSource>,
    /// Memory facts keyed by source graph IDs.
    pub memory: StaticMemoryAnalysis,
    pub resources: GraphResourceSummary,
    /// Only the declarations used by this report; no data arrays are retained.
    pub inputs: Vec<CatalogBinding>,
    /// Boundaries reference the original source graph IDs.
    pub boundaries: Vec<AnalysisBoundary>,
}

#[derive(Default)]
pub struct StaticAnalyzer {
    catalog: BTreeMap<String, CatalogBinding>,
    revision: u64,
}

impl StaticAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn binding(&self, name: &str) -> Option<&CatalogBinding> {
        self.catalog.get(name)
    }

    pub fn declare_noun(&mut self, name: &str, mut facts: GraphFacts) -> Result<()> {
        if let Some(shape) = &facts.shape {
            if facts.rank.is_some_and(|rank| rank != shape.len()) {
                return Err(Error::Rank);
            }
            facts.rank = Some(shape.len());
            if !shape.contains(&0) {
                shape
                    .iter()
                    .try_fold(1usize, |n, d| n.checked_mul(*d))
                    .ok_or(Error::Limit)?;
            }
        }
        if facts.rank.is_some_and(|rank| rank > 63) {
            return Err(Error::Limit);
        }
        self.declare(name, CatalogKind::Noun(facts))
    }

    pub fn declare_function(&mut self, name: &str, pos: FunctionPartOfSpeech) -> Result<()> {
        self.declare(name, CatalogKind::Function(pos))
    }

    /// Declare registered core modifier semantics without executing its application.
    pub fn declare_primitive_modifier(
        &mut self,
        name: &str,
        id: crate::primitive::PrimitiveSemanticId,
    ) -> Result<()> {
        if !matches!(
            id,
            crate::primitive::PrimitiveSemanticId::Adverb(_)
                | crate::primitive::PrimitiveSemanticId::Conjunction(_)
        ) {
            return Err(Error::Domain);
        }
        self.declare(name, CatalogKind::PrimitiveModifier(id))
    }
    fn declare(&mut self, name: &str, kind: CatalogKind) -> Result<()> {
        let words = enqueue(name)?;
        if words.len() != 1 || !matches!(words[0].payload, EnqueuedPayload::Name(n) if n == name) {
            return Err(Error::IllFormedName);
        }
        let revision = self.revision.checked_add(1).ok_or(Error::Limit)?;
        self.catalog.insert(
            name.to_owned(),
            CatalogBinding {
                name: name.to_owned(),
                kind,
                version: NameVersion(revision),
            },
        );
        self.revision = revision;
        Ok(())
    }

    /// Analyze without invocation, assignment, or allocation of input arrays.
    /// Value-dependent constructions outside the current static subset return
    /// Unsupported, not a fabricated noun or a statement about J validity.
    pub fn analyze(&self, source: &str) -> Result<StaticAnalysis> {
        self.analyze_inner(source)
            .map_err(|e| e.in_phase(DiagnosticPhase::SemanticAnalysis))
    }

    fn analyze_inner(&self, source: &str) -> Result<StaticAnalysis> {
        let queue = enqueue(source)?;
        let source_words = queue
            .iter()
            .map(|word| SourceWord {
                span: word.span.clone(),
                word_index: word.word_index,
                class: word.class,
                flags: word.flags,
            })
            .collect();
        let mut inputs = BTreeMap::new();
        for word in &queue {
            if word.class == EnqueueClass::Name && word.flags.lookup_name {
                let EnqueuedPayload::Name(name) = word.payload else {
                    unreachable!()
                };
                let declaration = self.catalog.get(name).ok_or_else(|| {
                    Error::Unsupported(format!("static catalog has no binding/POS for {name:?}"))
                        .at(word.span.clone())
                        .blamed_on_word(word.word_index)
                })?;
                inputs.insert(name.to_owned(), declaration.clone());
            }
        }
        let program = parse_analysis(source, &|name| {
            self.catalog.get(name).map(|entry| match &entry.kind {
                CatalogKind::Noun(_) => ParserNameBinding::AbstractNoun,
                CatalogKind::Function(pos) => ParserNameBinding::Function(*pos),
                CatalogKind::PrimitiveModifier(id) => {
                    let function = match id {
                        crate::primitive::PrimitiveSemanticId::Adverb(id) => {
                            crate::semantic::FunctionEntity::primitive_adverb(*id, 0..0)
                        }
                        crate::primitive::PrimitiveSemanticId::Conjunction(id) => {
                            crate::semantic::FunctionEntity::primitive_conjunction(*id, 0..0)
                        }
                        _ => unreachable!("validated declaration"),
                    };
                    ParserNameBinding::KnownModifier {
                        function,
                        version: entry.version,
                    }
                }
            })
        })?;
        let reductions = program.reductions.clone();
        let assignment_source = program.assignment_source.clone();
        let bound = bind(program, |name| {
            self.catalog.get(name).map(|entry| entry.version)
        })?;
        let graph = Plan::from_bound_with_graph_facts(bound, &|name| match self
            .catalog
            .get(name)
            .map(|entry| &entry.kind)
        {
            Some(CatalogKind::Noun(facts)) => facts.clone(),
            _ => GraphFacts::default(),
        })?;
        graph
            .verify()
            .map_err(|message| Error::Unsupported(format!("invalid static J graph: {message}")))?;
        let memory = graph.static_memory_analysis();
        let resources = graph.symbolic_resource_analysis();
        let boundaries = graph
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let reason = match node.analyzability {
                    GraphAnalyzability::Static => return None,
                    GraphAnalyzability::StaticWithUnknownFacts => BoundaryReason::UnknownFacts,
                    GraphAnalyzability::RequiresSpecialization => {
                        BoundaryReason::FunctionSpecialization
                    }
                    GraphAnalyzability::DynamicSemanticFallback => BoundaryReason::RuntimeSemantics,
                };
                Some(AnalysisBoundary {
                    value: ValueId(index),
                    reason,
                })
            })
            .collect();
        Ok(StaticAnalysis {
            graph,
            source_words,
            reductions,
            assignment_source,
            memory,
            resources,
            inputs: inputs.into_values().collect(),
            boundaries,
        })
    }
}

//! Owned source identities and composable byte-coordinate provenance.
//! Diagnostic identity is independent of names, bindings and frontend units.
use crate::{Error, Result, definition_code::DefinitionSourceMap};
use std::{
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceUnitId(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceUnit {
    id: SourceUnitId,
    name: Arc<str>,
    text: Arc<str>,
}

impl SourceUnit {
    pub fn id(&self) -> SourceUnitId {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn text(&self) -> &str {
        &self.text
    }

    /// A new immutable input revision, even when its name/text match another.
    pub fn new(name: impl Into<Arc<str>>, text: impl Into<Arc<str>>) -> Arc<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        // Preserve non-wrapping unique IDs on Rust 1.85 without the deprecated
        // fetch_update API or the newer (1.95+) try_update API.
        let mut id = NEXT.load(Ordering::Relaxed);
        loop {
            let next = id.checked_add(1).expect("source unit identity exhausted");
            match NEXT.compare_exchange_weak(id, next, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(current) => id = current,
            }
        }
        Arc::new(Self {
            id: SourceUnitId(id),
            name: name.into(),
            text: text.into(),
        })
    }

    pub fn origin(self: &Arc<Self>) -> SourceOrigin {
        SourceOrigin {
            unit: self.clone(),
            text: self.text.clone(),
            text_range: 0..self.text.len(),
            mapping: Arc::new(Mapping::Root),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Mapping {
    Root,
    Slice {
        parent: SourceOrigin,
        start: usize,
    },
    Body {
        parent: SourceOrigin,
        map: DefinitionSourceMap,
    },
}

/// Local parser text plus a checked path to its immutable root input.
/// Sparse quote maps and shared parents avoid per-byte root maps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceOrigin {
    unit: Arc<SourceUnit>,
    text: Arc<str>,
    text_range: Range<usize>,
    mapping: Arc<Mapping>,
}

impl SourceOrigin {
    pub fn unit(&self) -> &Arc<SourceUnit> {
        &self.unit
    }
    pub fn text(&self) -> &str {
        &self.text[self.text_range.clone()]
    }

    pub fn slice(&self, span: Range<usize>) -> Result<Self> {
        self.text()
            .get(span.clone())
            .ok_or_else(|| Error::Unsupported("invalid source slice".into()))?;
        Ok(Self {
            unit: self.unit.clone(),
            text: self.text.clone(),
            text_range: self.text_range.start + span.start..self.text_range.start + span.end,
            mapping: Arc::new(Mapping::Slice {
                parent: self.clone(),
                start: span.start,
            }),
        })
    }

    pub(crate) fn body(&self, text: Arc<str>, map: DefinitionSourceMap) -> Self {
        Self {
            unit: self.unit.clone(),
            text_range: 0..text.len(),
            text,
            mapping: Arc::new(Mapping::Body {
                parent: self.clone(),
                map,
            }),
        }
    }

    pub fn root_span(&self, span: Range<usize>) -> Option<Range<usize>> {
        self.text().get(span.clone())?;
        let mut span = span;
        let mut origin = self;
        loop {
            match origin.mapping.as_ref() {
                Mapping::Root => {
                    origin.unit.text.get(span.clone())?;
                    return Some(span);
                }
                Mapping::Slice { parent, start } => {
                    span = start.checked_add(span.start)?..start.checked_add(span.end)?;
                    origin = parent;
                }
                Mapping::Body { parent, map } => {
                    span = map.original_span(span)?;
                    origin = parent;
                }
            }
        }
    }
}

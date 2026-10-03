//! Immutable explicit-definition code. No invocation values or name bindings.
use crate::{
    Error, Result,
    definition_input::{DefinitionForm, DefinitionInput},
    enqueuer::{EnqueueClass, EnqueueEnvironment, EnqueueFlags},
    primitive::PrimitiveContext,
    semantic::FunctionPartOfSpeech,
};
use std::{borrow::Cow, ops::Range, sync::Arc};

#[derive(Clone, Debug)]
pub struct DefinitionSource {
    pub source: Arc<str>,
    pub input: DefinitionInput,
    pub primitives: Arc<PrimitiveContext>,
}

impl PartialEq for DefinitionSource {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.input == other.input
            && Arc::ptr_eq(&self.primitives, &other.primitives)
    }
}
impl Eq for DefinitionSource {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionWord {
    pub span: Range<usize>,
    pub index: usize,
    pub class: EnqueueClass,
    pub flags: EnqueueFlags,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionSentence {
    /// Byte range in body, and original body line (zero based).
    pub span: Range<usize>,
    pub line: usize,
    pub words: Vec<DefinitionWord>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionCode {
    pub source: Arc<str>,
    pub body: Arc<str>,
    pub source_span: Range<usize>,
    pub form: DefinitionForm,
    /// Resolved explicit mode, including direct definition inference.
    pub mode: u8,
    pub result_pos: FunctionPartOfSpeech,
    pub sentences: Vec<DefinitionSentence>,
    pub monad: Range<usize>,
    pub dyad: Range<usize>,
    /// C VXOPR: operator definition refers to x/y invocation arguments.
    pub operator_definition: bool,
}

pub(crate) fn semantic_body<'a>(source: &'a str, input: &DefinitionInput) -> Result<Cow<'a, str>> {
    let raw = input.body_text(source)?;
    if input.form != DefinitionForm::Direct {
        return Ok(raw);
    }
    let first = crate::tokenizer::scan_unfinished(raw.as_bytes())
        .first()
        .map_or(raw.len(), |span| span.start);
    let start = first + usize::from(raw.as_bytes().get(first) == Some(&b'\n'));
    Ok(match raw {
        Cow::Borrowed(text) => Cow::Borrowed(&text[start..]),
        Cow::Owned(text) => Cow::Owned(text[start..].to_owned()),
    })
}

fn body_error(
    error: Error,
    source: &str,
    input: &DefinitionInput,
    body: &str,
    line_offset: usize,
    line_len: usize,
) -> Error {
    let relative = error.span().cloned().unwrap_or(0..line_len);
    let boundary = |decoded: usize| {
        if matches!(input.form, DefinitionForm::ExplicitString(_)) {
            let mut original = input.body.start + 1;
            let bytes = source.as_bytes();
            for _ in 0..decoded {
                original += if bytes[original] == b'\'' && bytes.get(original + 1) == Some(&b'\'') {
                    2
                } else {
                    1
                };
            }
            original
        } else {
            // Direct bodies skip leading whitespace and at most one initial LF.
            input.body.start + (input.body.len() - body.len()) + decoded
        }
    };
    let mut context = error.context().cloned().unwrap_or_default();
    context.span =
        Some(boundary(line_offset + relative.start)..boundary(line_offset + relative.end));
    // A body queue's index is not an outer expanded sentence index.
    context.blame_word_index = None;
    error.into_unlocated().with_context(context)
}

pub fn compile(
    source: &str,
    input: &DefinitionInput,
    primitives: &PrimitiveContext,
) -> Result<Arc<DefinitionCode>> {
    if !input.nested.is_empty() {
        return Err(
            Error::Unsupported("nested definition code construction".into()).at(input.span.clone()),
        );
    }
    let body: Arc<str> = Arc::from(semantic_body(source, input)?.as_ref());
    let mut lines: Vec<_> = body.split_inclusive('\n').collect();
    if lines.is_empty() {
        lines.push("");
    }
    let mut sentences = Vec::new();
    let mut offset = 0;
    let mut split = None;
    let mut names = Vec::new();
    for (line_index, physical) in lines.iter().enumerate() {
        let line = physical.trim_end_matches(['\r', '\n']);
        // cx.c only recognizes a spaces-only : before the final body line.
        if line_index + 1 < lines.len() && line.trim_matches(' ') == ":" && split.is_none() {
            split = Some(sentences.len());
            offset += physical.len();
            continue;
        }
        let parts = crate::definition_control::partition_line(line)
            .map_err(|error| body_error(error, source, input, &body, offset, line.len()))?;
        if parts.iter().any(|part| part.control.is_some()) {
            return Err(
                Error::Unsupported("definition control-flow audit is pending".into())
                    .at(input.span.clone()),
            );
        }
        let queue = crate::enqueuer::enqueue_in_environment(
            line,
            primitives,
            EnqueueEnvironment::ExplicitDefinition,
        )
        .map_err(|error| body_error(error, source, input, &body, offset, line.len()))?;
        let mut words = Vec::with_capacity(queue.len());
        for word in queue {
            if let crate::enqueuer::EnqueuedPayload::Name(name) = &word.payload {
                names.push((*name).to_owned());
            }
            words.push(DefinitionWord {
                span: offset + word.span.start..offset + word.span.end,
                index: word.word_index,
                class: word.class,
                flags: word.flags,
            });
        }
        sentences.push(DefinitionSentence {
            span: offset..offset + line.len(),
            line: line_index,
            words,
        });
        offset += physical.len();
    }
    let mode = match input.form {
        DefinitionForm::Direct => {
            if names.iter().any(|n| n == "v" || n == "n") {
                2
            } else if names.iter().any(|n| n == "u" || n == "m") {
                1
            } else if split.is_none() && names.iter().any(|n| n == "x") {
                4
            } else {
                3
            }
        }
        DefinitionForm::ExplicitString(mode) | DefinitionForm::ExplicitBlock(mode) => mode,
    };
    let result_pos = match mode {
        1 => FunctionPartOfSpeech::Adverb,
        2 => FunctionPartOfSpeech::Conjunction,
        _ => FunctionPartOfSpeech::Verb,
    };
    // cx.c xop: infer mode first, then rearrange operator valences.
    let len = sentences.len();
    let (mut monad, mut dyad) = if mode == 4 {
        (
            0..0,
            split.map_or(0..len, |s| if s == len { 0..s } else { s..len }),
        )
    } else {
        (0..split.unwrap_or(len), split.map_or(len..len, |s| s..len))
    };
    let has_x = names.iter().any(|n| n == "x");
    let has_y = names.iter().any(|n| n == "y");
    let operator_definition = mode <= 2 && (has_x || has_y);
    if mode <= 2 {
        if !operator_definition && !monad.is_empty() && !dyad.is_empty() {
            return Err(Error::Valence.at(input.span.clone()));
        }
        if !monad.is_empty()
            && dyad.is_empty()
            && ((!operator_definition && mode == 2) || (operator_definition && has_x))
        {
            std::mem::swap(&mut monad, &mut dyad);
        }
    }
    Ok(Arc::new(DefinitionCode {
        source: Arc::from(source),
        body,
        source_span: input.span.clone(),
        form: input.form,
        mode,
        result_pos,
        sentences,
        monad,
        dyad,
        operator_definition,
    }))
}

impl DefinitionCode {
    /// Semantic 5!:1 body projection after valence rearrangement; source stays intact.
    pub fn representation_lines(&self) -> Vec<&str> {
        let section = |range: Range<usize>| {
            self.sentences[range]
                .iter()
                .map(|sentence| &self.body[sentence.span.clone()])
                .collect::<Vec<_>>()
        };
        if self.mode == 4 {
            return section(self.dyad.clone());
        }
        if self.mode <= 2 && !self.operator_definition {
            return section(if self.monad.is_empty() {
                self.dyad.clone()
            } else {
                self.monad.clone()
            });
        }
        let mut lines = section(self.monad.clone());
        if !self.dyad.is_empty() {
            lines.push(":");
            lines.extend(section(self.dyad.clone()));
        }
        lines
    }
}

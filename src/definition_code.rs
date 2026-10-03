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
    pub monad_controls: Vec<crate::definition_flow::ControlNode>,
    pub dyad_controls: Vec<crate::definition_flow::ControlNode>,
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
    use crate::{
        definition_control::ControlWord as W,
        definition_flow::{ControlJump as J, ControlKind as K, ControlNode},
    };
    let split_line = lines
        .iter()
        .enumerate()
        .find(|(i, line)| {
            *i + 1 < lines.len() && line.trim_end_matches(['\r', '\n']).trim_matches(' ') == ":"
        })
        .map(|(i, _)| i);
    let mut sentences = Vec::new();
    let mut offset = 0;
    let mut split = None;
    let mut names = Vec::new();
    let mut sections: [Vec<ControlNode>; 2] = [Vec::new(), Vec::new()];
    let mut queued_words = [0usize; 2];
    let mut audited = [false; 2];
    let mut pending_assert: Option<std::ops::Range<usize>> = None;
    for (line_index, physical) in lines.iter().enumerate() {
        let line = physical.trim_end_matches(['\r', '\n']);
        if Some(line_index) == split_line {
            if let Some(span) = pending_assert.take() {
                return Err(body_error(
                    Error::Control.at(span),
                    source,
                    input,
                    &body,
                    0,
                    body.len(),
                ));
            }
            // C calls preparse separately: audit monad before enqueueing dyad.
            crate::definition_flow::audit(&mut sections[0])
                .map_err(|e| body_error(e, source, input, &body, 0, body.len()))?;
            audited[0] = true;
            split = Some(sentences.len());
            offset += physical.len();
            continue;
        }
        let side = usize::from(split.is_some());
        let mode4 = matches!(
            input.form,
            DefinitionForm::ExplicitString(4) | DefinitionForm::ExplicitBlock(4)
        );
        // C discards a supplied monadic section before preparse for literal 4 :.
        let ignored = mode4 && split_line.is_some() && split.is_none();
        let mut words = Vec::new();
        if !ignored {
            let parts = crate::definition_control::partition_line(line)
                .map_err(|e| body_error(e, source, input, &body, offset, line.len()))?;
            for part in parts {
                let span = offset + part.span.start..offset + part.span.end;
                // j.h CWMAX/SWMAX/EXPWMAX; pending assert occupies a temporary slot.
                if sections[side].len() + usize::from(pending_assert.is_some()) + 1 >= 32766 {
                    return Err(body_error(
                        Error::Limit.at(span),
                        source,
                        input,
                        &body,
                        0,
                        body.len(),
                    ));
                }
                if let Some(control) = part.control {
                    if let Some(marker) = pending_assert.take() {
                        return Err(body_error(
                            Error::Control.at(marker),
                            source,
                            input,
                            &body,
                            0,
                            body.len(),
                        ));
                    }
                    if control == W::Assert {
                        pending_assert = Some(span);
                        continue;
                    }
                    let named_target = match control {
                        W::Goto => Some(body[span.start + 5..span.end - 1].to_owned()),
                        W::Label => Some(body[span.start + 6..span.end - 1].to_owned()),
                        _ => None,
                    };
                    if matches!(control, W::For | W::Goto | W::Label) {
                        queued_words[side] += 1;
                    }
                    let go = if matches!(control, W::Break | W::Continue | W::Throw) {
                        J::DynamicError
                    } else if control == W::Return {
                        J::Return
                    } else {
                        J::Index(sections[side].len() + 1)
                    };
                    sections[side].push(ControlNode {
                        span,
                        line: line_index,
                        words: words.len()..words.len(),
                        kind: K::Word(control),
                        go,
                        assertion: None,
                        analysis_barrier: false,
                        before_fallthrough_end: false,
                        previous_result: Default::default(),
                        named_target,
                    });
                } else {
                    let begin = words.len();
                    let queue = crate::enqueuer::enqueue_in_environment(
                        &line[part.span.clone()],
                        primitives,
                        EnqueueEnvironment::ExplicitDefinition,
                    )
                    .map_err(|e| {
                        body_error(
                            e,
                            source,
                            input,
                            &body,
                            offset + part.span.start,
                            part.span.len(),
                        )
                    })?;
                    if queue.len() >= 32767 {
                        return Err(body_error(
                            Error::Limit.at(span),
                            source,
                            input,
                            &body,
                            0,
                            body.len(),
                        ));
                    }
                    queued_words[side] += queue.len();
                    for word in queue {
                        if let crate::enqueuer::EnqueuedPayload::Name(name) = &word.payload {
                            names.push((*name).to_owned());
                        }
                        words.push(DefinitionWord {
                            span: span.start + word.span.start..span.start + word.span.end,
                            index: words.len(),
                            class: word.class,
                            flags: word.flags,
                        });
                    }
                    let assertion = pending_assert.take();
                    sections[side].push(ControlNode {
                        span,
                        line: line_index,
                        words: begin..words.len(),
                        kind: if assertion.is_some() {
                            K::Assert
                        } else {
                            K::Body
                        },
                        go: J::DynamicError,
                        assertion,
                        analysis_barrier: false,
                        before_fallthrough_end: false,
                        previous_result: Default::default(),
                        named_target: None,
                    });
                }
            }
        }
        sentences.push(DefinitionSentence {
            span: offset..offset + line.len(),
            line: line_index,
            words,
        });
        offset += physical.len();
        if queued_words[side] >= 16777215 {
            return Err(body_error(
                Error::Limit,
                source,
                input,
                &body,
                offset - physical.len(),
                line.len(),
            ));
        }
    }
    if let Some(span) = pending_assert {
        return Err(body_error(
            Error::Control.at(span),
            source,
            input,
            &body,
            0,
            body.len(),
        ));
    }
    // Within each valence enqueue completes before conall auditing.
    for (index, section) in sections.iter_mut().enumerate() {
        if audited[index] {
            continue;
        }
        crate::definition_flow::audit(section)
            .map_err(|e| body_error(e, source, input, &body, 0, body.len()))?;
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
    let [mut monad_controls, mut dyad_controls] = sections;
    if mode == 4 || (mode <= 2 && monad.is_empty() && !dyad.is_empty() && dyad_controls.is_empty())
    {
        if dyad_controls.is_empty() {
            std::mem::swap(&mut monad_controls, &mut dyad_controls);
        }
        if mode == 4 {
            monad_controls.clear();
        }
    }
    let code = DefinitionCode {
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
        monad_controls,
        dyad_controls,
    };
    code.verify()?;
    Ok(Arc::new(code))
}

impl DefinitionCode {
    /// Verify source/word/control references before consuming this code in analysis.
    pub fn verify(&self) -> Result<()> {
        for range in [&self.monad, &self.dyad] {
            if self.sentences.get(range.clone()).is_none() {
                return Err(Error::Unsupported(
                    "definition valence range outside source lines".into(),
                ));
            }
        }
        for (nodes, range) in [
            (&self.monad_controls, &self.monad),
            (&self.dyad_controls, &self.dyad),
        ] {
            crate::definition_flow::verify(nodes)?;
            for node in nodes {
                let position = self
                    .sentences
                    .binary_search_by_key(&node.line, |line| line.line)
                    .map_err(|_| Error::Unsupported("control source line is missing".into()))?;
                if !range.contains(&position) {
                    return Err(Error::Unsupported(
                        "control source belongs to a different valence".into(),
                    ));
                }
                let target = match node.kind {
                    crate::definition_flow::ControlKind::Word(
                        crate::definition_control::ControlWord::Goto,
                    ) => self
                        .body
                        .get(node.span.clone())
                        .and_then(|s| s.strip_prefix("goto_"))
                        .and_then(|s| s.strip_suffix('.')),
                    crate::definition_flow::ControlKind::Word(
                        crate::definition_control::ControlWord::Label,
                    ) => self
                        .body
                        .get(node.span.clone())
                        .and_then(|s| s.strip_prefix("label_"))
                        .and_then(|s| s.strip_suffix('.')),
                    _ => None,
                };
                if target != node.named_target.as_deref() {
                    return Err(Error::Unsupported("invalid named control source".into()));
                }
                let line = &self.sentences[position];
                if self.body.get(node.span.clone()).is_none()
                    || node.span.start < line.span.start
                    || node.span.end > line.span.end
                    || line.words.get(node.words.clone()).is_none()
                {
                    return Err(Error::Unsupported(
                        "control source/word reference outside body".into(),
                    ));
                }
                for word in &line.words[node.words.clone()] {
                    if self.body.get(word.span.clone()).is_none()
                        || word.span.start < node.span.start
                        || word.span.end > node.span.end
                    {
                        return Err(Error::Unsupported(
                            "statement word outside control fragment".into(),
                        ));
                    }
                }
                if node
                    .assertion
                    .as_ref()
                    .is_some_and(|span| self.body.get(span.clone()) != Some("assert."))
                {
                    return Err(Error::Unsupported("invalid assertion source marker".into()));
                }
            }
        }
        Ok(())
    }

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

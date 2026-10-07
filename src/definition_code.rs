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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefinitionNameRole {
    /// A declaration does not prebind a read: unbound locals can fall back globally.
    ReadCurrentFrameThenGlobal,
    LocalAssignmentTarget,
    GlobalAssignmentTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionNameOccurrence {
    pub sentence: usize,
    pub word: usize,
    pub span: Range<usize>,
    pub role: DefinitionNameRole,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DefinitionScopePlan {
    /// Literal simple-name =. targets for this valence; not a closed-world proof.
    /// Implicit operands/arguments are supplied separately by each invocation.
    pub local_declarations: Vec<String>,
    pub occurrences: Vec<DefinitionNameOccurrence>,
    /// Computed/noun assignment targets cannot be guessed from name spelling.
    pub dynamic_assignment_targets: Vec<(usize, usize)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DefinitionNamePlan {
    pub monad: DefinitionScopePlan,
    pub dyad: DefinitionScopePlan,
}

fn scope_plan(
    body: &str,
    sentences: &[DefinitionSentence],
    range: Range<usize>,
) -> DefinitionScopePlan {
    let mut plan = DefinitionScopePlan::default();
    let mut declarations = std::collections::BTreeSet::new();
    for sentence_index in range {
        let sentence = &sentences[sentence_index];
        for (word_index, word) in sentence.words.iter().enumerate() {
            if word.class == EnqueueClass::Assignment
                && (word_index == 0 || sentence.words[word_index - 1].class != EnqueueClass::Name)
            {
                plan.dynamic_assignment_targets
                    .push((sentence_index, word_index));
            }
            if word.class != EnqueueClass::Name {
                continue;
            }
            let role = match sentence.words.get(word_index + 1) {
                Some(copula) if copula.class == EnqueueClass::Assignment => {
                    if copula.flags.local_assignment {
                        declarations.insert(body[word.span.clone()].to_owned());
                        DefinitionNameRole::LocalAssignmentTarget
                    } else {
                        DefinitionNameRole::GlobalAssignmentTarget
                    }
                }
                _ => DefinitionNameRole::ReadCurrentFrameThenGlobal,
            };
            plan.occurrences.push(DefinitionNameOccurrence {
                sentence: sentence_index,
                word: word_index,
                span: word.span.clone(),
                role,
            });
        }
    }
    plan.local_declarations = declarations.into_iter().collect();
    plan
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionSourceMap {
    start: usize,
    len: usize,
    /// Decoded byte positions where the original contains a doubled quote.
    escaped_quotes: Vec<usize>,
}

impl DefinitionSourceMap {
    fn new(source: &str, input: &DefinitionInput, body: &str) -> Self {
        let mut escaped_quotes = Vec::new();
        let start = if matches!(input.form, DefinitionForm::ExplicitString(_)) {
            let bytes = &source.as_bytes()[input.body.start + 1..input.body.end - 1];
            let (mut original, mut decoded) = (0, 0);
            while original < bytes.len() {
                let doubled = bytes[original] == b'\'' && bytes.get(original + 1) == Some(&b'\'');
                if doubled {
                    escaped_quotes.push(decoded);
                }
                original += 1 + usize::from(doubled);
                decoded += 1;
            }
            input.body.start + 1
        } else {
            input.body.end - body.len()
        };
        Self {
            start,
            len: body.len(),
            escaped_quotes,
        }
    }

    /// Map decoded body byte boundaries to the immutable original source.
    pub fn original_span(&self, span: Range<usize>) -> Option<Range<usize>> {
        if span.start > span.end || span.end > self.len {
            return None;
        }
        let boundary = |n| self.start + n + self.escaped_quotes.partition_point(|&q| q < n);
        Some(boundary(span.start)..boundary(span.end))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionCode {
    pub name_plan: DefinitionNamePlan,
    pub source: Arc<str>,
    pub body: Arc<str>,
    pub source_span: Range<usize>,
    pub source_map: DefinitionSourceMap,
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
    if input.form == DefinitionForm::NounDirect {
        return Err(Error::Domain.at(input.span.clone()));
    }
    let body: Arc<str> = Arc::from(semantic_body(source, input)?.as_ref());
    // Collect complete input units before partitioning outer control words.
    // A nested definition owns its physical lines, names and valence separator.
    let mut lines = Vec::new();
    let mut unit_start = 0;
    let mut unit_line = 0;
    let mut physical_end = 0;
    for (line_index, physical) in body.split_inclusive('\n').enumerate() {
        physical_end += physical.len();
        let unit = &body[unit_start..physical_end];
        let framed = crate::definition_input::frame(unit)
            .map_err(|e| body_error(e, source, input, &body, unit_start, unit.len()))?;
        if matches!(
            framed,
            crate::definition_input::InputFrame::Definition(DefinitionInput {
                form: DefinitionForm::ExplicitBlock(_),
                ..
            })
        ) {
            // C colon0 reads from the external input stream, not the enclosing
            // immutable body's lines. An embedded ')' is a syntax error.
            return Err(Error::Syntax(
                "nested colon-zero input is not an embedded definition".into(),
            ));
        }
        if !matches!(framed, crate::definition_input::InputFrame::NeedMore) {
            lines.push((unit_line, unit));
            unit_start = physical_end;
            unit_line = line_index + 1;
        }
    }
    if unit_start != body.len() {
        return Err(Error::Syntax("incomplete nested definition".into()));
    }
    if lines.is_empty() {
        lines.push((0, ""));
    }
    use crate::{
        definition_control::ControlWord as W,
        definition_flow::{ControlJump as J, ControlKind as K, ControlNode},
    };
    let split_line = lines
        .iter()
        .enumerate()
        .find(|(i, (_, line))| {
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
    for (unit_index, &(line_index, physical)) in lines.iter().enumerate() {
        let line = physical.trim_end_matches(['\r', '\n']);
        if Some(unit_index) == split_line {
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
                        // cx.c xop counts the implicit-locative primitives as
                        // operands too; their lexical class remains VERB.
                        if let crate::enqueuer::EnqueuedPayload::Verb(id) = &word.payload {
                            use crate::primitive::PrimitiveId;
                            match id {
                                PrimitiveId::OperandU => names.push("u".into()),
                                PrimitiveId::OperandV => names.push("v".into()),
                                _ => {}
                            }
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
        DefinitionForm::NounDirect => return Err(Error::Domain),
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
        source_map: DefinitionSourceMap::new(source, input, &body),
        name_plan: DefinitionNamePlan {
            monad: scope_plan(&body, &sentences, monad.clone()),
            dyad: scope_plan(&body, &sentences, dyad.clone()),
        },
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
        let mapped = self
            .source_map
            .original_span(0..self.body.len())
            .and_then(|span| self.source.get(span));
        if mapped.is_none_or(|text| {
            if matches!(self.form, DefinitionForm::ExplicitString(_)) {
                let mut original = text.bytes();
                let matches = self.body.bytes().all(|byte| {
                    original.next() == Some(byte)
                        && (byte != b'\'' || original.next() == Some(b'\''))
                });
                !matches || original.next().is_some()
            } else {
                text != self.body.as_ref()
            }
        }) {
            return Err(Error::Unsupported(
                "definition source map does not match body".into(),
            ));
        }
        for sentence in &self.sentences {
            if self.body.get(sentence.span.clone()).is_none()
                || sentence
                    .words
                    .iter()
                    .any(|word| self.body.get(word.span.clone()).is_none())
            {
                return Err(Error::Unsupported(
                    "definition NAME source outside body".into(),
                ));
            }
        }
        for range in [&self.monad, &self.dyad] {
            if self.sentences.get(range.clone()).is_none() {
                return Err(Error::Unsupported(
                    "definition valence range outside source lines".into(),
                ));
            }
        }
        for (plan, range) in [
            (&self.name_plan.monad, &self.monad),
            (&self.name_plan.dyad, &self.dyad),
        ] {
            if plan != &scope_plan(&self.body, &self.sentences, range.clone()) {
                return Err(Error::Unsupported(
                    "definition NAME plan does not match source/valence".into(),
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

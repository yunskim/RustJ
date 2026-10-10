//! wc.c::conword/getsen source partitioning. No control-flow audit or execution.
use crate::{Error, Result};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlWord {
    Do,
    If,
    End,
    Else,
    While,
    ElseIf,
    For,
    Return,
    Break,
    Continue,
    Select,
    Case,
    FCase,
    Whilst,
    Assert,
    Throw,
    Try,
    Catch,
    CatchD,
    CatchT,
    Goto,
    Label,
}

pub const FIXED_WORDS: &[(&str, ControlWord)] = &[
    ("do.", ControlWord::Do),
    ("if.", ControlWord::If),
    ("end.", ControlWord::End),
    ("else.", ControlWord::Else),
    ("while.", ControlWord::While),
    ("elseif.", ControlWord::ElseIf),
    ("for.", ControlWord::For),
    ("return.", ControlWord::Return),
    ("break.", ControlWord::Break),
    ("continue.", ControlWord::Continue),
    ("select.", ControlWord::Select),
    ("case.", ControlWord::Case),
    ("fcase.", ControlWord::FCase),
    ("whilst.", ControlWord::Whilst),
    ("assert.", ControlWord::Assert),
    ("throw.", ControlWord::Throw),
    ("try.", ControlWord::Try),
    ("catch.", ControlWord::Catch),
    ("catchd.", ControlWord::CatchD),
    ("catcht.", ControlWord::CatchT),
];

pub fn classify(word: &str) -> Result<Option<ControlWord>> {
    if !word.ends_with('.') || word.len() < 3 {
        return Ok(None);
    }
    if let Some((_, kind)) = FIXED_WORDS.iter().find(|(text, _)| *text == word) {
        return Ok(Some(*kind));
    }
    if let Some(name) = word.strip_prefix("for_").and_then(|s| s.strip_suffix('.')) {
        // conword audits vnm here. Reuse the supported ordinary name contract;
        // locative names remain an explicit unsupported capability.
        if !name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(Error::IllFormedName);
        }
        let queue = crate::enqueuer::enqueue(name)?;
        if queue.len() != 1
            || !matches!(queue[0].payload, crate::enqueuer::EnqueuedPayload::Name(_))
        {
            return Err(Error::IllFormedName);
        }
        if queue[0].flags.name_form.is_locative() {
            return Err(Error::Unsupported("J locative loop binding".into()));
        }
        return Ok(Some(ControlWord::For));
    }
    // goto/label targets are audited later by congoto, not by conword.
    Ok(if word.starts_with("goto_") {
        Some(ControlWord::Goto)
    } else if word.starts_with("label_") {
        Some(ControlWord::Label)
    } else {
        None
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionPart {
    pub span: Range<usize>,
    pub control: Option<ControlWord>,
}

/// Split a physical body line at actual control words, preserving getsen's
/// spacing before controls and discarding leading/trailing space and comments.
pub fn partition_line(source: &str) -> Result<Vec<DefinitionPart>> {
    let spans = crate::tokenizer::parse_word_spans(source.as_bytes())?;
    use crate::definition_input::InputFrame;
    let definitions = match crate::definition_input::frame(source)? {
        InputFrame::Definition(input) => vec![input.span],
        InputFrame::Definitions(inputs) => inputs.into_iter().map(|input| input.span).collect(),
        _ => Vec::new(),
    };
    let mut parts = Vec::new();
    let mut start = None;
    let mut last_end = 0;
    for span in spans {
        let first = *start.get_or_insert(span.start);
        let control = if definitions
            .iter()
            .any(|definition| definition.start <= span.start && span.end <= definition.end)
        {
            None
        } else {
            classify(&source[span.clone()]).map_err(|error| error.at(span.clone()))?
        };
        if control.is_some() {
            if first < span.start {
                parts.push(DefinitionPart {
                    span: first..span.start,
                    control: None,
                });
            }
            parts.push(DefinitionPart {
                span: span.clone(),
                control,
            });
            start = None;
        }
        last_end = span.end;
    }
    if let Some(first) = start {
        parts.push(DefinitionPart {
            span: first..last_end,
            control: None,
        });
    }
    Ok(parts)
}

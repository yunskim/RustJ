//! Execution-free input framing before J enqueue/reduction.
//! This preserves source; it does not create a callable or resolve body names.
use crate::{Error, Result, tokenizer};
use std::{borrow::Cow, ops::Range};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefinitionForm {
    Direct,
    ExplicitString(u8),
    ExplicitBlock(u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionInput {
    pub form: DefinitionForm,
    /// Definition operator/delimiters, excluding the assignment prefix.
    pub span: Range<usize>,
    /// Direct/block text, or the complete quoted word for ExplicitString.
    pub body: Range<usize>,
    /// Nested direct definitions, in closing order (inner before outer).
    pub nested: Vec<Range<usize>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputFrame {
    Sentence,
    NeedMore,
    Definition(DefinitionInput),
}

impl DefinitionInput {
    pub fn body_text<'a>(&self, source: &'a str) -> Result<Cow<'a, str>> {
        let body = source
            .get(self.body.clone())
            .ok_or_else(|| Error::Syntax("definition body span outside source".into()))?;
        if matches!(self.form, DefinitionForm::ExplicitString(_)) {
            let inner = body
                .strip_prefix('\'')
                .and_then(|b| b.strip_suffix('\''))
                .ok_or_else(|| Error::Syntax("invalid quoted definition body".into()))?;
            Ok(if inner.contains("''") {
                Cow::Owned(inner.replace("''", "'"))
            } else {
                Cow::Borrowed(inner)
            })
        } else {
            Ok(Cow::Borrowed(body))
        }
    }
}

/// Inspect one input unit. Call again after appending a physical line when
/// NeedMore is returned. Spans refer to the supplied UTF-8 source, never an
/// expanded 9 : string. Arbitrary computed colon operands are not classified.
pub fn frame(source: &str) -> Result<InputFrame> {
    let first_end = source.find('\n').unwrap_or(source.len());
    let first = source[..first_end].trim_end_matches('\r');
    let spans = tokenizer::scan_unfinished(first.as_bytes());
    let visible: Vec<_> = spans
        .into_iter()
        .take_while(|s| {
            let word = &first[s.clone()];
            !word.starts_with("NB.") || word.starts_with("NB..") || word.starts_with("NB.:")
        })
        .collect();
    if let Some(delimiter) = visible
        .iter()
        .find(|s| matches!(&first[(*s).clone()], "{{" | "}}"))
    {
        if &first[delimiter.clone()] == "}}" {
            return Err(Error::Syntax("unmatched }}".into()).at(delimiter.clone()));
        }
    }
    if let Some(open) = visible.iter().find(|s| &first[(*s).clone()] == "{{") {
        return direct(source, open.clone());
    }
    for parts in visible.windows(3) {
        let mode = match &first[parts[0].clone()] {
            "1" => 1,
            "2" => 2,
            "3" => 3,
            "4" => 4,
            _ => continue,
        };
        if &first[parts[1].clone()] != ":" {
            continue;
        }
        let operand = &first[parts[2].clone()];
        if operand == "0" && parts[2] == *visible.last().unwrap() {
            return explicit_block(source, mode, parts[0].start, first_end);
        }
        if operand.starts_with('\'') {
            // The supplied body string may contain LF. Scan the complete
            // input, rather than incorrectly treating its first line as an
            // open quote. A genuinely unfinished quote remains OpenQuote.
            let quoted = tokenizer::scan(source.as_bytes())?
                .into_iter()
                .find(|span| span.start == parts[2].start)
                .ok_or_else(|| Error::Syntax("missing quoted definition body".into()))?;
            return Ok(InputFrame::Definition(DefinitionInput {
                form: DefinitionForm::ExplicitString(mode),
                span: parts[0].start..quoted.end,
                body: quoted,
                nested: Vec::new(),
            }));
        }
    }
    Ok(InputFrame::Sentence)
}

fn direct(source: &str, open: Range<usize>) -> Result<InputFrame> {
    let mut stack = Vec::new();
    let mut nested = Vec::new();
    // wordil keeps NB. comments opaque and permits LF inside a quoted word.
    // Comments stay single opaque spans, including on interior body lines.
    for span in tokenizer::scan_unfinished(source.as_bytes()) {
        let word = &source[span.clone()];
        if span.start < open.start {
            continue;
        }
        match word {
            "{{" => {
                if source[span.end..].starts_with(")") {
                    return Err(
                        Error::Unsupported("tagged direct definition input".into()).at(span)
                    );
                }
                if stack.len() >= crate::semantic::MAX_EXPR_DEPTH {
                    return Err(Error::Limit);
                }
                stack.push(span);
            }
            "}}" => {
                let start = stack
                    .pop()
                    .ok_or_else(|| Error::Syntax("unmatched }}".into()).at(span.clone()))?;
                if stack.is_empty() {
                    // Completed definitions still require well-formed quotes.
                    tokenizer::scan(&source.as_bytes()[open.start..span.end])?;
                    let tail = &source[span.end..];
                    let tail = tail.split('\n').next().unwrap_or(tail);
                    if tokenizer::scan_unfinished(tail.as_bytes())
                        .iter()
                        .any(|word| &tail[word.clone()] == "{{")
                    {
                        return Err(Error::Unsupported(
                            "multiple direct definitions in one sentence".into(),
                        )
                        .at(span));
                    }
                    return Ok(InputFrame::Definition(DefinitionInput {
                        form: DefinitionForm::Direct,
                        span: open.start..span.end,
                        body: open.end..span.start,
                        nested,
                    }));
                }
                nested.push(start.start..span.end);
            }
            _ => {}
        }
    }
    Ok(InputFrame::NeedMore)
}

fn explicit_block(source: &str, mode: u8, start: usize, first_end: usize) -> Result<InputFrame> {
    if first_end == source.len() {
        return Ok(InputFrame::NeedMore);
    }
    let body_start = first_end + 1;
    let mut offset = body_start;
    for line in source[body_start..].split_inclusive('\n') {
        let text = line
            .strip_suffix('\n')
            .unwrap_or(line)
            .strip_suffix('\r')
            .unwrap_or(line.strip_suffix('\n').unwrap_or(line));
        // cx.c::colon0 accepts ASCII spaces, not tabs or trailing comments.
        if text.trim_matches(' ') == ")" {
            let Some(nested) = block_nested(source, body_start, offset)? else {
                offset += line.len();
                continue;
            };
            return Ok(InputFrame::Definition(DefinitionInput {
                form: DefinitionForm::ExplicitBlock(mode),
                span: start..offset + text.len(),
                body: body_start..offset,
                nested,
            }));
        }
        offset += line.len();
    }
    Ok(InputFrame::NeedMore)
}

// colon0 expands/collects nested DDs before testing a physical ) line.
fn block_nested(source: &str, start: usize, end: usize) -> Result<Option<Vec<Range<usize>>>> {
    let mut stack = Vec::new();
    let mut nested = Vec::new();
    for relative in tokenizer::scan_unfinished(&source.as_bytes()[start..end]) {
        let span = start + relative.start..start + relative.end;
        match &source[span.clone()] {
            "{{" => {
                if source[span.end..].starts_with(')') {
                    return Err(
                        Error::Unsupported("tagged direct definition input".into()).at(span)
                    );
                }
                if stack.len() >= crate::semantic::MAX_EXPR_DEPTH {
                    return Err(Error::Limit);
                }
                stack.push(span);
            }
            "}}" => {
                let open = stack
                    .pop()
                    .ok_or_else(|| Error::Syntax("unmatched }}".into()).at(span.clone()))?;
                nested.push(open.start..span.end);
            }
            _ => {}
        }
    }
    Ok(stack.is_empty().then_some(nested))
}

/// Owned physical-line collector for CLI/frontends. push_line accepts a line
/// without its line ending; the collector inserts LF between physical lines.
#[derive(Default)]
pub struct DefinitionCollector {
    source: String,
}
impl DefinitionCollector {
    pub fn push_line(&mut self, line: &str) -> Result<InputFrame> {
        if !self.source.is_empty() {
            self.source.push('\n');
        }
        self.source.push_str(line);
        frame(&self.source)
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn finish(&self) -> Result<InputFrame> {
        match frame(&self.source)? {
            InputFrame::NeedMore => {
                Err(Error::Syntax("unterminated definition input".into()).at(0..self.source.len()))
            }
            other => Ok(other),
        }
    }
}

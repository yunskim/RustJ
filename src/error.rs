use std::{fmt, ops::Range};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Syntax(String),
    Spelling,
    Domain,
    Length,
    Rank,
    Index,
    Value(String),
    Limit,
    OpenQuote,
    Unsupported(String),
    Located {
        error: Box<Error>,
        span: Range<usize>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub byte_span: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub class_name: &'static str,
    pub kind: &'static str,
    pub message: String,
    pub location: Option<SourceLocation>,
}

impl Error {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Syntax(_) => "syntax error",
            Self::Spelling => "spelling error",
            Self::Domain => "domain error",
            Self::Length => "length error",
            Self::Rank => "rank error",
            Self::Index => "index error",
            Self::Value(_) => "value error",
            Self::Limit => "limit error",
            Self::OpenQuote => "open quote",
            Self::Unsupported(_) => "unsupported",
            Self::Located { error, .. } => error.kind(),
        }
    }

    pub fn class_name(&self) -> &'static str {
        match self.root() {
            Self::Syntax(_) => "SyntaxError",
            Self::Spelling => "SpellingError",
            Self::Domain => "DomainError",
            Self::Length => "LengthError",
            Self::Rank => "RankError",
            Self::Index => "IndexError",
            Self::Value(_) => "ValueError",
            Self::Limit => "LimitError",
            Self::OpenQuote => "OpenQuoteError",
            Self::Unsupported(_) => "UnsupportedError",
            Self::Located { .. } => unreachable!(),
        }
    }

    pub fn at(self, span: Range<usize>) -> Self {
        if matches!(self, Self::Located { .. }) {
            self
        } else {
            Self::Located {
                error: Box::new(self),
                span,
            }
        }
    }

    pub fn span(&self) -> Option<&Range<usize>> {
        match self {
            Self::Located { span, .. } => Some(span),
            _ => None,
        }
    }

    pub fn root(&self) -> &Error {
        match self {
            Self::Located { error, .. } => error.root(),
            _ => self,
        }
    }

    pub fn diagnostic(&self, source: &str) -> Diagnostic {
        Diagnostic {
            class_name: self.class_name(),
            kind: self.kind(),
            message: self.detail_message(),
            location: self
                .span()
                .map(|span| SourceLocation::from_span(source, span.clone())),
        }
    }

    fn detail_message(&self) -> String {
        match self.root() {
            Self::Syntax(detail) if !detail.is_empty() => detail.clone(),
            Self::Value(name) if !name.is_empty() => format!("undefined name {name:?}"),
            Self::Unsupported(detail) if !detail.is_empty() => detail.clone(),
            root => root.kind().to_owned(),
        }
    }

    pub fn render(&self, source_name: &str, source: &str, base_line: usize) -> String {
        let diagnostic = self.diagnostic(source);
        let Some(location) = diagnostic.location else {
            return format!("{}: {}", diagnostic.class_name, diagnostic.message);
        };

        let display_line = base_line + location.line.saturating_sub(1);
        let line_start = source[..location.byte_span.start]
            .rfind('\n')
            .map_or(0, |i| i + 1);
        let line_end = source[location.byte_span.start..]
            .find('\n')
            .map_or(source.len(), |i| location.byte_span.start + i);
        let source_line = &source[line_start..line_end];

        let start_column = location.column.max(1);
        let width = if location.line == location.end_line {
            location.end_column.saturating_sub(location.column).max(1)
        } else {
            1
        };
        let marker = format!("{}{}", " ".repeat(start_column - 1), "^".repeat(width));

        format!(
            "  File {source_name:?}, line {display_line}, column {start_column}\n    {source_line}\n    {marker}\n{}: {}",
            diagnostic.class_name, diagnostic.message
        )
    }
}

impl SourceLocation {
    pub fn from_span(source: &str, span: Range<usize>) -> Self {
        let len = source.len();
        let start = floor_char_boundary(source, span.start.min(len));
        let end = ceil_char_boundary(source, span.end.min(len).max(start));
        let (line, column) = line_column(source, start);
        let (end_line, end_column) = line_column(source, end);
        Self {
            line,
            column,
            end_line,
            end_column,
            byte_span: start..end,
        }
    }
}

fn floor_char_boundary(source: &str, mut offset: usize) -> usize {
    while offset > 0 && !source.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn ceil_char_boundary(source: &str, mut offset: usize) -> usize {
    while offset < source.len() && !source.is_char_boundary(offset) {
        offset += 1;
    }
    offset
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() + 1;
    let line_start = prefix.rfind('\n').map_or(0, |i| i + 1);
    let column = source[line_start..offset].chars().count() + 1;
    (line, column)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Located { error, .. } => error.fmt(f),
            _ => {
                write!(f, "{}", self.kind())?;
                match self {
                    Self::Syntax(s) | Self::Value(s) | Self::Unsupported(s) => write!(f, ": {s}"),
                    _ => Ok(()),
                }
            }
        }
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_location_counts_unicode_columns_not_bytes() {
        let source = "a=: '한글' + )";
        let start = source.find(')').unwrap();
        let error = Error::Syntax("unexpected )".into()).at(start..start + 1);
        let d = error.diagnostic(source);
        let loc = d.location.unwrap();
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, source[..start].chars().count() + 1);
    }

    #[test]
    fn python_style_render_contains_file_line_source_caret_and_class() {
        let source = "a=: 1\nb=: + )";
        let start = source.find(')').unwrap();
        let rendered = Error::Syntax("unexpected )".into())
            .at(start..start + 1)
            .render("sample.ijs", source, 1);
        assert!(rendered.contains("File \"sample.ijs\", line 2"));
        assert!(rendered.contains("b=: + )"));
        assert!(rendered.contains("^"));
        assert!(rendered.contains("SyntaxError: unexpected )"));
    }

    #[test]
    fn located_errors_preserve_machine_error_kind() {
        let error = Error::Rank.at(3..4);
        assert_eq!(error.kind(), "rank error");
        assert_eq!(error.class_name(), "RankError");
    }
}

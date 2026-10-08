use std::{fmt, ops::Range, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticPhase {
    WordFormation,
    Enqueue,
    Parse,
    SemanticAnalysis,
    Runtime,
    Lowering,
    PhysicalExecution,
}

impl DiagnosticPhase {
    fn label(self) -> &'static str {
        match self {
            Self::WordFormation => "during word formation",
            Self::Enqueue => "before sentence execution",
            Self::Parse => "during parse",
            Self::SemanticAnalysis => "during semantic analysis",
            Self::Runtime => "during execution",
            Self::Lowering => "during lowering",
            Self::PhysicalExecution => "during physical execution",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticValence {
    Monad,
    Dyad,
}

impl DiagnosticValence {
    fn label(self) -> &'static str {
        match self {
            Self::Monad => "monad",
            Self::Dyad => "dyad",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentRole {
    X,
    Y,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FailureDetail {
    ShapeMismatch {
        x_shape: Vec<usize>,
        y_shape: Vec<usize>,
    },
    InvalidValue {
        role: ArgumentRole,
        position: Option<usize>,
        value: String,
        requirement: String,
    },
    Message(String),
}

impl ArgumentRole {
    fn label(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
        }
    }
}

/// Small, allocation-bounded noun summary for diagnostics.  Never retain or
/// copy the full argument merely to improve an error message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArgumentSummary {
    pub role: ArgumentRole,
    pub type_code: i32,
    pub shape: Vec<usize>,
}

impl ArgumentSummary {
    pub fn rank(&self) -> usize {
        self.shape.len()
    }

    pub fn type_name(&self) -> &'static str {
        match self.type_code {
            1 => "boolean",
            2 => "character",
            4 => "integer",
            8 => "floating",
            32 => "boxed",
            64 => "extended integer",
            128 => "rational",
            code if code >= 1024 => "sparse",
            _ => "unknown",
        }
    }

    fn shape_text(&self) -> String {
        if self.shape.is_empty() {
            "scalar".into()
        } else {
            self.shape
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum DiagnosticFrameKind {
    #[default]
    DefinitionBody,
    DefinitionCall,
    DefinitionAdmission,
    DefinitionReturn,
}

/// Source-owned coordinates, distinct from the current caller's span/index.
/// Ordered from the innermost failure to outer definition callsites.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticSourceFrame {
    pub kind: DiagnosticFrameKind,
    pub origin: crate::source::SourceOrigin,
    pub source: Arc<str>,
    pub definition_span: Range<usize>,
    pub span: Range<usize>,
    /// Index in the failing/calling control fragment's queue, not the entire body.
    pub blame_word_index: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ErrorContext {
    pub phase: Option<DiagnosticPhase>,
    pub span: Option<Range<usize>>,
    /// Original enqueue-word index, when known.  This is deliberately kept
    /// separate from byte spans so the jsource-style parser can preserve its
    /// token-blame semantics through stack reductions.
    pub blame_word_index: Option<usize>,
    pub current_name: Option<String>,
    pub operation: Option<String>,
    pub valence: Option<DiagnosticValence>,
    pub arguments: Vec<ArgumentSummary>,
    pub details: Vec<FailureDetail>,
    pub source_frames: Vec<DiagnosticSourceFrame>,
}

impl ErrorContext {
    pub fn phase(phase: DiagnosticPhase) -> Self {
        Self {
            phase: Some(phase),
            ..Self::default()
        }
    }

    pub fn with_span(mut self, span: Range<usize>) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_blame_word(mut self, word_index: usize) -> Self {
        self.blame_word_index = Some(word_index);
        self
    }

    pub fn executing(mut self, operation: impl Into<String>, valence: DiagnosticValence) -> Self {
        self.operation = Some(operation.into());
        self.valence = Some(valence);
        self
    }

    pub fn with_current_name(mut self, name: impl Into<String>) -> Self {
        self.current_name = Some(name.into());
        self
    }

    pub fn with_argument(mut self, argument: ArgumentSummary) -> Self {
        self.arguments.push(argument);
        self
    }

    pub fn with_detail(mut self, detail: FailureDetail) -> Self {
        self.details.push(detail);
        self
    }

    pub fn with_note(self, note: impl Into<String>) -> Self {
        self.with_detail(FailureDetail::Message(note.into()))
    }

    fn merge_outer(&mut self, outer: ErrorContext) {
        if self.phase.is_none() {
            self.phase = outer.phase;
        }
        if self.span.is_none() {
            self.span = outer.span;
        }
        if self.blame_word_index.is_none() {
            self.blame_word_index = outer.blame_word_index;
        }
        if self.current_name.is_none() {
            self.current_name = outer.current_name;
        }
        if self.operation.is_none() {
            self.operation = outer.operation;
        }
        if self.valence.is_none() {
            self.valence = outer.valence;
        }
        if self.arguments.is_empty() {
            self.arguments = outer.arguments;
        }
        self.details.extend(outer.details);
        self.source_frames.extend(outer.source_frames);
    }
}

/// Failure ownership, independent of diagnostic phase or a particular route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureCategory {
    JLanguage,
    UnsupportedCapability,
    VerifierDefect,
    BackendFailure,
}
impl FailureCategory {
    pub fn name(self) -> &'static str {
        match self {
            Self::JLanguage => "j-language",
            Self::UnsupportedCapability => "unsupported-capability",
            Self::VerifierDefect => "verifier-defect",
            Self::BackendFailure => "backend-failure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Syntax(String),
    Spelling,
    IllFormedName,
    IllFormedNumber,
    Domain,
    Length,
    Rank,
    Valence,
    Control,
    NounResult,
    ReadOnly,
    Index,
    Value(String),
    Limit,
    OpenQuote,
    Unsupported(String),
    /// Invalid compiler-owned representation; never a source-language error.
    Verification(String),
    /// An admitted implementation failed; not a route capability miss.
    Backend(String),
    /// J error classification plus diagnostic provenance/semantic context.
    /// Stable machine APIs strip this wrapper before returning.
    Context {
        error: Box<Error>,
        context: Box<ErrorContext>,
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
    pub context: ErrorContext,
    pub explanations: Vec<String>,
}

impl Error {
    pub fn category(&self) -> FailureCategory {
        match self.root() {
            Self::Unsupported(_) => FailureCategory::UnsupportedCapability,
            Self::Verification(_) => FailureCategory::VerifierDefect,
            Self::Backend(_) => FailureCategory::BackendFailure,
            _ => FailureCategory::JLanguage,
        }
    }

    /// Only for failures raised during actual J execution. Read-only admission
    /// results must not enter a J handler, even when their error class is J-like.
    pub fn is_j_catchable(&self) -> bool {
        self.category() == FailureCategory::JLanguage
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Syntax(_) => "syntax error",
            Self::Spelling => "spelling error",
            Self::IllFormedName => "ill-formed name",
            Self::IllFormedNumber => "ill-formed number",
            Self::Domain => "domain error",
            Self::Length => "length error",
            Self::Rank => "rank error",
            Self::Valence => "valence error",
            Self::Control => "control error",
            Self::NounResult => "noun result was required",
            Self::ReadOnly => "read-only data",
            Self::Index => "index error",
            Self::Value(_) => "value error",
            Self::Limit => "limit error",
            Self::OpenQuote => "open quote",
            Self::Unsupported(_) => "unsupported",
            Self::Verification(_) => "verifier failure",
            Self::Backend(_) => "backend failure",
            Self::Context { error, .. } => error.kind(),
        }
    }

    pub fn class_name(&self) -> &'static str {
        match self.root() {
            Self::Syntax(_) => "SyntaxError",
            Self::Spelling => "SpellingError",
            Self::IllFormedName => "IllFormedNameError",
            Self::IllFormedNumber => "IllFormedNumberError",
            Self::Domain => "DomainError",
            Self::Length => "LengthError",
            Self::Rank => "RankError",
            Self::Valence => "ValenceError",
            Self::Control => "ControlError",
            Self::NounResult => "NounResultError",
            Self::ReadOnly => "ReadOnlyError",
            Self::Index => "IndexError",
            Self::Value(_) => "ValueError",
            Self::Limit => "LimitError",
            Self::OpenQuote => "OpenQuoteError",
            Self::Unsupported(_) => "UnsupportedError",
            Self::Verification(_) => "VerificationError",
            Self::Backend(_) => "BackendError",
            Self::Context { .. } => unreachable!(),
        }
    }

    pub fn with_context(self, outer: ErrorContext) -> Self {
        match self {
            Self::Context { error, mut context } => {
                context.merge_outer(outer);
                Self::Context { error, context }
            }
            error => Self::Context {
                error: Box::new(error),
                context: Box::new(outer),
            },
        }
    }

    pub fn at(self, span: Range<usize>) -> Self {
        self.with_context(ErrorContext::default().with_span(span))
    }

    pub fn in_phase(self, phase: DiagnosticPhase) -> Self {
        self.with_context(ErrorContext::phase(phase))
    }

    pub fn blamed_on_word(self, word_index: usize) -> Self {
        self.with_context(ErrorContext::default().with_blame_word(word_index))
    }

    pub fn span(&self) -> Option<&Range<usize>> {
        self.context().and_then(|context| context.span.as_ref())
    }

    pub fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::Context { context, .. } => Some(context),
            _ => None,
        }
    }

    pub fn into_unlocated(self) -> Error {
        match self {
            Self::Context { error, .. } => error.into_unlocated(),
            other => other,
        }
    }

    pub fn root(&self) -> &Error {
        match self {
            Self::Context { error, .. } => error.root(),
            _ => self,
        }
    }

    pub fn diagnostic(&self, source: &str) -> Diagnostic {
        DiagnosticAnalyzer::analyze(self, source)
    }

    fn detail_message(&self) -> String {
        match self.root() {
            Self::Syntax(detail) if !detail.is_empty() => detail.clone(),
            Self::Value(name) if !name.is_empty() => format!("undefined name {name:?}"),
            Self::Unsupported(detail) | Self::Verification(detail) | Self::Backend(detail)
                if !detail.is_empty() =>
            {
                detail.clone()
            }
            root => root.kind().to_owned(),
        }
    }

    pub fn render(&self, source_name: &str, source: &str, base_line: usize) -> String {
        let diagnostic = self.diagnostic(source);
        let mut out = String::new();

        for frame in &diagnostic.context.source_frames {
            let root_span = frame.origin.root_span(frame.span.clone());
            let (frame_source, frame_name, span) = match root_span {
                Some(span) => (frame.origin.unit().text(), frame.origin.unit().name(), span),
                None => (frame.source.as_ref(), "<input>", frame.span.clone()),
            };
            let location = SourceLocation::from_span(frame_source, span);
            let begin = frame_source[..location.byte_span.start]
                .rfind('\n')
                .map_or(0, |n| n + 1);
            let end = frame_source[location.byte_span.start..]
                .find('\n')
                .map_or(frame_source.len(), |n| location.byte_span.start + n);
            let label = match frame.kind {
                DiagnosticFrameKind::DefinitionBody => "definition failure",
                DiagnosticFrameKind::DefinitionCall => "called from definition",
                DiagnosticFrameKind::DefinitionAdmission => "before definition execution",
                DiagnosticFrameKind::DefinitionReturn => "returning from definition",
            };
            out.push_str(&format!(
                "  {label} in {frame_name}, line {}, column {}\n    {}\n    {}{}\n",
                location.line,
                location.column,
                &frame_source[begin..end],
                " ".repeat(location.column.saturating_sub(1)),
                "^".repeat(if location.line == location.end_line {
                    location.end_column.saturating_sub(location.column).max(1)
                } else {
                    1
                })
            ));
        }

        if let Some(location) = &diagnostic.location {
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
            out.push_str(&format!(
                "  File {source_name:?}, line {display_line}, column {start_column}\n    {source_line}\n    {marker}\n"
            ));
        }

        out.push_str(&format!(
            "{}: {}",
            diagnostic.class_name, diagnostic.message
        ));

        if let Some(phase) = diagnostic.context.phase {
            out.push_str(&format!("\n  {}", phase.label()));
        }
        if let Some(operation) = &diagnostic.context.operation {
            if let Some(valence) = diagnostic.context.valence {
                out.push_str(&format!(
                    "\n  while executing {} {}",
                    valence.label(),
                    operation
                ));
            } else {
                out.push_str(&format!("\n  while executing {operation}"));
            }
        }
        if let Some(name) = &diagnostic.context.current_name {
            out.push_str(&format!("\n  in {name}"));
        }
        for argument in &diagnostic.context.arguments {
            out.push_str(&format!(
                "\n  {}: {}, rank {}, shape {}",
                argument.role.label(),
                argument.type_name(),
                argument.rank(),
                argument.shape_text()
            ));
        }
        for explanation in &diagnostic.explanations {
            out.push_str(&format!("\n  {explanation}"));
        }
        out
    }
}

/// Converts a J-compatible error plus structured execution context into a
/// human-oriented explanation.  Keep this separate from the renderer so new
/// J/eformat-style semantic analyzers can be added without changing error
/// propagation or UI formatting.
pub struct DiagnosticAnalyzer;

impl DiagnosticAnalyzer {
    pub fn analyze(error: &Error, source: &str) -> Diagnostic {
        let context = error.context().cloned().unwrap_or_default();
        let mut explanations = context
            .details
            .iter()
            .map(Self::explain_detail)
            .collect::<Vec<_>>();

        // Conservative generic explanation only when the failure site did not
        // provide a more precise structured detail.
        if error.kind() == "length error" && context.arguments.len() == 2 && explanations.is_empty()
        {
            let x = &context.arguments[0];
            let y = &context.arguments[1];
            explanations.push(format!(
                "shapes {} and {} do not conform",
                x.shape_text(),
                y.shape_text()
            ));
        }

        Diagnostic {
            class_name: error.class_name(),
            kind: error.kind(),
            message: error.detail_message(),
            location: context
                .span
                .clone()
                .map(|span| SourceLocation::from_span(source, span)),
            context,
            explanations,
        }
    }

    fn explain_detail(detail: &FailureDetail) -> String {
        match detail {
            FailureDetail::ShapeMismatch { x_shape, y_shape } => format!(
                "shapes {} and {} do not conform",
                shape_text(x_shape),
                shape_text(y_shape)
            ),
            FailureDetail::InvalidValue {
                role,
                position,
                value,
                requirement,
            } => {
                let where_at = position
                    .map(|p| format!(" at position {p}"))
                    .unwrap_or_default();
                format!(
                    "{} has invalid value ({value}){where_at}; {requirement}",
                    role.label()
                )
            }
            FailureDetail::Message(message) => message.clone(),
        }
    }
}

fn shape_text(shape: &[usize]) -> String {
    if shape.is_empty() {
        "scalar".into()
    } else {
        shape
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(" ")
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
            Self::Context { error, .. } => error.fmt(f),
            _ => {
                write!(f, "{}", self.kind())?;
                match self {
                    Self::Syntax(s)
                    | Self::Value(s)
                    | Self::Unsupported(s)
                    | Self::Verification(s)
                    | Self::Backend(s) => write!(f, ": {s}"),
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
        let error = Error::Syntax("unexpected )".into())
            .with_context(ErrorContext::phase(DiagnosticPhase::Parse).with_span(start..start + 1));
        let d = error.diagnostic(source);
        let loc = d.location.unwrap();
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, source[..start].chars().count() + 1);
    }

    #[test]
    fn outer_context_never_overwrites_more_precise_inner_context() {
        let error = Error::Rank
            .with_context(
                ErrorContext::phase(DiagnosticPhase::Runtime)
                    .with_span(4..5)
                    .with_blame_word(2),
            )
            .with_context(
                ErrorContext::phase(DiagnosticPhase::SemanticAnalysis)
                    .with_span(0..9)
                    .with_blame_word(0),
            );
        let context = error.context().unwrap();
        assert_eq!(context.phase, Some(DiagnosticPhase::Runtime));
        assert_eq!(context.span, Some(4..5));
        assert_eq!(context.blame_word_index, Some(2));
    }

    #[test]
    fn semantic_context_renders_execution_and_argument_summaries() {
        let source = "2 3 + 4 5 6";
        let error = Error::Length.with_context(
            ErrorContext::phase(DiagnosticPhase::Runtime)
                .with_span(4..5)
                .executing("+", DiagnosticValence::Dyad)
                .with_argument(ArgumentSummary {
                    role: ArgumentRole::X,
                    type_code: 4,
                    shape: vec![2],
                })
                .with_argument(ArgumentSummary {
                    role: ArgumentRole::Y,
                    type_code: 4,
                    shape: vec![3],
                })
                .with_note("shapes 2 and 3 do not conform"),
        );
        let rendered = error.render("<test>", source, 1);
        assert!(rendered.contains("LengthError"));
        assert!(rendered.contains("while executing dyad +"));
        assert!(rendered.contains("x: integer, rank 1, shape 2"));
        assert!(rendered.contains("shapes 2 and 3 do not conform"));
    }

    #[test]
    fn into_unlocated_preserves_machine_error_variant() {
        assert_eq!(
            Error::Rank
                .with_context(ErrorContext::phase(DiagnosticPhase::Runtime).with_span(3..4))
                .into_unlocated(),
            Error::Rank
        );
    }
}

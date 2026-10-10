//! Read-only stage admission, distinct from J execution and route dispatch.
use crate::{Error, Result, error::FailureCategory};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Frontend,
    FrontendHandoff,
    SemanticBinding,
    JGraph,
    Logical,
    NameEffects,
    NameArrays,
}

impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Self::Frontend => "frontend",
            Self::FrontendHandoff => "frontend-handoff",
            Self::SemanticBinding => "semantic-binding",
            Self::JGraph => "j-graph",
            Self::Logical => "logical",
            Self::NameEffects => "name-effects",
            Self::NameArrays => "name-arrays",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Rejection {
    stage: Stage,
    error: Error,
    frontend_prefix: Option<std::sync::Arc<crate::frontend_context::FrontendContext>>,
}

impl Rejection {
    /// Requested inspection boundary, not a claim about the originating phase.
    pub fn stage(&self) -> Stage {
        self.stage
    }
    pub fn category(&self) -> FailureCategory {
        self.error.category()
    }
    pub fn error(&self) -> &Error {
        &self.error
    }
    pub fn into_error(self) -> Error {
        self.error
    }
    pub fn frontend_prefix(
        &self,
    ) -> Option<&std::sync::Arc<crate::frontend_context::FrontendContext>> {
        self.frontend_prefix.as_ref()
    }

    /// Inspection has not executed effects. Another read-only analysis may be
    /// attempted for a capability miss. This never authorizes execution/replay.
    pub fn may_inspect_another_route(&self) -> bool {
        self.category() == FailureCategory::UnsupportedCapability
    }

    /// Even a J-class error found by analysis has not been raised by J execution.
    pub fn is_j_handler_eligible(&self) -> bool {
        false
    }
}

/// Success admits only this stage's representation. It is neither an executable
/// lease nor evidence that later stages, targets or mutable bindings are valid.
#[derive(Clone, Debug)]
pub struct Admission<T> {
    stage: Stage,
    result: std::result::Result<T, Rejection>,
}

impl<T> Admission<T> {
    pub(crate) fn inspected(stage: Stage, result: Result<T>) -> Self {
        Self {
            stage,
            result: result.map_err(|error| Rejection {
                stage,
                error,
                frontend_prefix: None,
            }),
        }
    }
    pub(crate) fn parsed(
        stage: Stage,
        result: std::result::Result<
            crate::semantic::Program,
            crate::frontend_context::FrontendFailure,
        >,
        lower: impl FnOnce(crate::semantic::Program) -> Result<T>,
    ) -> Self {
        match result {
            Ok(program) => Self::inspected(stage, lower(program)),
            Err(failure) => Self {
                stage,
                result: Err(Rejection {
                    stage,
                    error: failure.error,
                    frontend_prefix: Some(std::sync::Arc::new(*failure.context)),
                }),
            },
        }
    }
    pub fn stage(&self) -> Stage {
        self.stage
    }
    pub fn result(&self) -> std::result::Result<&T, &Rejection> {
        self.result.as_ref()
    }
    pub fn into_result(self) -> std::result::Result<T, Rejection> {
        self.result
    }
}

impl Admission<crate::semantic::Program> {
    pub(crate) fn frontend(
        result: std::result::Result<
            crate::semantic::Program,
            crate::frontend_context::FrontendFailure,
        >,
    ) -> Self {
        Self::parsed(Stage::Frontend, result, Ok)
    }
}

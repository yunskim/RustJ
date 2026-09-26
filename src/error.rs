use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Syntax(String),
    Domain,
    Length,
    Rank,
    Index,
    Value(String),
    Limit,
    Unsupported(String),
}

impl Error {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Syntax(_) => "syntax error",
            Self::Domain => "domain error",
            Self::Length => "length error",
            Self::Rank => "rank error",
            Self::Index => "index error",
            Self::Value(_) => "value error",
            Self::Limit => "limit error",
            Self::Unsupported(_) => "unsupported",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind())?;
        match self {
            Self::Syntax(s) | Self::Value(s) | Self::Unsupported(s) => write!(f, ": {s}"),
            _ => Ok(()),
        }
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

//! Error types for regex parsing.

use parserc::{ControlFlow, Kind, ParseError, Span};

/// Kind of parsing `regular expressions` error.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CompileError {
    #[error("whitespaces")]
    S,
    #[error("digits")]
    Digits,
    #[error("repeat")]
    Repeat,
    #[error("token")]
    Token,
    #[error("escape")]
    Escape,
    #[error("escape hexidecimal number")]
    EscapeHex,
    #[error("escape unicode")]
    EscapeUnicode,
    #[error("character sequence")]
    CharSequence,
    #[error("character range")]
    CharRange,
    #[error("character class")]
    CharClass,
    #[error("pattern char sequence.")]
    PatternChars,
}

impl CompileError {
    /// Map underlying error into `PatternKind`.
    pub fn map(self) -> impl FnOnce(RegexError) -> RegexError {
        |err: RegexError| RegexError::Compile(self, err.control_flow(), err.to_span())
    }

    /// Map underlying error into `PatternKind` fatal error.
    pub fn map_fatal(self) -> impl FnOnce(RegexError) -> RegexError {
        |err: RegexError| RegexError::Compile(self, ControlFlow::Fatal, err.to_span())
    }
}

/// Error type returns by `regular expressions` parser.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RegexError {
    /// Unhandle error kind.
    #[error("{0:?}")]
    Other(#[from] Kind),
    /// Identified parsing errors
    #[error("failed to parsing `{0:?}`, {1:?}, {2:?}")]
    Compile(CompileError, ControlFlow, Span),
}

impl ParseError for RegexError {
    fn to_span(&self) -> Span {
        match self {
            RegexError::Other(kind) => kind.to_span(),
            RegexError::Compile(_, _, span) => span.clone(),
        }
    }

    fn control_flow(&self) -> ControlFlow {
        match self {
            RegexError::Other(kind) => kind.control_flow(),
            RegexError::Compile(_, control_flow, _) => *control_flow,
        }
    }

    fn into_fatal(self) -> Self {
        match self {
            RegexError::Other(kind) => RegexError::Other(kind.into_fatal()),
            RegexError::Compile(kind, _, span) => {
                RegexError::Compile(kind, ControlFlow::Fatal, span)
            }
        }
    }
}

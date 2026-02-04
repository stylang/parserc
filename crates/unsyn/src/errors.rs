//! The types used for `unsyn` parsing error reports.

use parserc::{ControlFlow, ParseError, Span};

/// Error for punct tokens.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PunctKind {
    #[error("punct ';'")]
    Semi,
    #[error("punct '")]
    SingleQuote,
    #[error("punct '+'")]
    Plus,
    #[error("punct '*'")]
    Star,
    #[error("punct '?'")]
    Question,
    #[error("punct '～'")]
    Tilde,
    #[error("punct '|'")]
    Or,
    #[error("punct '('")]
    ParenStart,
    #[error("punct ')'")]
    ParenEnd,
    #[error("punct '['")]
    BracketStart,
    #[error("punct ']'")]
    BracketEnd,
    #[error("punct '{{'")]
    BraceStart,
    #[error("punct '}}'")]
    BraceEnd,
    #[error("punct '<'")]
    Lt,
    #[error("punct '>'")]
    Gt,
    #[error("punct '::'")]
    PathSep,
    #[error("punct ','")]
    Comma,
    #[error("punct '->'")]
    ArrowRight,
    #[error("punct '..'")]
    DotDot,
    #[error("punct '-'")]
    Minus,
}

impl PunctKind {
    /// Map error to `punct` error.
    #[inline]
    pub fn map(self) -> impl FnOnce(UnsynError) -> UnsynError {
        |err: UnsynError| UnsynError::Punct(self, err.control_flow(), err.to_span())
    }
}

/// Error for keyword tokens.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum KeywordKind {
    #[error("keyword 'lexer'")]
    Lexer,
    #[error("keyword 'syntax'")]
    Syntax,
    #[error("keyword 'followed'")]
    Followed,
    #[error("keyword 'except'")]
    Except,
    #[error("keyword 'use'")]
    Use,
    #[error("keyword 'super'")]
    Super,
    #[error("keyword 'crate'")]
    Crate,
    #[error("keyword 'as'")]
    As,
    #[error("keyword 'this'")]
    This,
    #[error("keyword 'whitespace'")]
    Whitespace,
    #[error("keyword 'mod'")]
    Mod,
    #[error("keyword 'concat'")]
    Concat,
}

impl KeywordKind {
    /// Map error to `punct` error.
    #[inline]
    pub fn map(self) -> impl FnOnce(UnsynError) -> UnsynError {
        |err: UnsynError| UnsynError::Keyword(self, err.control_flow(), err.to_span())
    }
}

/// Error for syntax tree.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SyntaxKind {
    #[error("unicode literal")]
    Unicode,
    #[error("single quote")]
    QuoteEscape,
    #[error("ascii escape")]
    ASCIIEscape,
    #[error("string content")]
    StrContent,
    #[error("ident")]
    Ident,
    #[error("line comment")]
    LineComment,
    #[error("block comment")]
    BlockComment,
    #[error("outer line document")]
    OuterLineDoc,
    #[error("inner block document")]
    InnerBlockDoc,
    #[error("inner line document")]
    InnerLineDoc,
    #[error("outer block document")]
    OuterBlockDoc,
    #[error("use tree")]
    UseTree,
    #[error("unicode escape")]
    UnicodeEscape,
    #[error("literal decimal number")]
    Dec,
    #[error("ExprNoTopAlt")]
    ExprNoTopAlt,
}

impl SyntaxKind {
    /// Map error to `syntax` error.
    #[inline]
    pub fn map(self) -> impl FnOnce(UnsynError) -> UnsynError {
        |err: UnsynError| UnsynError::Syntax(self, err.control_flow(), err.to_span())
    }

    /// Map error to `syntax` fatal error.
    #[inline]
    pub fn map_into_fatal(self) -> impl FnOnce(UnsynError) -> UnsynError {
        |err: UnsynError| UnsynError::Syntax(self, ControlFlow::Fatal, err.to_span())
    }
}

/// Error for semantic check.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SemanticsKind {
    #[error("unicode literal")]
    Unicode,
    #[error("7bit char escape")]
    Char7BitEscapeTooShort,
    #[error("7bit char escape out of range")]
    Char7BitEscapeOutOfRange,
    #[error("7bit char escape content")]
    HexDigit,
    #[error("string content")]
    StrContent,
    #[error("keyword")]
    Keyword,
    #[error("unicode escape")]
    UnicodeEscape,
    #[error("empty set expression")]
    EmptySet,
    #[error("invalid set item")]
    SetItem,
}

impl SemanticsKind {
    /// Map error to `semantic` error.
    #[inline]
    pub fn map(self) -> impl FnOnce(UnsynError) -> UnsynError {
        |err: UnsynError| UnsynError::Semantics(self, err.to_span())
    }
}

/// Error information container for `unsyn` parsing.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UnsynError {
    /// Unhandle parserc `Errors`.
    #[error(transparent)]
    Kind(#[from] parserc::Kind),

    /// Reports a lexer error.
    #[error("punct error: expect {0}, {1:?},{2:?}")]
    Punct(PunctKind, ControlFlow, Span),

    /// Expect a keyword
    #[error("keyword error: expect {0}, {1:?},{2:?}")]
    Keyword(KeywordKind, ControlFlow, Span),

    /// Reports a syntax error.
    #[error("syntax error: expect {0}, {1:?},{2:?}")]
    Syntax(SyntaxKind, ControlFlow, Span),

    /// Reports a semantics error
    #[error("unexpect/invalid: {0}, {1:?}")]
    Semantics(SemanticsKind, Span),
}

impl ParseError for UnsynError {
    #[inline]
    fn to_span(&self) -> Span {
        match self {
            UnsynError::Kind(kind) => kind.to_span(),
            UnsynError::Syntax(_, _, span) => span.clone(),
            UnsynError::Punct(_, _, span) => span.clone(),
            UnsynError::Keyword(_, _, span) => span.clone(),
            UnsynError::Semantics(_, span) => span.clone(),
        }
    }

    #[inline]
    fn control_flow(&self) -> parserc::ControlFlow {
        match self {
            UnsynError::Kind(kind) => kind.control_flow(),
            UnsynError::Syntax(_, control_flow, _) => *control_flow,
            UnsynError::Punct(_, control_flow, _) => *control_flow,
            UnsynError::Keyword(_, control_flow, _) => *control_flow,
            UnsynError::Semantics(_, _) => ControlFlow::Fatal,
        }
    }

    #[inline]
    fn into_fatal(self) -> Self {
        match self {
            UnsynError::Kind(kind) => Self::Kind(kind.into_fatal()),
            UnsynError::Syntax(syntax_kind, _, span) => {
                Self::Syntax(syntax_kind, ControlFlow::Fatal, span)
            }
            UnsynError::Punct(punct_kind, _, span) => {
                UnsynError::Punct(punct_kind, ControlFlow::Fatal, span)
            }
            UnsynError::Keyword(punct_kind, _, span) => {
                UnsynError::Keyword(punct_kind, ControlFlow::Fatal, span)
            }
            UnsynError::Semantics(semantics_kind, span) => {
                UnsynError::Semantics(semantics_kind, span)
            }
        }
    }
}

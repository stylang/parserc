//! The types used for `unsyn` parsing error reports.
//!
//! An error says which stage of [`unsyn`](crate) met the problem, what it was
//! looking for, and how far the parser may still go:
//!
//! - [`PunctKind`]: a punctuation token, [`KeywordKind`]: a keyword;
//! - [`SyntaxKind`]: a composite token or node built out of them;
//! - [`SemanticsKind`]: a token that is well formed but meaningless, and every
//!   rule the grammar itself cannot state (see [`crate::semantics`]).
//!
//! Everything but [`UnsynError::Semantics`] also carries a
//! [`ControlFlow`]: [`Recovable`](ControlFlow::Recovable) means an enclosing
//! backtracking parser may try another alternative, [`Fatal`](ControlFlow::Fatal)
//! means the file is malformed and no alternative can help.

use parserc::{ControlFlow, ParseError, Span};

/// Error for punct tokens.
///
/// Every variant names what the parser expected at the position it stopped on;
/// [`PunctKind::map`] turns the error the underlying keyword parser produced
/// into a [`UnsynError::Punct`] that carries this expectation.
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
    #[error("punct '~'")]
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
///
/// A keyword is matched as a literal prefix, so this family also reports the
/// case where a keyword is only the head of a longer word: `lexerX` is an
/// identifier, and `lexer` inside it is not a keyword.
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
    #[error("keyword 'whitespace'")]
    Whitespace,
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

/// Error for a construct made of several tokens.
///
/// This is what [`crate::syntax`] and the composite tokens of
/// [`crate::lexical`] report: literals, comments, documentation, identifiers and
/// the expression forms of a node definition. The one exception is
/// [`SyntaxKind::Eof`], which a whole file reports when it cannot be read to its
/// end.
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
    #[error("unicode escape")]
    UnicodeEscape,
    #[error("literal decimal number")]
    Dec,
    #[error("expression")]
    ExprNoTopAlt,
    /// The end of a file: whatever an item cannot read is left over, and the
    /// construct that contains it reports this instead of handing the rest to
    /// its caller.
    #[error("end of file")]
    Eof,
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

/// Error for values that are well formed but meaningless.
///
/// Two groups live here. The first one is produced while *lexing*: a token can
/// have a legal shape and still denote nothing, as `\x8F` (a two-digit escape
/// outside the 7-bit range) or `U+D800` (a surrogate). The second one is
/// produced while [analyzing](crate::semantics) a whole file, where a name, a
/// range or a repetition can only be judged against the rest of the program.
///
/// The variants that concern one named node carry its name, so that a report
/// reads `undefined symbol `MISSING`` rather than pointing at a bare span.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SemanticsKind {
    #[error("unicode code point literal of four to six hex digits")]
    Unicode,
    #[error("7-bit character escape of exactly two hex digits")]
    Char7BitEscapeTooShort,
    #[error("7-bit character escape out of the `0x00..=0x7F` range")]
    Char7BitEscapeOutOfRange,
    #[error("hex digit")]
    HexDigit,
    #[error("string content")]
    StrContent,
    /// A name that is a keyword of the language can not be used as a node name.
    #[error("identifier is a keyword")]
    Keyword,
    #[error("unicode escape")]
    UnicodeEscape,
    #[error("empty set expression")]
    EmptySet,
    #[error("undefined symbol `{0}`")]
    UndefinedSymbol(String),
    #[error("name collision with the previous definition at {0:?}")]
    NameCollision(Span),
    #[error("duplicate `whitespace` declaration")]
    DuplicateWhitespace,
    #[error("`syntax` node `{0}` can not be used by a terminal pattern")]
    UnexpectedSyntax(String),
    #[error("invalid range: the lower bound is greater than the upper bound")]
    InvalidRange,
    #[error("invalid repetition range")]
    InvalidRepeat,
    #[error("not a valid unicode code point")]
    InvalidCodePoint,
    #[error("expect a single character literal")]
    NotAChar,
    #[error("left-recursive definition of node `{0}`")]
    LeftRecursion(String),
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
    /// Unhandled error produced by the [`parserc`] combinators.
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

#[cfg(test)]
mod tests {
    use parserc::{ControlFlow, Kind, ParseError, Span};

    use super::*;

    /// The rendering of an error is what a user of the DSL reads, so the
    /// wording of every family is pinned here.
    #[test]
    fn every_family_names_what_was_expected() {
        assert_eq!(
            UnsynError::Punct(PunctKind::Semi, ControlFlow::Fatal, Span::Range(3..4)).to_string(),
            "punct error: expect punct ';', Fatal,Range(3..4)"
        );

        assert_eq!(
            UnsynError::Keyword(
                KeywordKind::Lexer,
                ControlFlow::Recovable,
                Span::Range(0..5)
            )
            .to_string(),
            "keyword error: expect keyword 'lexer', Recovable,Range(0..5)"
        );

        assert_eq!(
            UnsynError::Syntax(SyntaxKind::Ident, ControlFlow::Fatal, Span::None).to_string(),
            "syntax error: expect ident, Fatal,None"
        );

        assert_eq!(
            UnsynError::Semantics(
                SemanticsKind::UndefinedSymbol("MISSING".into()),
                Span::Range(11..18),
            )
            .to_string(),
            "unexpect/invalid: undefined symbol `MISSING`, Range(11..18)"
        );
    }

    #[test]
    fn an_unhandled_combinator_error_is_passed_through() {
        let error = UnsynError::Kind(Kind::NextIf(ControlFlow::Recovable, Span::Range(0..1)));

        // `transparent` keeps the message of the combinator, and so it keeps the
        // code and the span it carried.
        assert_eq!(error.to_string(), "Error from `next_if` combinator");
        assert_eq!(error.control_flow(), ControlFlow::Recovable);
        assert_eq!(error.to_span(), Span::Range(0..1));
        assert!(!error.is_fatal());
    }

    #[test]
    fn a_semantic_error_can_not_be_backtracked_over() {
        let span = Span::Range(4..6);

        assert_eq!(
            UnsynError::Semantics(SemanticsKind::EmptySet, span.clone()).control_flow(),
            ControlFlow::Fatal
        );
        assert!(UnsynError::Semantics(SemanticsKind::EmptySet, span.clone()).is_fatal());

        // Turning it fatal changes nothing but must not lose it either.
        assert_eq!(
            UnsynError::Semantics(SemanticsKind::EmptySet, span.clone()).into_fatal(),
            UnsynError::Semantics(SemanticsKind::EmptySet, span)
        );
    }

    #[test]
    fn becoming_fatal_keeps_the_kind_and_the_span() {
        let span = Span::Range(2..3);

        assert_eq!(
            UnsynError::Punct(PunctKind::Or, ControlFlow::Recovable, span.clone()).into_fatal(),
            UnsynError::Punct(PunctKind::Or, ControlFlow::Fatal, span.clone())
        );

        assert_eq!(
            UnsynError::Keyword(KeywordKind::Except, ControlFlow::Recovable, span.clone())
                .into_fatal(),
            UnsynError::Keyword(KeywordKind::Except, ControlFlow::Fatal, span.clone())
        );

        assert_eq!(
            UnsynError::Syntax(SyntaxKind::Dec, ControlFlow::Incomplete, span.clone()).into_fatal(),
            UnsynError::Syntax(SyntaxKind::Dec, ControlFlow::Fatal, span.clone())
        );

        assert_eq!(
            UnsynError::Kind(Kind::Keyword(ControlFlow::Recovable, span.clone())).into_fatal(),
            UnsynError::Kind(Kind::Keyword(ControlFlow::Fatal, span))
        );
    }

    #[test]
    fn mapping_relabels_the_error_of_a_combinator() {
        // The token that owns a failed parser reports what *it* expected, at the
        // place where the combinator stopped.
        let inner = UnsynError::Kind(Kind::Keyword(ControlFlow::Recovable, Span::Range(2..3)));

        assert_eq!(
            PunctKind::Semi.map()(inner),
            UnsynError::Punct(PunctKind::Semi, ControlFlow::Recovable, Span::Range(2..3))
        );

        let inner = UnsynError::Kind(Kind::Keyword(ControlFlow::Recovable, Span::Range(2..3)));

        assert_eq!(
            KeywordKind::Whitespace.map()(inner),
            UnsynError::Keyword(
                KeywordKind::Whitespace,
                ControlFlow::Recovable,
                Span::Range(2..3)
            )
        );

        let inner = UnsynError::Kind(Kind::Keyword(ControlFlow::Recovable, Span::Range(2..3)));

        assert_eq!(
            SyntaxKind::Ident.map()(inner),
            UnsynError::Syntax(SyntaxKind::Ident, ControlFlow::Recovable, Span::Range(2..3))
        );

        // The fatal variant overrides the code instead of keeping it.
        let inner = UnsynError::Kind(Kind::Keyword(ControlFlow::Recovable, Span::Range(2..3)));

        assert_eq!(
            SyntaxKind::Ident.map_into_fatal()(inner),
            UnsynError::Syntax(SyntaxKind::Ident, ControlFlow::Fatal, Span::Range(2..3))
        );

        // A semantic error has no code, so only the span travels over.
        let inner = UnsynError::Kind(Kind::Keyword(ControlFlow::Recovable, Span::Range(2..3)));

        assert_eq!(
            SemanticsKind::EmptySet.map()(inner),
            UnsynError::Semantics(SemanticsKind::EmptySet, Span::Range(2..3))
        );
    }
}

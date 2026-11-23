//! The types used for `unsyn` diagnostic reports.
//!
//! Every report says which stage of [`unsyn`](crate) met the problem and which
//! of two channels it belongs to:
//!
//! - an [`UnsynError`] means the file denotes nothing, and the analysis of it
//!   ends with its diagnostics rather than with a model;
//! - a [`Lint`] means the file denotes something it need not have said twice,
//!   and the analysis produces the model of it all the same.
//!
//! [`Diagnostics`] is the pair of lists a pass writes both kinds of finding to.
//!
//! An error also says what the stage was looking for, and how far the parser may
//! still go:
//!
//! - [`PunctKind`]: a punctuation token, [`KeywordKind`]: a keyword;
//! - [`SyntaxKind`]: a composite token or node built out of them;
//! - [`SemanticsKind`]: a token that is well formed but meaningless, and every
//!   rule the grammar itself cannot state (see [`crate::compiler::semantic`]).
//!
//! Everything but [`UnsynError::Semantics`] carries a
//! [`ControlFlow`]: [`Recovable`](ControlFlow::Recovable) means an enclosing
//! backtracking parser may try another alternative, [`Fatal`](ControlFlow::Fatal)
//! means the file is malformed and no alternative can help.

use parserc::{ControlFlow, ParseError, Span};

/// Error for punct tokens.
///
/// Every variant names what the parser expected at the position it stopped on;
/// [`PunctKind::map`] turns the error the underlying keyword parser produced
/// into a [`UnsynError::Punct`] that carries this expectation.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone, Copy)]
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
    #[error("punct '-'")]
    Minus,
}

impl PunctKind {
    /// Map error to `punct` error.
    #[inline]
    pub fn map(self) -> impl FnOnce(UnsynError) -> UnsynError {
        move |err: UnsynError| UnsynError::Punct(self, err.control_flow(), err.to_span())
    }
}

/// Error for keyword tokens.
///
/// A keyword is matched as a literal prefix, so this family also reports the
/// case where a keyword is only the head of a longer word: `lexerX` is an
/// identifier, and `lexer` inside it is not a keyword.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone, Copy)]
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
    /// Map error to `keyword` error.
    #[inline]
    pub fn map(self) -> impl FnOnce(UnsynError) -> UnsynError {
        move |err: UnsynError| UnsynError::Keyword(self, err.control_flow(), err.to_span())
    }
}

/// Error for a construct made of several tokens.
///
/// This is what [`crate::compiler::syntax`] and the composite tokens of
/// [`crate::compiler::lexer`] report: literals, comments, documentation, identifiers and
/// the expression forms of a node definition. The one exception is
/// [`SyntaxKind::Eof`], which a whole file reports when it cannot be read to its
/// end.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone, Copy)]
pub enum SyntaxKind {
    #[error("unicode literal")]
    Unicode,
    #[error("single quote")]
    QuoteEscape,
    #[error("ascii escape")]
    AsciiEscape,
    #[error("string content")]
    StrContent,
    #[error("ident")]
    Ident,
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
    /// One of the words a definition opens with, where none of them begins.
    #[error("definition keyword")]
    LeadingKeywords,
    /// One of the words that join two operands, where none of them begins.
    #[error("pattern operator keyword")]
    FollowingKeywords,
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
        move |err: UnsynError| UnsynError::Syntax(self, err.control_flow(), err.to_span())
    }

    /// Map error to `syntax` fatal error.
    #[inline]
    pub fn map_into_fatal(self) -> impl FnOnce(UnsynError) -> UnsynError {
        move |err: UnsynError| UnsynError::Syntax(self, ControlFlow::Fatal, err.to_span())
    }
}

/// Error for values that are well formed but meaningless.
///
/// Two groups live here. The first one is produced while *lexing*: a token can
/// have a legal shape and still denote nothing, as `\x8F` (a two-digit escape
/// outside the 7-bit range) or `U+D800` (a surrogate). The second one is
/// produced while [analyzing](crate::compiler::semantic) a whole file, where a name, a
/// range or a repetition can only be judged against the rest of the program.
///
/// No variant carries a piece of the file's text. The [`UnsynError`] a report
/// is written as already holds the span of the thing said of, and whoever holds
/// the file can read its text back from that span; what a variant may add is one
/// more span — the *other* place the report needs and the outer one cannot
/// speak for, such the first of two definitions of one name, or the definition a
/// mention reaches.
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
    /// A set member that answers with more than the one character a test of
    /// this position can ask.
    #[error(
        "set member denotes more than one character; write the refusal or the choice it is instead"
    )]
    WideSetMember,
    /// A set member whose definition can be matched by nothing.
    ///
    /// A set asks one question of one character, which is only a question when
    /// every member has a character to answer with; a member matched by nothing
    /// adds no character to the set, and the file that names one has written a
    /// choice of the characters it means, not a set of them.
    #[error(
        "set member reaches a rule that can be matched by nothing, defined at {0:?}; a set is one question of one character"
    )]
    EmptySetMember(Span),
    /// A refusal whose operand can be matched by nothing, which nothing can
    /// satisfy.
    ///
    /// `~X` takes the character at a position where `X` does not begin, and a
    /// definition that can be matched by nothing begins everywhere — so the
    /// refusal fails before it can take anything, wherever it stands. Only a
    /// name can be asked so of: a suffix written beside a `~` stands on the
    /// refusal and not under it, so `~X*` repeats a refusal that always takes
    /// one character, and `~B` is the refusal of whatever `B` reaches to.
    #[error(
        "refusal of a rule that can be matched by nothing, defined at {0:?}, is satisfied by nothing"
    )]
    UnsatisfiableRefusal(Span),
    #[error("undefined symbol")]
    UndefinedSymbol,
    #[error("name collision with the previous definition at {0:?}")]
    NameCollision(Span),
    #[error("duplicate `whitespace` declaration")]
    DuplicateWhitespace,
    /// A file that names no trivia has no separator, and so no tokens.
    ///
    /// The generated reader of a `syntax` rule drops a run of the trivia between
    /// every two operands, and it is that dropping which tells `lexer` from
    /// `lexerX`: without a separator the two are one uninterrupted run of
    /// characters, and an `except` on a keyword refuses only a whole token, never
    /// the boundary between one token and the next. No build can be written for
    /// such a file, so this is said once for the file rather than at each rule.
    #[error("the file declares no `whitespace` rule: without a separator no token has an end")]
    MissingWhitespace,
    #[error("terminal pattern can not use a `syntax` node, defined at {0:?}")]
    UnexpectedSyntax(Span),
    #[error("invalid range: the lower bound is greater than the upper bound")]
    InvalidRange,
    #[error("invalid repetition range")]
    InvalidRepeat,
    #[error("not a valid unicode code point")]
    InvalidCodePoint,
    #[error("expect a single character literal")]
    NotAChar,
    /// A node that can begin with the text it has to match as a whole.
    #[error("left-recursive definition")]
    LeftRecursion,
    /// A node whose match at a position can only be decided by deciding its own
    /// match at that same position, which neither advances nor terminates.
    #[error("definition that must be evaluated to evaluate itself")]
    SameOffsetRecursion,
}

impl SemanticsKind {
    /// Map error to `semantic` error.
    #[inline]
    pub fn map(self) -> impl FnOnce(UnsynError) -> UnsynError {
        move |err: UnsynError| UnsynError::Semantics(self, err.to_span())
    }
}

/// A report about a file that is meaningful but says something twice.
///
/// Nothing here makes a definition unusable: a rule of this family points at a
/// construct that could be written shorter, or that does not do what its author
/// meant, while the pattern it is part of still matches one language. The
/// analyzer therefore keeps these apart from its [`UnsynError`]s, and hands them
/// over with the model of a file it accepted (see
/// [`Diagnostics::lints`]); a caller that would rather refuse such a file is
/// free to treat a lint as fatal.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum LintKind {
    /// A member of a set whose value an earlier member of the same set already
    /// denotes.
    #[error("member `{0}` of this set is already declared")]
    DuplicateMember(String),
    /// A member every value of which an earlier member of the same set denotes.
    #[error("member `{0}` is covered by the earlier member `{1}` of this set")]
    CoveredMember(String, String),
    /// A member that denotes everything an earlier member of the same set does,
    /// and more besides, which makes that earlier member the redundant one.
    #[error("member `{0}` covers the earlier member `{1}` of this set")]
    CoversMember(String, String),
    /// A member that denotes some — but not all — of the values of an earlier
    /// member of the same set.
    #[error("member `{0}` overlaps the earlier member `{1}` of this set")]
    OverlappingMember(String, String),
    /// A rule of nodes that asks for the trivia the language separates itself
    /// with, which its generated separator takes whether it is named or not.
    #[error("`{0}` is the natural separator of the language, which a rule of nodes takes anyway")]
    RedundantSeparator(String),
    /// A choice written inside a group rather than as the whole of a right side.
    ///
    /// The root of a right side turns an `|` into the alternatives of a named
    /// node; inside a group the choice belongs to no left side, so its
    /// alternatives name nothing, and whatever reads the tree can only tell them
    /// apart by their shape.
    /// An alternative of a choice that names nothing — neither the rule it
    /// reaches nor a word it reads.
    ///
    /// The root of a right side turns an `|` into the alternatives of a named
    /// node, and an alternative that names no rule and reads no word gives its
    /// variant nothing to be called but its place, `F1`, `F2`. Inside a group
    /// the whole choice belongs to no left side, and every alternative of it
    /// names nothing at all. Either way what reads the tree can only tell the
    /// parts apart by their shape.
    /// A choice written inside a group rather than as the whole of a right side.
    ///
    /// The root of a right side turns an `|` into the alternatives of a named
    /// node; inside a group the choice belongs to no left side, so it becomes
    /// no node of the tree at all and whatever reads the tree can only tell its
    /// parts apart by their shape.
    /// A part of a pattern that becomes no named node of the tree.
    ///
    /// Two ways lead there, and they are told as one report because they say
    /// one thing and are fixed by one act: a choice written inside a group
    /// belongs to no left side, so it becomes no node at all, and an alternative
    /// of the whole of a right side that names no rule and reads no word gives
    /// its variant nothing to be called but its place, `F1`, `F2`. Either way
    /// the file is asked to write the nameless part as a rule of its own, whose
    /// left side then names it — or to begin it with a word of the language,
    /// which the variant can wear.
    ///
    /// Neither is asked of a group inside a `lexer` rule, which owes no name:
    /// the code reads what such a group matched as one run of text and keeps no
    /// part of it, so there is no node there to name. Asking for one would turn
    /// a token into a list of nodes, which is not what a terminal is for. A
    /// `lexer` rule is still asked about the alternatives of its whole right
    /// side, which do become the variants of an enum.
    ///
    /// An alternative of one operand is told only where it meant a name and did
    /// not get one. A `*`, a `'\n'` or a set reads no word, and there is nothing
    /// inside a single operand for a reader to tell apart, so its place says all
    /// a name would; two alternatives that both reach for the same word are a
    /// different matter, whatever their size, since one of them is left named by
    /// nothing but where it was written.
    #[error(
        "this names no node of the tree; write it as a rule of its own, or begin it with a word of the language"
    )]
    UnnamedAlternative,
}

/// One [`LintKind`] and the place in the file that says it.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
#[error("lint: {kind}, {span:?}")]
pub struct Lint {
    /// What the construct at `span` does that it need not do.
    pub kind: LintKind,
    /// The construct that earns the report.
    pub span: Span,
}

/// The two channels of a diagnostic report, kept apart by what they imply.
///
/// A pass that can find both kinds of problem writes to one of these and leaves
/// the question of what to do about each of them to its caller, which for the
/// [analyzer](crate::compiler::semantic) is the difference between rejecting a file and
/// returning the model of it with a note attached.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Diagnostics {
    /// The problems that leave the file meaning nothing.
    pub errors: Vec<UnsynError>,
    /// The problems that leave the file meaningful.
    pub lints: Vec<Lint>,
}

impl Diagnostics {
    /// Records a problem that leaves the file meaning nothing.
    #[inline]
    pub fn push(&mut self, error: UnsynError) {
        self.errors.push(error);
    }

    /// Records a problem that leaves the file meaningful.
    #[inline]
    pub fn lint(&mut self, kind: LintKind, span: Span) {
        self.lints.push(Lint { kind, span });
    }

    /// Records every report of `other`.
    #[inline]
    pub fn merge(&mut self, other: Self) {
        self.errors.extend(other.errors);
        self.lints.extend(other.lints);
    }

    /// Returns `true` when the file means nothing.
    #[inline]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns `true` when nothing at all was found.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty() && self.lints.is_empty()
    }
}

/// What a build refused to write, and where the file wrote the thing no code
/// answers.
///
/// A build writes the code of a file that means something; this family is what
/// it says when some part of that meaning has no code to answer it — an
/// external no caller bound, a name no item can carry, a literal that spells no
/// value — and unlike the errors of reading it is not a fault of the file's
/// text. It is told as [`UnsynError::Build`], beside the rule's own text and
/// the place the file wrote the thing there is no code to write.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum BuildKind {
    /// The file calls an external the builder was not given. The span is the
    /// definition of the rule that calls it; the call itself is where the report
    /// stands.
    #[error("the rule defined at {0:?} calls an external that is not bound")]
    External(Span),
    /// A `whitespace` rule denotes no set of single characters.
    #[error("the whitespace rule defined at {0:?} does not denote a set of characters")]
    Whitespace(Span),
    /// A name no item of Rust can carry. The report stands on the name itself,
    /// so this says no more than where it is.
    #[error("the rule is named what no item can be named")]
    Name,
    /// A literal that spells no value a parser can be written with.
    #[error("the rule holds a literal that spells nothing to read")]
    Literal,
    /// A shape of a `syntax` rule that no field of a node can hold. The span is
    /// the definition of the rule whose shape it is.
    #[error("the syntax rule defined at {0:?} holds a shape no code is written for")]
    Syntax(Span),
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
    #[error("unexpect/invalid: {0} at {1:?}")]
    Semantics(SemanticsKind, Span),

    /// Reports that a build of the file refused: the part at the span is what
    /// no code was written for.
    #[error("build: {0}, part at {1:?}")]
    Build(BuildKind, Span),
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
            UnsynError::Build(_, span) => span.clone(),
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
            UnsynError::Build(_, _) => ControlFlow::Fatal,
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
            UnsynError::Build(build_kind, span) => UnsynError::Build(build_kind, span),
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

        // A report of the file's own text carries no name of it: the span is
        // where the reader looks the name up.
        assert_eq!(
            UnsynError::Semantics(SemanticsKind::UndefinedSymbol, Span::Range(11..18)).to_string(),
            "unexpect/invalid: undefined symbol at Range(11..18)"
        );

        // What a report needs besides that place is one more place — the first
        // of two definitions of one name.
        assert_eq!(
            UnsynError::Semantics(
                SemanticsKind::NameCollision(Span::Range(6..7)),
                Span::Range(13..14),
            )
            .to_string(),
            "unexpect/invalid: name collision with the previous definition at Range(6..7) at Range(13..14)"
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

    /// The wording of the lint channel is as much a part of what a user reads as
    /// the wording of the errors, and it names the members by their value.
    #[test]
    fn every_lint_names_the_members_it_compares() {
        assert_eq!(
            Lint {
                kind: LintKind::DuplicateMember("'a'".into()),
                span: Span::Range(6..9),
            }
            .to_string(),
            "lint: member `'a'` of this set is already declared, Range(6..9)"
        );

        assert_eq!(
            Lint {
                kind: LintKind::CoveredMember("'c'".into(), "'a'-'z'".into()),
                span: Span::Range(12..15),
            }
            .to_string(),
            "lint: member `'c'` is covered by the earlier member `'a'-'z'` of this set, Range(12..15)"
        );

        assert_eq!(
            Lint {
                kind: LintKind::CoversMember("'a'-'z'".into(), "'c'".into()),
                span: Span::Range(10..16),
            }
            .to_string(),
            "lint: member `'a'-'z'` covers the earlier member `'c'` of this set, Range(10..16)"
        );

        assert_eq!(
            Lint {
                kind: LintKind::OverlappingMember("'b'-'d'".into(), "'a'-'c'".into()),
                span: Span::Range(16..22),
            }
            .to_string(),
            "lint: member `'b'-'d'` overlaps the earlier member `'a'-'c'` of this set, Range(16..22)"
        );
    }

    #[test]
    fn the_two_channels_of_a_report_stay_apart() {
        let mut report = Diagnostics::default();

        assert!(report.is_empty());
        assert!(!report.has_errors());

        report.push(UnsynError::Semantics(
            SemanticsKind::EmptySet,
            Span::Range(0..2),
        ));
        report.lint(LintKind::DuplicateMember("'a'".into()), Span::Range(1..4));

        assert!(report.has_errors());
        assert!(!report.is_empty());

        // A lint alone is not a reason to refuse a file.
        let only_a_lint = Diagnostics {
            errors: vec![],
            lints: report.lints.clone(),
        };

        assert!(!only_a_lint.has_errors());
        assert!(!only_a_lint.is_empty());

        let mut merged = Diagnostics::default();
        merged.merge(only_a_lint);

        assert!(merged.errors.is_empty());
        assert_eq!(merged.lints.len(), 1);
    }
}

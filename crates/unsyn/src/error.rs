//! The types used for `unsyn` diagnostic reports, and the error a build wrote.
//!
//! Two things meet in this module, and they are kept apart by where they come
//! from:
//!
//! - [`generated`] is the file `build.rs` wrote from `unsyn.us`: the framework's
//!   kind, and the family of rules a refusal is named after. It is included
//!   rather than written, and nothing here decides its contents — the spec does.
//! - [`UnsynError`] is the report every pass returns: that generated error, wrapped
//!   around the two families no grammar states — what a well formed token can still
//!   denote wrongly ([`SemanticsKind`]), and what no code was written for
//!   ([`BuildKind`]).
//!
//! The generated names are also re-exported here, because the code a build writes
//! reaches its error at `super::error::{ParseError, SyntaxKind}` — as a sibling of
//! the file it is written into, which is the one path a generator can know without
//! being told. Inside this module the same items are spelled `generated::…`, which
//! says where they come from.
//!
//! Every report also says which stage of [`unsyn`](crate) met the problem and which
//! of two channels it belongs to:
//!
//! - an [`UnsynError`] means the file denotes nothing, or that a build found no
//!   code to write for it, and the pass ends with its diagnostics rather than
//!   with a model;
//! - a [`Lint`] means the file denotes something it need not have said twice,
//!   and the analysis produces the model of it all the same.
//!
//! [`Diagnostics`] is the pair of lists a pass writes both kinds of finding to.
//!
//! Everything but the [`Semantics`](UnsynError::Semantics) and
//! [`Build`](UnsynError::Build) arms carries a
//! [`ControlFlow`]: [`Recovable`](ControlFlow::Recovable) means an enclosing
//! backtracking parser may try another alternative, [`Fatal`](ControlFlow::Fatal)
//! means the file is malformed and no alternative can help. A report of the two
//! hand-written families is fatal by definition: nothing further in the file can
//! be trusted with it.

use parserc::{ControlFlow, ParseError as ParseErrorTrait, Span};

/// What a build wrote for `unsyn.us`: the error a file of the language reports
/// when it is read, and the family of rules a refusal of one is named after.
///
/// Both are generated, and both are the whole of what the spec says about
/// failure: a combinator's own kind, and the rules whose productions point at
/// themselves. Nothing here adds a name the file did not write.
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/unsyn/error.rs"));
}

pub use generated::{ParseError, SyntaxKind};

/// Error for values that are well formed but meaningless.
///
/// Two groups live here. The first one is produced while *lexing*: a token can
/// have a legal shape and still denote nothing, as `\x8F` (a two-digit escape
/// outside the 7-bit range) or `U+D800` (a surrogate). The second one is
/// produced while [analyzing](crate::semantic) a whole file, where a name, a
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
    /// characters, and nothing in the language can put a boundary back. An
    /// `except` refuses only a whole token, never a boundary inside one, and the
    /// language has no lookahead of any kind — not even an assertion that could
    /// stop a name where a word ends. No build can be written for such a file,
    /// so this is said once for the file rather than at each rule.
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
///
/// Like an error, a lint carries **no text of the file**: [`Lint`] holds the span
/// of the member the lint is said of, and the variants below hold at most one
/// more span — the other member or definition the lint compares it to, which is
/// the one place the outer span cannot point.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum LintKind {
    /// A member of a set whose value an earlier member of the same set already
    /// denotes.
    #[error("member of this set is already declared")]
    DuplicateMember,
    /// A member every value of which an earlier member of the same set denotes.
    #[error("member is covered by the earlier member at {0:?} of this set")]
    CoveredMember(Span),
    /// A member that denotes everything an earlier member of the same set does,
    /// and more besides, which makes that earlier member the redundant one.
    #[error("member covers the earlier member at {0:?} of this set")]
    CoversMember(Span),
    /// A member that denotes some — but not all — of the values of an earlier
    /// member of the same set.
    #[error("member overlaps the earlier member at {0:?} of this set")]
    OverlappingMember(Span),
    /// A rule of nodes that asks for the trivia the language separates itself
    /// with, which its generated separator takes whether it is named or not.
    /// The span is the `whitespace` declaration that named the trivia.
    #[error(
        "the trivia declared at {0:?} is the natural separator of the language, which a rule of nodes takes anyway"
    )]
    RedundantSeparator(Span),
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
/// [analyzer](crate::semantic) is the difference between rejecting a file and
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
    /// A directory a build was told could not be made, or a file in it could not
    /// be written. The path is the caller's own, which is where it is read from.
    #[error("a build could not write the directory it was given")]
    Output,
}

/// The error of any phase that reads or builds a file of the language.
///
/// Three phases meet here, and they are not all written by the same hand:
///
/// - [`UnsynError::Parse`] carries what reading a file reports, and that enum is
///   [`generated`] — the code a build wrote from `unsyn.us`, so its kind of
///   combinator failure and its family of rules a refusal is named after arrive as
///   the generated file declares them. A change to a production is a change to
///   this arm, and nothing here has to be edited to follow it.
/// - [`UnsynError::Semantics`] carries what a well formed token or file can still
///   mean wrongly, which no grammar states because it is a fact about the whole
///   program rather than about one rule.
/// - [`UnsynError::Build`] carries what a build refused to write code for.
///
/// The last two are the DSL's own rules and belong to whoever analyzes a tree, so
/// they stay here by hand; the first is a fact of the spec, so it is generated.
/// Wrapping one inside the other is what lets a pass that both reads a file and
/// judges it return one error type, with `?` carrying a reading failure in on its
/// own.
#[derive(Debug, PartialEq, Eq)]
pub enum UnsynError {
    /// What reading the file reported: a combinator's own failure, or a refusal
    /// named after the rule whose production asked for it.
    Parse(generated::ParseError),

    /// A token or file that is well formed and denotes nothing.
    Semantics(SemanticsKind, Span),

    /// A build refused to write code for the part at the span.
    Build(BuildKind, Span),
}

/// The report of a reading failure is the generated enum's own words, told here
/// rather than asked of it: the front end this crate ships was written by the
/// released generator, which emits no `Display` for that error. The two arms are
/// the contract of a build's error — the framework's kind, and a refusal named
/// after a rule — and a spec changes which rules are named, never which arms
/// there are.
///
/// A rule's name is printed through `Debug` for the same reason: the released
/// generator writes no `Display` for the family of rules either.
impl std::fmt::Display for UnsynError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnsynError::Parse(generated::ParseError::Kind(kind)) => std::fmt::Display::fmt(kind, f),
            UnsynError::Parse(generated::ParseError::Syntax(kind, control_flow, span)) => {
                write!(
                    f,
                    "syntax error: expect {kind:?}, {control_flow:?}, {span:?}"
                )
            }
            UnsynError::Semantics(kind, span) => write!(f, "unexpect/invalid: {kind} at {span:?}"),
            UnsynError::Build(kind, span) => write!(f, "build: {kind}, part at {span:?}"),
        }
    }
}

impl std::error::Error for UnsynError {}

/// A combinator's failure is a reading failure, which the generated arm carries.
///
/// This is what lets `?` move a [`Kind`](parserc::Kind) out of a call into a pass
/// that returns [`UnsynError`], exactly as the generated parser's own functions
/// do.
impl From<parserc::Kind> for UnsynError {
    #[inline]
    fn from(kind: parserc::Kind) -> Self {
        Self::Parse(kind.into())
    }
}

/// What a generated parser reports is a report of reading, which is what a pass
/// that reads and then analyzes a file returns.
impl From<generated::ParseError> for UnsynError {
    #[inline]
    fn from(err: generated::ParseError) -> Self {
        Self::Parse(err)
    }
}

impl parserc::ParseError for UnsynError {
    #[inline]
    fn to_span(&self) -> Span {
        match self {
            UnsynError::Parse(err) => err.to_span(),
            UnsynError::Semantics(_, span) => span.clone(),
            UnsynError::Build(_, span) => span.clone(),
        }
    }

    #[inline]
    fn control_flow(&self) -> parserc::ControlFlow {
        match self {
            UnsynError::Parse(err) => err.control_flow(),
            UnsynError::Semantics(_, _) => ControlFlow::Fatal,
            UnsynError::Build(_, _) => ControlFlow::Fatal,
        }
    }

    #[inline]
    fn into_fatal(self) -> Self {
        match self {
            UnsynError::Parse(err) => Self::Parse(err.into_fatal()),
            UnsynError::Semantics(kind, span) => Self::Semantics(kind, span),
            UnsynError::Build(kind, span) => Self::Build(kind, span),
        }
    }
}

#[cfg(test)]
mod tests {
    use parserc::{ControlFlow, Kind, ParseError, Span};

    use super::*;

    /// The rendering of an error is what a user of the DSL reads, so the wording
    /// of every family is pinned here.
    #[test]
    fn every_family_names_what_was_expected() {
        // The arm a build wrote: a rule's name, and the two facts about the
        // position the refusal stands at.
        assert_eq!(
            UnsynError::Parse(generated::ParseError::Syntax(
                generated::SyntaxKind::LitUnicode,
                ControlFlow::Fatal,
                Span::None,
            ))
            .to_string(),
            "syntax error: expect LitUnicode, Fatal, None"
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
        let error = UnsynError::from(Kind::NextIf(ControlFlow::Recovable, Span::Range(0..1)));

        // The reading arm keeps the message of the combinator, and so it keeps
        // the code and the span it carried: the wrapper says nothing of its own
        // about a failure it did not judge.
        assert!(
            matches!(error, UnsynError::Parse(generated::ParseError::Kind(_))),
            "{error:?}"
        );
        assert_eq!(error.to_string(), "Error from `next_if` combinator");
        assert_eq!(error.control_flow(), ControlFlow::Recovable);
        assert_eq!(error.to_span(), Span::Range(0..1));
        assert!(!error.is_fatal());
    }

    #[test]
    fn a_readers_error_and_a_judges_error_are_one_type() {
        // The whole point of the wrapper: a pass that reads a file and then
        // judges it returns one error, and `?` moves either kind into it. The
        // reading kind is the generated enum, which the wrapper carries and does
        // not flatten — a refusal named after a rule of the spec stays in the
        // family the spec wrote.
        let read = UnsynError::from(generated::ParseError::Kind(Kind::Next(
            ControlFlow::Incomplete,
            Span::Range(4..4),
        )));
        let judged = UnsynError::Semantics(SemanticsKind::EmptySet, Span::Range(4..6));

        assert_eq!(read.to_span(), Span::Range(4..4));
        assert_eq!(judged.to_span(), Span::Range(4..6));
        assert_eq!(read.control_flow(), ControlFlow::Incomplete);
        assert_eq!(judged.control_flow(), ControlFlow::Fatal);

        // A failure of reading can be made fatal; a judgment is already fatal and
        // stays as it was.
        assert_eq!(
            read.into_fatal(),
            UnsynError::Parse(generated::ParseError::Kind(Kind::Next(
                ControlFlow::Fatal,
                Span::Range(4..4),
            )))
        );
        // A judgment is already fatal, and making it fatal keeps the family and
        // the place it stands at.
        assert_eq!(
            UnsynError::Semantics(SemanticsKind::EmptySet, Span::Range(4..6)).into_fatal(),
            UnsynError::Semantics(SemanticsKind::EmptySet, Span::Range(4..6)),
        );
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
            UnsynError::Parse(generated::ParseError::Syntax(
                generated::SyntaxKind::UnicodeEscape,
                ControlFlow::Incomplete,
                span.clone(),
            ))
            .into_fatal(),
            UnsynError::Parse(generated::ParseError::Syntax(
                generated::SyntaxKind::UnicodeEscape,
                ControlFlow::Fatal,
                span.clone(),
            ))
        );

        assert_eq!(
            UnsynError::Parse(generated::ParseError::Kind(Kind::Keyword(
                ControlFlow::Recovable,
                span.clone(),
            )))
            .into_fatal(),
            UnsynError::Parse(generated::ParseError::Kind(Kind::Keyword(
                ControlFlow::Fatal,
                span,
            )))
        );
    }

    #[test]
    fn a_refusal_named_after_a_rule_survives_being_wrapped() {
        // The wrapper exists so that a pass can return one error type; it must not
        // flatten what a build wrote. A refusal named after a rule of the spec
        // arrives holding that rule, and says so when it is read back.
        let refusal = generated::ParseError::Syntax(
            generated::SyntaxKind::NonKeywordIdent,
            ControlFlow::Fatal,
            Span::Range(30..46),
        );

        let wrapped = UnsynError::from(refusal.clone());

        assert_eq!(wrapped.to_span(), Span::Range(30..46));
        assert_eq!(wrapped.control_flow(), ControlFlow::Fatal);
        assert_eq!(
            wrapped,
            UnsynError::Parse(generated::ParseError::Syntax(
                generated::SyntaxKind::NonKeywordIdent,
                ControlFlow::Fatal,
                Span::Range(30..46),
            ))
        );

        // A semantic error has no code of its own, so only the span travels over
        // into the wrapper's judgment arm.
        assert_eq!(
            SemanticsKind::EmptySet.map()(UnsynError::from(refusal)),
            UnsynError::Semantics(SemanticsKind::EmptySet, Span::Range(30..46))
        );
    }

    /// The wording of the lint channel is as much a part of what a user reads as
    /// the wording of the errors, and it points at the member it compares.
    #[test]
    fn every_lint_points_at_the_member_it_compares() {
        assert_eq!(
            Lint {
                kind: LintKind::DuplicateMember,
                span: Span::Range(6..9),
            }
            .to_string(),
            "lint: member of this set is already declared, Range(6..9)"
        );

        // The other member is the one place the lint's own span cannot be: the
        // report stands on the later member and points back at the earlier one.
        assert_eq!(
            Lint {
                kind: LintKind::CoveredMember(Span::Range(2..8)),
                span: Span::Range(12..15),
            }
            .to_string(),
            "lint: member is covered by the earlier member at Range(2..8) of this set, Range(12..15)"
        );

        assert_eq!(
            Lint {
                kind: LintKind::CoversMember(Span::Range(2..5)),
                span: Span::Range(10..16),
            }
            .to_string(),
            "lint: member covers the earlier member at Range(2..5) of this set, Range(10..16)"
        );

        assert_eq!(
            Lint {
                kind: LintKind::OverlappingMember(Span::Range(2..8)),
                span: Span::Range(16..22),
            }
            .to_string(),
            "lint: member overlaps the earlier member at Range(2..8) of this set, Range(16..22)"
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
        report.lint(LintKind::DuplicateMember, Span::Range(1..4));

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

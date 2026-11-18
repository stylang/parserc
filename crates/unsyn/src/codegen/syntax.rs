//! The concrete syntax tree of the DSL: the twelve `syntax` rules that
//! `unsyn.us` beside this crate writes.
//!
//! One production, one type, named for the left side of the production. A type
//! holds the operands of its right side in the order they are written, and holds
//! them as the nodes they name, so the tree is a view of the source rather than a
//! summary of it: every character of a file can be found again in the tree, which
//! is what lets a diagnostic point at the text it talks about.
//!
//! A group on the right side of a production is no node of its own, since nothing
//! names it: `X+` is [`OneOrMore`], `X*` is a [`Vec`], `('a' 'b')` is a tuple, and
//! `'[' … ']'` is a [`Delimiter`]. What earns a type is a name on the *left* side,
//! and the code that reads a tree reads the names.
//!
//! # Where the trivia goes
//!
//! The terminals of this module are the puncts and the fixed words a `syntax` rule
//! is written with — `';'`, `'|'`, `'concat'` — and each of them owns the run of
//! the trivia on both sides of its text, which is why no field of a node here is
//! an `Option<S>` laid around a terminal. Where two operands that own nothing
//! stand next to each other (one name after another, or one operand of a sequence
//! after another), the sequence carries the run between them as a field of its
//! own. See the [crate documentation](crate) for the rule, and for why a run can
//! belong to exactly one of the two neighbours it lies between.
//!
//! The one place the two levels meet is a `lexer` rule reached by name, such as
//! [`Ident`] or [`LitStr`]: it owns no trivia, since its text is read into the
//! text of the rule that reaches it.

use parserc::{
    ControlFlow, Input, Kind, Span,
    syntax::{Delimiter, Or as Choice, Punctuated, Syntax, SyntaxInput},
};

use crate::codegen::lexer::{Ident, LeadingKeywords, LitDec, LitStr, LitUnicode, S, keyword_word};
use crate::codegen::{
    errors::{PunctKind, SemanticsKind, SyntaxKind, UnsynError},
    input::UnsynInput,
};

// ---------------------------------------------------------------------------
// The terminals a `syntax` rule is written with.
//
// Each is one fixed word, and each owns the run of the trivia on both sides of
// it: a definition may be written `A| B` as much as `A |B`, and no rule of the
// language has to say so. The runs are optional, so a terminal never fails for a
// missing separator; it fails only when its own text is not there.
// ---------------------------------------------------------------------------

/// Declares one terminal of the syntax level: a fixed word between two runs of
/// the trivia.
macro_rules! syntax_terminal {
    (
        $(#[$meta:meta])*
        $name:ident, $value:literal, $kind:ident :: $variant:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name<I>(
            /// The run of the trivia before the word.
            pub Option<S<I>>,
            /// The word itself.
            #[parserc(
                keyword = $value,
                map_err = crate::codegen::errors::$kind::$variant.map()
            )]
            pub I,
            /// The run of the trivia after the word, which the word keeps.
            pub Option<S<I>>,
        )
        where
            I: UnsynInput;
    };
}

syntax_terminal!(
    /// `;`, which ends a definition.
    Semi, ";", PunctKind::Semi
);
syntax_terminal!(
    /// `+`, the suffix that asks for one operand or more.
    Plus, "+", PunctKind::Plus
);
syntax_terminal!(
    /// `*`, the suffix that asks for an operand any number of times.
    Star, "*", PunctKind::Star
);
syntax_terminal!(
    /// `?`, the suffix that asks for an operand at most once.
    Question, "?", PunctKind::Question
);
syntax_terminal!(
    /// `~`, the prefix that asks for the one character a set does not denote.
    Tilde, "~", PunctKind::Tilde
);
syntax_terminal!(
    /// `|`, which joins two alternatives.
    Or, "|", PunctKind::Or
);
syntax_terminal!(
    /// `(`, which opens a group.
    ParenStart, "(", PunctKind::ParenStart
);
syntax_terminal!(
    /// `)`, which closes a group.
    ParenEnd, ")", PunctKind::ParenEnd
);
syntax_terminal!(
    /// `[`, which opens a set.
    BracketStart, "[", PunctKind::BracketStart
);
syntax_terminal!(
    /// `]`, which closes a set.
    BracketEnd, "]", PunctKind::BracketEnd
);
syntax_terminal!(
    /// `{`, which opens the bound of a repetition.
    BraceStart, "{", PunctKind::BraceStart
);
syntax_terminal!(
    /// `}`, which closes the bound of a repetition.
    BraceEnd, "}", PunctKind::BraceEnd
);
syntax_terminal!(
    /// `<`, which opens a call of an external.
    Lt, "<", PunctKind::Lt
);
syntax_terminal!(
    /// `>`, which closes a call of an external.
    Gt, ">", PunctKind::Gt
);
syntax_terminal!(
    /// `,`, which separates two members of a set or the two bounds of a
    /// repetition.
    Comma, ",", PunctKind::Comma
);
syntax_terminal!(
    /// `->`, which divides the name of a node from the pattern it denotes.
    ArrowRight, "->", PunctKind::ArrowRight
);
/// Declares one terminal of the syntax level that is a word rather than a sign.
///
/// A word of the pattern level is read by [`keyword_word`], which refuses the
/// head of a longer name: `exceptKeywords` is one name, not the word `except`
/// and the start of another, since a name of the described language is read as
/// long as it can be.
macro_rules! syntax_word {
    (
        $(#[$meta:meta])*
        $name:ident, $value:literal, $kind:ident :: $variant:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name<I>(
            /// The run of the trivia before the word.
            pub Option<S<I>>,
            /// The word itself, which takes no run of the trivia.
            #[parserc(parser = keyword_word($value, crate::codegen::errors::$kind::$variant))]
            pub I,
            /// The run of the trivia after the word, which the word keeps.
            pub Option<S<I>>,
        )
        where
            I: UnsynInput;
    };
}

syntax_word!(
    /// `concat`, which asks that what two operands matched be joined into one
    /// token.
    Concat, "concat", KeywordKind::Concat
);
syntax_word!(
    /// `followed`, which asks that an operand come next without being taken.
    Followed, "followed", KeywordKind::Followed
);
syntax_word!(
    /// `except`, which removes what one operand matches from another.
    Except, "except", KeywordKind::Except
);
syntax_terminal!(
    /// `-`, which divides the two ends of a range.
    ///
    /// The form of a range written between two strings asks for this word by
    /// `concat`, which is why [`Range::Str`] holds the bare text of a `-` rather
    /// than a terminal of this shape: nothing may stand between a string, this
    /// `-` and the string after it.
    Minus, "-", PunctKind::Minus
);

/// `X+`, over the trivia of the language: one `X`, then every `X` the text offers
/// after it, each with the run that separates it from the operand before.
///
/// A sequence has no operator of its own, which is why the run between two of its
/// operands is part of the tree: it is what tells them apart. A run that the
/// operand before already swallowed leaves the field `None`, as in `<ident> 'x'`,
/// where the call keeps the space in its closing `>`.
///
/// This is the shape of a repetition and not a node of the grammar, which is why
/// it is generic over the operand it repeats.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OneOrMore<I, X>
where
    I: UnsynInput,
    X: Syntax<I>,
{
    /// The operand every sequence must have, since `+` asks for at least one.
    pub first: X,
    /// The operands that follow it, each with the run before it.
    pub rest: Vec<(Option<S<I>>, X)>,
}

impl<I, X> OneOrMore<I, X>
where
    I: UnsynInput,
    X: Syntax<I>,
{
    /// The operands of the sequence, in the order they were written.
    #[inline]
    pub fn elements(&self) -> impl Iterator<Item = &X> {
        std::iter::once(&self.first).chain(self.rest.iter().map(|(_, element)| element))
    }
}

// ---------------------------------------------------------------------------
// `syntax Expr -> ExprNoTopAlt+ ( '|' ExprNoTopAlt+ )*;`
// ---------------------------------------------------------------------------

/// The pattern a definition gives its node: alternatives joined by `|`, each of
/// them a sequence of operands.
///
/// `|` binds loosest and a sequence binds tightest, so `'a' | 'b' 'c'` is the
/// alternative `'a'` against the sequence of `'b'` then `'c'`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = map_expr_err)]
pub struct Expr<I>
where
    I: UnsynInput,
{
    /// The first alternative, which is also the whole expression when no `|`
    /// follows it.
    pub first: OneOrMore<I, ExprNoTopAlt<I>>,
    /// Every alternative after the first, with the `|` that introduced it.
    pub rest: Vec<Alternative<I>>,
}

impl<I: UnsynInput> Expr<I> {
    /// The alternatives of the expression, in the order they were written.
    ///
    /// The `|` that introduced an alternative is trivia of the tree rather than
    /// part of the alternative, so it is dropped here.
    #[inline]
    pub fn branches(&self) -> impl Iterator<Item = &OneOrMore<I, ExprNoTopAlt<I>>> {
        std::iter::once(&self.first).chain(self.rest.iter().map(|(_, branch)| branch))
    }

    /// Whether the expression offers exactly one way of being read.
    #[inline]
    pub fn is_single(&self) -> bool {
        self.rest.is_empty()
    }
}

/// The group `('|' ExprNoTopAlt+)` of the [`Expr`] rule, in the shape the
/// production writes it: the bar that introduced an alternative, then the
/// sequence of operands that makes it up.
///
/// A group of a right side names no node of the tree, so the tuple it is written
/// as stands for it as it is.
pub type Alternative<I> = (Or<I>, OneOrMore<I, ExprNoTopAlt<I>>);

/// Turns an expression that ran into a keyword into the definition that should
/// have ended before it.
///
/// A definition's pattern is followed by `;`, so the next keyword can only mean
/// that the `;` was left out. Reporting a missing `;` where the pattern ends is
/// more useful than reporting an unexpected keyword there, since the file is
/// otherwise read as one definition glued to the next.
#[inline]
fn map_expr_err(err: UnsynError) -> UnsynError {
    match err {
        UnsynError::Semantics(SemanticsKind::Keyword, span) => {
            UnsynError::Punct(PunctKind::Semi, ControlFlow::Fatal, span)
        }
        err => err,
    }
}

// ---------------------------------------------------------------------------
// `syntax ExprNoTopAlt -> ExprWithSuffix | ExprWithoutSuffix;`
// ---------------------------------------------------------------------------

/// One operand of one alternative: either something that stands alone, or that
/// same thing with one suffix.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = map_expr_no_top_alt_err)]
pub enum ExprNoTopAlt<I>
where
    I: UnsynInput,
{
    /// An operand carrying a suffix.
    WithSuffix(ExprWithSuffix<I>),
    /// An operand that stands alone.
    WithoutSuffix(ExprWithoutSuffix<I>),
}

/// Reports that no operand of an alternative matched here.
///
/// An enum that has tried every variant ends with the bare combinator error of
/// its own name, which is the one error of this node that says nothing beyond "no
/// operand"; it is given the [`SyntaxKind`] the other nodes report with. An error
/// from inside a variant is left as it is: the operand that met it has already
/// named what it expected there, which is more than this node can say.
#[inline]
fn map_expr_no_top_alt_err(err: UnsynError) -> UnsynError {
    match err {
        UnsynError::Kind(Kind::Syntax("ExprNoTopAlt", control_flow, span)) => {
            UnsynError::Syntax(SyntaxKind::ExprNoTopAlt, control_flow, span)
        }
        err => err,
    }
}

// ---------------------------------------------------------------------------
// `syntax ExprWithoutSuffix ->`
// `      '~' '[' ( SetItem ',' )* SetItem? ']'`
// `    | '<' Ident '>'`
// `    | '(' Expr ')'`
// `    | '[' ( SetItem ',' )* SetItem? ']'`
// `    | LitStr | LitUnicode | Ident ;`
// ---------------------------------------------------------------------------

/// `('[' ( SetItem ',' )* SetItem? ']')`, which is what [`Punctuated`] reads: a
/// list of members, most of them followed by a comma, and a comma the closing
/// bracket may keep.
///
/// A group of a right side is no node of its own, so a set is the [`Delimiter`]
/// it is written as rather than a type declared for it.
pub type Set<I> = Delimiter<BracketStart<I>, BracketEnd<I>, Punctuated<SetItem<I>, Comma<I>>>;

/// An operand that stands alone, with no suffix on it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExprWithoutSuffix<I>
where
    I: UnsynInput,
{
    /// `'~' […]`, the one character a set does not denote.
    ///
    /// What `~` asks is answered by the character here, and takes it, so the
    /// complement fails where the set would have fitted and always consumes
    /// input. That is what lets a line comment read to the end of its line with
    /// `~[LF]*` rather than stopping short of it.
    Tilde(
        /// The `~` punct.
        Tilde<I>,
        /// The negated set, whose members are the values refused here.
        Set<I>,
    ),
    /// `'<' Ident '>'`, a call of an external the caller of the parser binds, as
    /// in `<XID_Start>`.
    Call(Delimiter<Lt<I>, Gt<I>, Ident<I>>),
    /// `'(' Expr ')'`, a group, so that an expression may sit inside a sequence
    /// or take a suffix as one operand.
    Paren(Delimiter<ParenStart<I>, ParenEnd<I>, Box<Expr<I>>>),
    /// `'[' … ']'`, a set of the characters and strings it lists.
    ///
    /// A set is a union, so the pass that reads it compares its members by the
    /// values they denote rather than by how they are written.
    Set(Set<I>),
    /// A literal string, as in `'a'`.
    Str(LitStr<I>),
    /// A literal code point, as in `U+0009`.
    Unicode(LitUnicode<I>),
    /// A reference to another rule by name.
    Reference(Ident<I>),
}

// ---------------------------------------------------------------------------
// `syntax ExprWithSuffix ->`
// `      ExprWithoutSuffix ( '*' | '?' | '+' | Repeat )`
// `    | ExprWithoutSuffix 'concat' ExprNoTopAlt`
// `    | ExprWithoutSuffix 'followed' ExprNoTopAlt`
// `    | ExprWithoutSuffix 'except' ExprWithoutSuffix ;`
// ---------------------------------------------------------------------------

/// The group `('*' | '?' | '+' | Repeat)`, in the shape the production writes it:
/// a choice of four, and no name of its own.
///
/// The nested [`Choice`] is the combinator for a choice, so the group is stored as
/// the group it is rather than as a type declared for it.
pub type SuffixGroup<I> = Choice<Star<I>, Choice<Question<I>, Choice<Plus<I>, Repeat<I>>>>;

/// An operand carrying one suffix.
///
/// The four quantity operators are the one alternative the production writes them
/// as, and not one alternative apiece: a node is named for the left side of a
/// production, and the group on this right side is denoted by [`SuffixGroup`],
/// which names nothing of its own.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExprWithSuffix<I>
where
    I: UnsynInput,
{
    /// The operand carries one of `*`, `?`, `+` or `{…}`.
    Grouped {
        /// The operand the operator applies to.
        target: ExprWithoutSuffix<I>,
        /// The operator the group picked.
        suffix: SuffixGroup<I>,
    },
    /// `concat`, which asks that what the two operands matched be joined into one
    /// token. Both operands are matched, the second one from where the first one
    /// ended.
    Concat {
        /// What is joined to the operand after the keyword.
        target: ExprWithoutSuffix<I>,
        /// The keyword, with the runs on each side of it.
        #[parserc(crucial)]
        keyword: Concat<I>,
        /// The operand after the keyword.
        suffix: Box<ExprNoTopAlt<I>>,
    },
    /// `followed`, which asks that the operand after the keyword come next in the
    /// source without being part of what matched here. It is therefore tested
    /// after whatever the target consumed, never at the position the target
    /// starts at.
    Followed {
        /// What the operand after the keyword must follow.
        target: ExprWithoutSuffix<I>,
        /// The keyword, with the runs on each side of it.
        #[parserc(crucial)]
        keyword: Followed<I>,
        /// The operand after the keyword.
        suffix: Box<ExprNoTopAlt<I>>,
    },
    /// `except`, which removes what the operand after the keyword matches from
    /// the operand before it.
    ///
    /// What is removed is asked of the text the target read and of nothing else,
    /// and only a removal that takes all of it refuses the operand: `lexerX` is
    /// therefore a name of its own, although `lexer` fits the head of it.
    Except {
        /// What the operand after the keyword is taken out of.
        target: ExprWithoutSuffix<I>,
        /// The keyword, with the runs on each side of it.
        #[parserc(crucial)]
        keyword: Except<I>,
        /// The operand to take out, which cannot itself carry a suffix.
        tokens: ExprWithoutSuffix<I>,
    },
}

// ---------------------------------------------------------------------------
// `syntax Repeat -> '{' ( ',' LitDec | LitDec ',' LitDec? | LitDec ) '}';`
// ---------------------------------------------------------------------------

/// The bounds of a repetition: the whole `{…}`, braces and all, since that is
/// what the production puts on its left side.
///
/// The operand it follows is the one that says what gets counted. What the braces
/// hold is the group [`RepeatBounds`] writes out, which is why the group is a type
/// expression and no type of its own.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Repeat<I>(
    /// The `{` … `}` the bounds are written between.
    pub Delimiter<BraceStart<I>, BraceEnd<I>, RepeatBounds<I>>,
)
where
    I: UnsynInput;

/// The group `(',' LitDec | LitDec ',' LitDec? | LitDec)` of the [`Repeat`] rule,
/// in the shape the production writes it.
///
/// The comma form is read before the bare count, which is what lets `{2, 3}` be
/// two bounds rather than a count of two followed by a closing brace that finds a
/// comma and fails fatally.
pub type RepeatBounds<I> =
    Choice<(Comma<I>, LitDec<I>), Choice<(LitDec<I>, Comma<I>, Option<LitDec<I>>), LitDec<I>>>;

/// The bounds a [`Repeat`] spells, flattened so that a match can name the three
/// forms instead of walking the nested [`Choice`] and the tuples inside it.
///
/// This is a *view* of a [`Repeat`] and not a node: no tree stores one, and taking
/// it costs nothing but reading a reference.
#[derive(Debug)]
pub enum RepeatBound<'a, I>
where
    I: UnsynInput,
{
    /// `,n`, at most `n` times.
    RangeTo(&'a Comma<I>, &'a LitDec<I>),
    /// `n,m`, or `n,` with no bound above.
    RangeFrom(&'a LitDec<I>, &'a Comma<I>, Option<&'a LitDec<I>>),
    /// `n`, exactly `n` times.
    Count(&'a LitDec<I>),
}

impl<'a, I: UnsynInput> RepeatBound<'a, I> {
    /// The form the bounds were written in.
    #[inline]
    pub fn of(repeat: &'a Repeat<I>) -> Self {
        match &repeat.0.body {
            Choice::First((comma, end)) => Self::RangeTo(comma, end),
            Choice::Second(Choice::First((start, comma, end))) => {
                Self::RangeFrom(start, comma, end.as_ref())
            }
            Choice::Second(Choice::Second(count)) => Self::Count(count),
        }
    }
}

// ---------------------------------------------------------------------------
// `syntax Range -> LitUnicode '-' LitUnicode`
// `               | LitStr concat '-' concat LitStr ;`
// ---------------------------------------------------------------------------

/// The two ends of a range of a set, written between a `-`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Range<I>
where
    I: UnsynInput,
{
    /// A range of code points, as `U+0009-U+000D` is.
    Unicode(
        LitUnicode<I>,
        /// The `-` that divides the two ends.
        #[parserc(crucial)]
        Minus<I>,
        LitUnicode<I>,
    ),
    /// A range of strings, as `'0'-'7'` is.
    ///
    /// The rule asks for the dash of this form by `concat`, so it holds no trivia
    /// and stands with nothing on either side of it.
    Str(
        LitStr<I>,
        /// The `-` that divides the two ends, joined to both of them.
        #[parserc(crucial, keyword = "-", map_err = PunctKind::Minus.map())]
        I,
        LitStr<I>,
    ),
}

// ---------------------------------------------------------------------------
// `syntax SetItem -> Range | LitStr | LitUnicode | Ident;`
// ---------------------------------------------------------------------------

/// One member of a set.
///
/// A range is read before the two literals one range is made of, which is what
/// lets `'0'-'7'` be one member rather than a member followed by a dash that no
/// rule can account for.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SetItem<I>
where
    I: UnsynInput,
{
    /// The values between two ends.
    Range(Range<I>),
    /// A literal string, which denotes the characters it holds.
    Str(LitStr<I>),
    /// A literal code point.
    Unicode(LitUnicode<I>),
    /// A reference to a rule whose values are members here.
    Reference(Ident<I>),
}

// ---------------------------------------------------------------------------
// `syntax Stmt -> LeadingKeywords Ident '->' Expr ';';`
// ---------------------------------------------------------------------------

/// One definition of a file, which names a node and gives the pattern it denotes.
///
/// Which of the three kinds of node it defines is not a field of its own but the
/// word it opens with, read from the text of the [`LeadingKeywords`] of it: a
/// `whitespace` declares the trivia of the language, a `lexer` a token, and a
/// `syntax` a node of the tree.
///
/// The `->` and the `;` are one field rather than three, since they are the two
/// ends of the run the pattern is written between, which is the shape the
/// production gives them.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Stmt<I>
where
    I: UnsynInput,
{
    /// The word that says what kind of node is defined.
    #[parserc(crucial)]
    pub keyword: LeadingKeywords<I>,
    /// The run between the word and the name, which neither of the two owns.
    pub s: Option<S<I>>,
    /// The name of the node.
    pub ident: Ident<I>,
    /// The `->` … `;` the pattern is written between.
    pub definition: Delimiter<ArrowRight<I>, Semi<I>, Expr<I>>,
}

// ---------------------------------------------------------------------------
// `syntax OuterDoc -> OuterBlockDoc | OuterLineDoc;`
// `syntax InnerDoc -> InnerBlockDoc | InnerLineDoc;`
// `syntax Item -> OuterDoc | S | Stmt;`
// ---------------------------------------------------------------------------

/// A document that describes the definition after it.
///
/// These are items of a file rather than fields of the definition they describe,
/// since a definition is read between its own two words and a document may be
/// written before any one of them.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OuterDoc<I>
where
    I: UnsynInput,
{
    /// A `/**` form.
    Block(super::lexer::OuterBlockDoc<I>),
    /// A `///` form.
    Line(super::lexer::OuterLineDoc<I>),
}

/// A document that describes the file it is written in.
///
/// These describe a file, so they come before anything a file defines.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InnerDoc<I>
where
    I: UnsynInput,
{
    /// A `/*!` form.
    Block(super::lexer::InnerBlockDoc<I>),
    /// A `//!` form.
    Line(super::lexer::InnerLineDoc<I>),
}

/// One item of a file: a document, a run of the trivia, or a definition.
///
/// The trivia is an item rather than something skipped between items, which is
/// what lets every character of a file be found again in the tree.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Item<I>
where
    I: UnsynInput,
{
    /// A document that belongs to the definition after it.
    OuterDoc(OuterDoc<I>),
    /// The trivia between two items.
    S(S<I>),
    /// A definition of a node.
    Stmt(Stmt<I>),
}

// ---------------------------------------------------------------------------
// `syntax Crate -> InnerDoc* Item*;`
// ---------------------------------------------------------------------------

/// A parsed file.
///
/// A file is read to its end, which is more than the two repetitions of the rule
/// say on their own: a list ends where the next item fails to match, and whatever
/// is left over then belongs to no rule of the grammar. It is therefore reported
/// as [`SyntaxKind::Eof`] rather than handed back to the caller, since a file that
/// stops short of its end is broken and not merely short.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Crate<I>
where
    I: UnsynInput,
{
    /// The documents that describe the file, each with the run before it.
    ///
    /// A `/*!` form owns no trivia, so the run between two of them is the list's
    /// own.
    pub inner_docs: Vec<(Option<S<I>>, InnerDoc<I>)>,
    /// The items of the file, in the order they were written.
    ///
    /// No run of the trivia stands between two of these, since a run is an item of
    /// its own: [`Item::S`] holds it.
    pub items: Vec<Item<I>>,
}

impl<I> Syntax<I> for Crate<I>
where
    I: UnsynInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as Input>::Error> {
        let inner_docs = input.parse()?;
        let items = input.parse()?;

        if !input.is_empty() {
            return Err(UnsynError::Syntax(
                SyntaxKind::Eof,
                ControlFlow::Fatal,
                input.to_span(),
            ));
        }

        Ok(Self { inner_docs, items })
    }

    #[inline]
    fn to_span(&self) -> Span {
        Span::None
            .union(&self.inner_docs.to_span())
            .union(&self.items.to_span())
    }
}

#[cfg(test)]
mod tests {
    use parserc::AsStr;

    use super::*;
    use crate::codegen::input::TokenStream;

    /// Parses `source` as the node `T`, refusing whatever is left of it.
    fn whole<'a, T>(source: &'a str) -> Result<T, UnsynError>
    where
        T: Syntax<TokenStream<'a>>,
    {
        let mut input = TokenStream::from(source);
        let node = input.parse::<T>()?;

        if !input.is_empty() {
            return Err(UnsynError::Syntax(
                SyntaxKind::Eof,
                ControlFlow::Recovable,
                input.to_span(),
            ));
        }

        Ok(node)
    }

    #[test]
    fn a_terminal_keeps_the_runs_on_both_sides_of_it() {
        let semi = whole::<Semi<_>>(" ; ").unwrap();

        assert_eq!(semi.1.as_str(), ";");
        assert_eq!(semi.0.unwrap().0.as_str(), " ");
        // The run after the word belongs to the word, which is why no sequence of
        // terminals of this level carries a gap of its own.
        assert_eq!(semi.2.unwrap().0.as_str(), " ");

        // A separator is never required, so a word of the language may stand
        // against its neighbours with nothing between them.
        let tight = whole::<Semi<_>>(";").unwrap();
        assert!(tight.0.is_none() && tight.2.is_none());
    }

    #[test]
    fn an_operand_is_read_with_or_without_its_suffix() {
        assert!(matches!(
            whole::<ExprNoTopAlt<_>>("'a'").unwrap(),
            ExprNoTopAlt::WithoutSuffix(_)
        ));
        assert!(matches!(
            whole::<ExprNoTopAlt<_>>("'a'*").unwrap(),
            ExprNoTopAlt::WithSuffix(ExprWithSuffix::Grouped { .. })
        ));
        assert!(matches!(
            whole::<ExprNoTopAlt<_>>("A{1, 2}").unwrap(),
            ExprNoTopAlt::WithSuffix(ExprWithSuffix::Grouped { .. })
        ));
        assert!(matches!(
            whole::<ExprNoTopAlt<_>>("<XID_Start>").unwrap(),
            ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Call(_))
        ));

        // Only one suffix is allowed, so `A*?` is not a repetition of an option.
        assert!(whole::<ExprNoTopAlt<_>>("A*?").is_err());
    }

    #[test]
    fn a_sequence_carries_the_run_between_two_operands_that_own_nothing() {
        let expr = whole::<Expr<_>>("'a' 'b'").unwrap();

        assert_eq!(expr.branches().count(), 1);
        let (gap, operand) = expr.first.rest.first().unwrap();
        // `'a'` is a lexer token and owns no run, so the space between the two is
        // the sequence's own.
        assert_eq!(gap.as_ref().unwrap().0.as_str(), " ");
        assert_eq!(operand.to_span(), Span::Range(4..7));

        // A call owns the run inside its closing `>`, so the sequence has nothing
        // left to carry.
        let expr = whole::<Expr<_>>("<XID_Start> 'a'").unwrap();
        assert!(expr.first.rest.first().unwrap().0.is_none());
    }

    #[test]
    fn an_alternative_is_joined_by_a_bar() {
        let expr = whole::<Expr<_>>("'a' | 'b' | 'c'").unwrap();

        assert_eq!(expr.branches().count(), 3);
        assert_eq!(expr.rest[0].1.elements().count(), 1);
        assert!(!expr.is_single());

        // A bar with nothing after it is not an alternative.
        assert!(whole::<Expr<_>>("'a' |").is_err());
    }

    #[test]
    fn a_missing_semicolon_is_reported_where_the_pattern_ends() {
        // The next definition can only mean that the `;` was left out, and that is
        // more useful here than a report of an unexpected keyword.
        let mut input = TokenStream::from("syntax A -> 'a' syntax B -> 'b';");

        assert_eq!(
            input.parse::<Stmt<_>>(),
            Err(UnsynError::Punct(
                PunctKind::Semi,
                ControlFlow::Fatal,
                Span::Range(16..22),
            )),
        );
    }

    #[test]
    fn the_three_suffix_keywords_tell_their_forms_apart() {
        assert!(matches!(
            whole::<ExprNoTopAlt<_>>("'a' concat 'b'").unwrap(),
            ExprNoTopAlt::WithSuffix(ExprWithSuffix::Concat { .. })
        ));
        assert!(matches!(
            whole::<ExprNoTopAlt<_>>("'a' followed 'b'").unwrap(),
            ExprNoTopAlt::WithSuffix(ExprWithSuffix::Followed { .. })
        ));
        assert!(matches!(
            whole::<ExprNoTopAlt<_>>("A except Keywords").unwrap(),
            ExprNoTopAlt::WithSuffix(ExprWithSuffix::Except { .. })
        ));

        // A suffix takes a single operand, so `A except B | C` is the alternative
        // `A except B` against the alternative `C`.
        let expr = whole::<Expr<_>>("A except B | C").unwrap();
        assert_eq!(expr.branches().count(), 2);
    }

    #[test]
    fn the_bounds_of_a_repetition_have_three_forms() {
        use RepeatBound::{Count, RangeFrom, RangeTo};

        assert!(matches!(
            RepeatBound::of(&whole::<Repeat<_>>("{3}").unwrap()),
            Count(_)
        ));
        assert!(matches!(
            RepeatBound::of(&whole::<Repeat<_>>("{1, 6}").unwrap()),
            RangeFrom(_, _, Some(_))
        ));
        assert!(matches!(
            RepeatBound::of(&whole::<Repeat<_>>("{1,}").unwrap()),
            RangeFrom(_, _, None)
        ));
        assert!(matches!(
            RepeatBound::of(&whole::<Repeat<_>>("{,4}").unwrap()),
            RangeTo(_, _)
        ));

        // A bound of two is not a bound followed by a brace that finds a comma.
        assert!(whole::<Repeat<_>>("{1, 2, 3}").is_err());
    }

    #[test]
    fn a_range_holds_its_two_ends() {
        assert!(matches!(
            whole::<SetItem<_>>("U+0009-U+000D").unwrap(),
            SetItem::Range(Range::Unicode(..))
        ));
        assert!(matches!(
            whole::<SetItem<_>>("'0'-'7'").unwrap(),
            SetItem::Range(Range::Str(..))
        ));

        // The `concat` of the rule leaves nothing between a string and its dash.
        assert!(whole::<SetItem<_>>("'0' - '7'").is_err());

        // A member is read as a range before it is read as the literal that one
        // end of a range is on its own.
        assert!(matches!(
            whole::<SetItem<_>>("'a'").unwrap(),
            SetItem::Str(_)
        ));
    }

    #[test]
    fn a_set_lists_its_members_with_the_optional_last_comma() {
        for source in [
            "['a']",
            "['a', 'b']",
            "['a', 'b',]",
            "[]",
            "[ U+0009-U+000D, CR ]",
        ] {
            assert!(
                matches!(
                    whole::<ExprWithoutSuffix<_>>(source),
                    Ok(ExprWithoutSuffix::Set(_))
                ),
                "{source}"
            );
        }

        // A `~` asks its question of a set and of nothing else, since what it
        // matches is exactly one character.
        assert!(matches!(
            whole::<ExprWithoutSuffix<_>>("~['a', 'b']"),
            Ok(ExprWithoutSuffix::Tilde(..))
        ));
        assert!(whole::<ExprWithoutSuffix<_>>("~'a'").is_err());
    }

    #[test]
    fn a_definition_is_its_keyword_its_name_and_its_pattern() {
        let source = "syntax Expr -> ExprNoTopAlt+ ( '|' ExprNoTopAlt+ )*;";
        let stmt = whole::<Stmt<_>>(source).unwrap();

        assert_eq!(stmt.keyword.0.as_str(), "syntax");
        assert_eq!(stmt.ident.text().as_str(), "Expr");
        assert_eq!(stmt.to_span(), Span::Range(0..source.len()));

        // The name is read after the word, and neither owns the run between them.
        assert_eq!(stmt.s.unwrap().0.as_str(), " ");
        assert!(whole::<Stmt<_>>("syntaxExpr -> 'a';").is_err());
        assert!(whole::<Stmt<_>>("syntax Expr -> 'a'").is_err());
    }

    #[test]
    fn a_file_holds_every_character_it_was_given() {
        let source = "//! about\n\nwhitespace S -> [' '];\n\n/// nothing\nlexer LF -> U+000A;\n";
        let file =
            whole::<Crate<_>>(source).expect("a file of two definitions and the text around them");

        assert_eq!(file.inner_docs.len(), 1);
        assert_eq!(
            file.items
                .iter()
                .filter(|item| matches!(item, Item::Stmt(_)))
                .count(),
            2
        );
        assert_eq!(
            file.items
                .iter()
                .filter(|item| matches!(item, Item::S(_)))
                .count(),
            2,
            "a run of the trivia no terminal took is an item of its own"
        );
        assert_eq!(file.to_span(), Span::Range(0..source.len()));
    }

    #[test]
    fn text_no_item_can_read_breaks_the_file() {
        assert_eq!(
            TokenStream::from("lexer A -> 'a';\n@").parse::<Crate<_>>(),
            Err(UnsynError::Syntax(
                SyntaxKind::Eof,
                ControlFlow::Fatal,
                Span::Range(16..17)
            ))
        );
    }

    /// The spec reads itself: the file that declares this grammar is a file of the
    /// language it declares.
    #[test]
    fn unsyn_us_is_a_file_of_the_language_it_declares() {
        let source = include_str!("../../unsyn.us");

        let file =
            whole::<Crate<_>>(source).expect("unsyn.us should be a file of its own language");

        let statements = file
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Stmt(stmt) => Some(stmt.keyword.0.as_str().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        // One `whitespace`, twenty-seven `lexer` and twelve `syntax` rules.
        assert_eq!(statements.len(), 40);
        assert_eq!(statements.iter().filter(|k| *k == "lexer").count(), 27);
        assert_eq!(statements.iter().filter(|k| *k == "syntax").count(), 12);
        assert_eq!(statements.iter().filter(|k| *k == "whitespace").count(), 1);

        // Every one of them is named, which is what says that a rule of the spec
        // and a type of this crate are the same thing under the same name.
        let names = file
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Stmt(stmt) => Some(stmt.ident.text().as_str().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(names.len(), 40, "a definition without a name");
        for name in [
            "S",
            "IdentOrKeyword",
            "Keywords",
            "LitStr",
            "BlockCommentOrDoc",
            "Expr",
            "ExprNoTopAlt",
            "ExprWithoutSuffix",
            "ExprWithSuffix",
            "Repeat",
            "Range",
            "SetItem",
            "Stmt",
            "Item",
            "Crate",
        ] {
            assert!(names.iter().any(|held| held == name), "{name}");
        }

        // The file describes itself, so its own documents are inner ones.
        assert_eq!(file.inner_docs.len(), 1);
        assert_eq!(
            file.inner_docs[0].1.to_span(),
            Span::Range(0..21),
            "the document that opens the spec"
        );
    }
}

//! The concrete syntax tree of the DSL: the twelve `syntax` rules that
//! `unsyn.us` beside this crate writes.
//!
//! One production, one type, named for the left side of the production. A type
//! holds the operands of its right side in the order they are written, and holds
//! them as the nodes they name, so the tree is a view of the source rather than a
//! summary of it: every character of a file can be found again in the tree, which
//! is what lets a diagnostic point at the text it talks about.
//!
//! A group on the right side of a production is usually no node of its own, since
//! nothing names it: `X+` is [`OneOrMore`], `X*` is a [`Vec`], `('a' 'b')` is a
//! tuple, and `('a' ',')*` is [`SetMember`]. What earns a type is a name on the
//! *left* side, and the code that reads a tree reads the names. The two
//! exceptions are [`Set`] and [`RepeatBound`], which name a group only because
//! reading one means matching on it or asking it questions either way.
//!
//! # Where the trivia goes
//!
//! No node of this module keeps a run of the trivia: the terminals a `syntax`
//! rule is written with — `';'`, `'|'`, `'concat'` — hold their own text and
//! nothing else, and the run between two operands is a field of the sequence
//! that reads them. See the [crate documentation](crate) for the rule, and for
//! why a run is filed away exactly once however two neighbours it lies between.
//!
//! The one place the two levels meet is a `lexer` rule reached by name, such as
//! [`Ident`] or [`LitStr`]: it keeps no trivia either, since its text is read
//! into the text of the rule that reaches it.

use parserc::{ControlFlow, Kind, syntax::Syntax};

use crate::compiler::lexer::{Ident, LeadingKeywords, LitDec, LitStr, LitUnicode, S};
use crate::compiler::{
    errors::{PunctKind, SemanticsKind, SyntaxKind, UnsynError},
    input::UnsynInput,
};

/// A node with the run of trivia that stands before it.
///
/// The run between two elements of a repetition is part of neither element, so
/// the repetition keeps it beside the element it precedes rather than around it.
pub type Led<I, N> = (Option<S<I>>, N);

// ---------------------------------------------------------------------------
// The terminals a `syntax` rule is written with.
//
// Each is one fixed word and nothing more: a definition may be written `A| B`
// as much as `A |B`, and the run that makes the two readings the same reading
// is the sequence's own field rather than a part of the word. A terminal
// therefore never fails for a missing separator; it fails only when its own
// text is not there.
// ---------------------------------------------------------------------------

/// Declares one terminal of the syntax level: a fixed word and nothing else.
macro_rules! syntax_terminal {
    (
        $(#[$meta:meta])*
        $name:ident, $value:literal, $kind:ident :: $variant:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name<I>(
            /// The word itself, kept whole — the runs around a terminal belong
            /// to the sequence it stands in, exactly as the generated code
            /// lays them out.
            #[parserc(
                keyword = $value,
                map_err = crate::compiler::errors::$kind::$variant.map()
            )]
            pub I,
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
/// The word is the text the spec writes and nothing more: `exceptKeywords` is
/// read as the word `except` and the name `Keywords`, since it is the file,
/// not the reader, that says where a word of the pattern ends.
macro_rules! syntax_word {
    (
        $(#[$meta:meta])*
        $name:ident, $value:literal, $kind:ident :: $variant:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name<I>(
            /// The word itself; the runs beside it belong to the sequence.
            #[parserc(
                keyword = $value,
                map_err = crate::compiler::errors::$kind::$variant.map()
            )]
            pub I,
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
/// operands is part of the tree: it is what tells them apart. The run before an
/// operand that begins with one of its own — the match of a repetition, the
/// element of an optional group — is that operand's field and not this one's,
/// which is what leaves a field of `rest` `None`.
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
    pub rest: Vec<Led<I, X>>,
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
        std::iter::once(&self.first).chain(self.rest.iter().map(|(_, _, _, branch)| branch))
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
///
/// The runs of trivia the sequence holds between its operands are fields of the
/// tuple, as [`Led`] says for the one case a repetition has to name.
pub type Alternative<I> = (
    Option<S<I>>,
    Or<I>,
    Option<S<I>>,
    OneOrMore<I, ExprNoTopAlt<I>>,
);

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

/// The members of a set, each written with its own comma.
///
/// The repetition `( SetItem ',' )` names no node of the tree, so the tuple
/// stands for it as the production writes it: the run before the member, the
/// member, the run before the comma, and the comma.
pub type SetMember<I> = (Option<S<I>>, SetItem<I>, Option<S<I>>, Comma<I>);

/// `syntax Set -> '[' ( SetItem ',' )* SetItem? ']';`: a list of members, most
/// of them followed by a comma, and a member the closing bracket may keep
/// without one.
///
/// The brackets and the members are fields of the sequence, and the runs
/// between them are the sequence's own — the shape the generated code gives the
/// same production. A rule of its own because a pattern is asked of a set in two
/// ways, as the values it denotes and as the one character `~` refuses.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Set<I>
where
    I: UnsynInput,
{
    /// The `[` that opens the set.
    pub open: BracketStart<I>,
    /// The members each followed by a comma, with the runs about them.
    pub items: Vec<SetMember<I>>,
    /// The member after the last comma, when one is written.
    pub tail: Option<Led<I, SetItem<I>>>,
    /// The run before the closing bracket.
    pub s: Option<S<I>>,
    /// The `]` that closes the set.
    pub close: BracketEnd<I>,
}

impl<I: UnsynInput> Set<I> {
    /// The members of the set, in the order they were written.
    pub fn members(&self) -> impl Iterator<Item = &SetItem<I>> {
        self.items
            .iter()
            .map(|(_, item, _, _)| item)
            .chain(self.tail.iter().map(|(_, item)| item))
    }

    /// How many members the set lists.
    pub fn len(&self) -> usize {
        self.items.len() + usize::from(self.tail.is_some())
    }

    /// Whether the set lists no member at all.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An operand that stands alone, with no suffix on it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExprWithoutSuffix<I>
where
    I: UnsynInput,
{
    /// `'~' Set`, the one character a set does not denote.
    ///
    /// What `~` asks is answered by the character here, and takes it, so the
    /// complement fails where the set would have fitted and always consumes
    /// input. That is what lets a line document read to the end of its line with
    /// `~[LF,CR]*` rather than stopping short of it.
    ///
    /// The punct and the set are two operands of one sequence, so the run
    /// between them is the variant's own field: `'~' [' a ']'` is written with
    /// the space, and the tree has to keep it.
    Tilde {
        /// The `~` punct.
        tilde: Tilde<I>,
        /// The run between the punct and the set.
        s: Option<S<I>>,
        /// The negated set, whose members are the values refused here.
        set: Set<I>,
    },
    /// `'<' Ident '>'`, a call of an external the caller of the parser binds,
    /// read as the sequence of its parts.
    Call {
        /// The `<` that opens the call.
        lt: Lt<I>,
        /// The run the call reads after the `<`.
        s: Option<S<I>>,
        /// The name the caller binds.
        ident: Ident<I>,
        /// The run before the `>`.
        s2: Option<S<I>>,
        /// The `>` that closes the call.
        gt: Gt<I>,
    },
    /// `'(' Expr ')'`, a group, so that an expression may sit inside a sequence
    /// or take a suffix as one operand.
    Paren {
        /// The `(` that opens the group.
        open: ParenStart<I>,
        /// The run the group reads after the `(`.
        s: Option<S<I>>,
        /// The expression the group holds.
        expr: Box<Expr<I>>,
        /// The run before the `)`.
        s2: Option<S<I>>,
        /// The `)` that closes the group.
        close: ParenEnd<I>,
    },
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

/// The four quantity operators: `syntax Suffix -> '*' | '?' | '+' | Repeat;`.
///
/// A rule of its own because a reader of an operand with a suffix has to tell
/// the four operators apart, which is what the left side of a production names
/// to earn a type.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Suffix<I>
where
    I: UnsynInput,
{
    /// `*`, the operand any number of times.
    Star(Star<I>),
    /// `?`, the operand at most once.
    Question(Question<I>),
    /// `+`, the operand one or more times.
    Plus(Plus<I>),
    /// `{…}`, the operand a bounded number of times.
    Repeat(Repeat<I>),
}

/// An operand carrying one suffix.
///
/// The quantity operators are the [`Suffix`] rule the spec names, so they are
/// one operand of this sequence and not a group the production leaves for the
/// reader to unpack.
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
        /// The run between the operand and its operator.
        s: Option<S<I>>,
        /// The operator the group picked.
        suffix: Suffix<I>,
    },
    /// `concat`, which asks that what the two operands matched be joined into one
    /// token. Both operands are matched, the second one from where the first one
    /// ended.
    Concat {
        /// What is joined to the operand after the keyword.
        target: ExprWithoutSuffix<I>,
        /// The run between the operand and the keyword.
        s: Option<S<I>>,
        /// The keyword, which keeps no run of its own.
        #[parserc(crucial)]
        keyword: Concat<I>,
        /// The run between the keyword and the operand after it.
        s2: Option<S<I>>,
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
        /// The run between the operand and the keyword.
        s: Option<S<I>>,
        /// The keyword, which keeps no run of its own.
        #[parserc(crucial)]
        keyword: Followed<I>,
        /// The run between the keyword and the operand after it.
        s2: Option<S<I>>,
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
        /// The run between the operand and the keyword.
        s: Option<S<I>>,
        /// The keyword, which keeps no run of its own.
        #[parserc(crucial)]
        keyword: Except<I>,
        /// The run between the keyword and the operand taken out.
        s2: Option<S<I>>,
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
/// The operand it follows is the one that says what gets counted, and what the
/// braces hold is named by [`RepeatBound`].
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Repeat<I>
where
    I: UnsynInput,
{
    /// The `{` that opens the bound.
    pub open: BraceStart<I>,
    /// The run the repetition reads after the `{`.
    pub s: Option<S<I>>,
    /// The bounds the braces hold.
    pub bound: RepeatBound<I>,
    /// The run before the `}`.
    pub s2: Option<S<I>>,
    /// The `}` that closes the bound.
    pub close: BraceEnd<I>,
}

/// The bounds a [`Repeat`] is written with: `syntax RepeatBound -> ',' LitDec
/// | LitDec ',' LitDec? | LitDec;`.
///
/// The spec names this one because a reader of a pattern has to tell the three
/// forms apart: the comma form comes before the bare count, which is what lets
/// `{2, 3}` be two bounds rather than a count of two followed by a brace that
/// finds a comma and fails fatally.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RepeatBound<I>
where
    I: UnsynInput,
{
    /// `,n`, at most `n` times.
    RangeTo(
        /// The run before the comma, as the sequence says it.
        Option<S<I>>,
        /// The `,` that opens the form, and says which form this is.
        Comma<I>,
        /// The run between the comma and the bound.
        Option<S<I>>,
        /// The bound above.
        Box<LitDec<I>>,
    ),
    /// `n,m`, or `n,` with no bound above.
    RangeFrom(
        /// The bound below.
        Box<LitDec<I>>,
        /// The run before the comma.
        Option<S<I>>,
        /// The `,` that divides the two bounds.
        Comma<I>,
        /// The bound above with the run before it, if one is written.
        Option<Led<I, Box<LitDec<I>>>>,
    ),
    /// `n`, exactly `n` times.
    Count(
        /// The count.
        LitDec<I>,
    ),
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
        Box<LitUnicode<I>>,
        /// The run before the dash.
        Option<S<I>>,
        /// The `-` that divides the two ends.
        #[parserc(crucial)]
        Minus<I>,
        /// The run before the end above.
        Option<S<I>>,
        Box<LitUnicode<I>>,
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
    /// The run between the name and the `->`.
    pub s1: Option<S<I>>,
    /// The `->` that opens the definition.
    pub arrow: ArrowRight<I>,
    /// The run between the `->` and the pattern.
    pub s2: Option<S<I>>,
    /// The pattern the definition gives its node.
    pub expr: Box<Expr<I>>,
    /// The run before the `;`.
    pub s3: Option<S<I>>,
    /// The `;` that ends the definition.
    pub semi: Semi<I>,
}

impl<I: UnsynInput> Stmt<I> {
    /// The kind of node this definition declares.
    ///
    /// The word the definition opens with is the only thing that says so, and it
    /// is the variant of [`LeadingKeywords`] that was read — the three words of
    /// the spec are the three alternatives of that rule.
    #[inline]
    pub fn decl(&self) -> Decl {
        match self.keyword {
            LeadingKeywords::Lexer(_) => Decl::Lexer,
            LeadingKeywords::Syntax(_) => Decl::Syntax,
            LeadingKeywords::Whitespace(_) => Decl::Whitespace,
        }
    }
}

/// The kind of node a definition declares, which is the word it opens with.
///
/// Which of the three words means which is a fact about the DSL rather than
/// about the language a file of it describes, in the same way that the shapes of
/// [`Expr`] and [`Stmt`] are: the nodes of the tree name the constructs, and the
/// variant of [`LeadingKeywords`] the definition opens with picks this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decl {
    /// `whitespace`: the trivia the language separates its nodes with.
    Whitespace,
    /// `lexer`: a terminal, a pattern over characters.
    Lexer,
    /// `syntax`: a node of the tree, a pattern over other nodes.
    Syntax,
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
/// A file is read as far as its items reach, exactly as the two repetitions of
/// the rule say: a list ends where the next item fails to begin, and the
/// caller—whoever asks for the file whole—reports whatever was left. Nothing
/// here invents an end-of-file check the pattern does not state.
#[derive(Debug, PartialEq, Eq, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Crate<I>
where
    I: UnsynInput,
{
    /// The documents that describe the file, each with the run before it.
    pub inner_docs: Vec<Led<I, Box<InnerDoc<I>>>>,
    /// The items of the file, in the order they were written.
    ///
    /// The run between two definitions is itself an item — the `S` the rule
    /// `Item` lists — so the repetition carries no run of its own: a run
    /// laid before an item would starve the item meant to keep it.
    pub items: Vec<Item<I>>,
}

#[cfg(test)]
mod tests {
    use parserc::{AsStr, Input, Span, syntax::SyntaxInput};

    use super::*;
    use crate::compiler::input::TokenStream;

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
    fn a_terminal_is_its_word_and_nothing_else() {
        // A terminal keeps no run: the CST lays the trivia between the operands
        // of a sequence out as fields of its own, exactly as the generated code
        // does for the same pattern, so every character stands where the
        // sequence says it and no terminal hides one.
        let semi = whole::<Semi<_>>(";").unwrap();
        assert_eq!(semi.0.as_str(), ";");

        // Over-reaching is not reading: what stands beside the word is left
        // for the sequence.
        assert!(whole::<Semi<_>>(" ;").is_err());
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
            ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Call { .. })
        ));

        // Only one suffix is allowed, so `A*?` is not a repetition of an option.
        assert!(whole::<ExprNoTopAlt<_>>("A*?").is_err());
    }

    #[test]
    fn a_sequence_carries_the_run_between_two_operands() {
        let expr = whole::<Expr<_>>("'a' 'b'").unwrap();

        assert_eq!(expr.branches().count(), 1);
        let (gap, operand) = &expr.first.rest.first().unwrap();
        // No operand owns a run, so the space between the two is the
        // sequence's own field.
        assert_eq!(gap.as_ref().unwrap().0.as_str(), " ");
        assert_eq!(operand.to_span(), Span::Range(4..7));

        // Two runs between three operands, each where the sequence says.
        let expr = whole::<Expr<_>>("'a' 'b' 'c'").unwrap();
        assert_eq!(expr.first.rest.len(), 2);
        assert!(expr.first.rest.iter().all(|(gap, _)| gap.is_some()));
    }

    #[test]
    fn an_alternative_is_joined_by_a_bar() {
        let expr = whole::<Expr<_>>("'a' | 'b' | 'c'").unwrap();

        assert_eq!(expr.branches().count(), 3);
        assert_eq!(expr.rest[0].3.elements().count(), 1);
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

        fn bounds(src: &str) -> RepeatBound<TokenStream<'_>> {
            whole::<Repeat<_>>(src).unwrap().bound
        }

        assert!(matches!(bounds("{3}"), Count(_)));
        assert!(matches!(bounds("{1, 6}"), RangeFrom(_, _, _, Some((_, _)))));
        assert!(matches!(bounds("{1,}"), RangeFrom(_, _, _, None)));
        assert!(matches!(bounds("{,4}"), RangeTo(_, _, _, _)));

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
            Ok(ExprWithoutSuffix::Tilde { .. })
        ));
        assert!(whole::<ExprWithoutSuffix<_>>("~'a'").is_err());
    }

    #[test]
    fn a_negation_keeps_the_run_between_its_punct_and_its_set() {
        // `~` and the set are two operands of one sequence, so a space written
        // between them is a field of the variant and not a character any
        // terminal hides — the same shape the generated tree gives the rule.
        let negated = whole::<ExprWithoutSuffix<_>>("~  ['a']").unwrap();
        let ExprWithoutSuffix::Tilde { tilde, s, set } = negated else {
            panic!("not a negation");
        };

        assert_eq!(tilde.to_span(), Span::Range(0..1));
        assert_eq!(s.unwrap().0.as_str(), "  ");
        assert_eq!(set.to_span(), Span::Range(3..8));

        // Nothing between them leaves the field empty rather than misplaced.
        let tight = whole::<ExprWithoutSuffix<_>>("~['a']").unwrap();
        let ExprWithoutSuffix::Tilde { s, .. } = tight else {
            panic!("not a negation");
        };
        assert!(s.is_none());
    }

    #[test]
    fn a_definition_is_its_keyword_its_name_and_its_pattern() {
        let source = "syntax Expr -> ExprNoTopAlt+ ( '|' ExprNoTopAlt+ )*;";
        let stmt = whole::<Stmt<_>>(source).unwrap();

        assert_eq!(stmt.keyword.word(), "syntax");
        assert_eq!(stmt.ident.text(), "Expr");
        assert_eq!(stmt.to_span(), Span::Range(0..source.len()));

        // The sequence carries every run between its operands as a field of
        // its own: the name comes after the word, the arrow after the name,
        // and the pattern after the arrow.
        assert_eq!(stmt.s.unwrap().0.as_str(), " ");
        assert_eq!(stmt.s1.unwrap().0.as_str(), " ");
        assert_eq!(stmt.s2.unwrap().0.as_str(), " ");
        // Nothing lies between a pattern and the `;` that closes it here, so
        // the last junction of the sequence stands empty.
        assert!(stmt.s3.is_none());
        // A word and a name run together is what the spec writes, not what it
        // refuses: `syntaxExpr` is the word `syntax` and then the name `Expr`,
        // and the space between them is trivia that belongs to no rule.
        assert!(whole::<Stmt<_>>("syntaxExpr -> 'a';").is_ok());
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
            4,
            "a run between two definitions is an item of its own"
        );
        // Every character is held: each run by the node that keeps it, and the
        // text a node reads by the node itself. No terminal owns a run, so a
        // run of trivia at the edge of an item stands there as an item.
        let mut covered = 0usize;
        let mut hold = |span: Span| {
            let Span::Range(range) = span else {
                panic!("a node without a range")
            };
            assert_eq!(range.start, covered, "a gap before {range:?}");
            covered = range.end;
        };

        for (run, doc) in &file.inner_docs {
            if let Some(run) = run {
                hold(run.to_span());
            }
            hold(doc.to_span());
        }

        for item in &file.items {
            match item {
                Item::Stmt(stmt) => hold(stmt.to_span()),
                Item::OuterDoc(doc) => hold(doc.to_span()),
                Item::S(s) => hold(s.to_span()),
            }
        }

        assert_eq!(covered, source.len(), "the file holds all of its text");
        assert_eq!(file.to_span(), Span::Range(0..source.len()));
    }

    #[test]
    fn text_no_item_can_read_is_left_of_the_file() {
        let source = "lexer A -> 'a';\n@";
        let mut input = TokenStream::from(source);
        let file = input
            .parse::<Crate<_>>()
            .expect("a file is read as far as its items reach");

        // The run in front of the foreign character is an item of its own, and
        // the character itself stands where it was written: the rule for a
        // file lists items and nothing else, so whoever asks for the file
        // whole is the one that reports what is left.
        assert_eq!(file.items.len(), 2);
        assert_eq!(input.to_span(), Span::Range(16..17));

        // Reading the same text whole is what refuses it.
        assert_eq!(
            whole::<Crate<_>>(source),
            Err(UnsynError::Syntax(
                SyntaxKind::Eof,
                ControlFlow::Recovable,
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
                Item::Stmt(stmt) => Some(stmt.keyword.word().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        // One `whitespace`, twenty-four `lexer` and fifteen `syntax` rules.
        assert_eq!(statements.len(), 40);
        assert_eq!(statements.iter().filter(|k| *k == "lexer").count(), 24);
        assert_eq!(statements.iter().filter(|k| *k == "syntax").count(), 15);
        assert_eq!(statements.iter().filter(|k| *k == "whitespace").count(), 1);

        // Every one of them is named, which is what says that a rule of the spec
        // and a type of this crate are the same thing under the same name.
        let names = file
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Stmt(stmt) => Some(stmt.ident.text()),
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
            "Set",
            "Suffix",
            "RepeatBound",
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

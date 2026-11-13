//! The expression that defines a node, and the statement that names it.
//!
//! Two levels of binding are at work in an expression. `|` joins alternatives and
//! binds loosest; writing operands one after another is a sequence, and binds
//! tightest. So `'a' | 'b' 'c'` is the alternative `'a'` against the sequence of
//! `'b'` then `'c'`, which is exactly how the tree reads: [`Expr::rest`] holds
//! only what a `|` introduced, and [`ExprNoTopAlts::rest`] holds the rest of the
//! operands of one alternative.
//!
//! An operand is either something that stands alone ([`ExprWithoutSuffix`]) or
//! that same thing with one suffix ([`ExprWithSuffix`]: `*`, `+`, `?`, `{…}`,
//! `except`, `followed`, `concat`). Only one suffix is allowed, and a suffix
//! takes a single operand rather than an expression, so `A except B | C` is
//! `(A except B) | C` and `A*?` is not a repetition of an option.
//!
//! A sequence has no operator of its own, which is why the whitespace between two
//! operands is part of the tree: it is what tells them apart.

use parserc::{
    BeforeSpan, ControlFlow, Kind,
    syntax::{Punctuated, Syntax},
};

use crate::{
    errors::{PunctKind, SemanticsKind, SyntaxKind, UnsynError},
    input::UnsynInput,
    lexical::{
        S,
        delimiter::{Angle, Brace, Bracket, Paren},
        ident::Ident,
        keyword::{Concat, Except, Followed, Lexer, Whitespace},
        lit::{LitDec, LitStr, LitUnicode},
        punct::{ArrowRight, Comma, DotDot, Minus, Or, Plus, Question, Semi, Star, Tilde},
    },
};

/// A statement, which defines one node of the syntax tree.
///
/// The three forms differ only by the keyword that opens them, and that keyword
/// is what decides what the node is:
///
/// * `whitespace` for the trivia a pattern may skip over, of which a file
///   declares exactly one;
/// * `lexer` for a token, matched against the source text;
/// * `syntax` for a node of the tree, built from the tokens it refers to.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Stmt<I>
where
    I: UnsynInput,
{
    Whitespace {
        /// keyword `whitespace`
        #[parserc(crucial)]
        keyword: Whitespace<I>,
        /// node name.
        ident: Ident<I>,
        /// separator punct `->`
        arrow_right: ArrowRight<I>,
        /// node definition expression
        expr: Expr<I>,
        /// Termination punct `;`
        semi: Semi<I>,
    },
    Lexer {
        /// keyword `lexer`
        #[parserc(crucial)]
        keyword: Lexer<I>,
        /// node name.
        ident: Ident<I>,
        /// separator punct `->`
        arrow_right: ArrowRight<I>,
        /// node definition expression
        expr: Expr<I>,
        /// Termination punct `;`
        semi: Semi<I>,
    },

    Syntax {
        /// keyword `syntax`
        #[parserc(crucial)]
        keyword: crate::lexical::keyword::Syntax<I>,
        /// node name.
        ident: Ident<I>,
        /// separator punct `->`
        arrow_right: ArrowRight<I>,
        /// node definition expression
        expr: Expr<I>,
        /// Termination punct `;`
        semi: Semi<I>,
    },
}

/// The expression that defines what a node matches.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = map_expr_err)]
pub struct Expr<I>
where
    I: UnsynInput,
{
    /// The first alternative, which is also the whole expression when there is
    /// no `|`.
    pub first: ExprNoTopAlts<I>,
    /// Each alternative after the first, with the `|` that introduced it.
    pub rest: Vec<(Or<I>, ExprNoTopAlts<I>)>,
}

/// Turns an expression that ran into a keyword into the statement that should
/// have ended before it.
///
/// A statement's expression is followed by `;`, so the next keyword can only mean
/// that the `;` was left out. Reporting it as a missing `;` where the expression
/// ends is more useful than reporting an unexpected keyword there: the file is
/// read as one statement glued to the next.
#[inline]
fn map_expr_err(err: UnsynError) -> UnsynError {
    match err {
        UnsynError::Semantics(SemanticsKind::Keyword, span) => {
            UnsynError::Punct(PunctKind::Semi, ControlFlow::Fatal, span.before())
        }
        _ => err,
    }
}

/// The operands of one alternative, which no `|` splits.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExprNoTopAlts<I>
where
    I: UnsynInput,
{
    /// The first operand of the sequence.
    pub first: ExprNoTopAlt<I>,
    /// The operands that follow it, each with the whitespace that separates it
    /// from the operand before it.
    ///
    /// A whitespace that the token before already swallowed leaves this `None`,
    /// as in `<ident> 'x'`, where the call carries the space in its closing `>`.
    pub rest: Vec<(Option<S<I>>, ExprNoTopAlt<I>)>,
}

/// One operand of an alternative.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = map_expr_no_top_alt_err)]
pub enum ExprNoTopAlt<I>
where
    I: UnsynInput,
{
    /// An operand with a suffix.
    WithSuffix(ExprWithSuffix<I>),
    /// An operand that stands alone.
    WithoutSuffix(ExprWithoutSuffix<I>),
}

/// Reports that no operand of an alternative matched here.
///
/// An enum that has tried every variant ends with the bare combinator error of
/// its own name, which is the one error of this node that says nothing more than
/// "no operand"; it is given the [`SyntaxKind`] the other nodes report with.
///
/// An error from inside a variant is left as it is: the operand that met it has
/// already named what it expected there, which is more than this node can say.
#[inline]
fn map_expr_no_top_alt_err(err: UnsynError) -> UnsynError {
    match err {
        UnsynError::Kind(Kind::Syntax("ExprNoTopAlt", control_flow, span)) => {
            UnsynError::Syntax(SyntaxKind::ExprNoTopAlt, control_flow, span)
        }
        err => err,
    }
}

impl<I> ExprNoTopAlt<I>
where
    I: UnsynInput,
{
    /// The name the operand refers to, where it is nothing but a bare reference
    /// to it: `A`, but neither `A*` nor `[A]` nor `(A)`.
    #[inline]
    pub fn bare_ref(&self) -> Option<&Ident<I>> {
        match self {
            ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Ident(ident)) => Some(ident),
            _ => None,
        }
    }
}

impl<I> ExprNoTopAlts<I>
where
    I: UnsynInput,
{
    /// The operands of the sequence, in the order they were written.
    #[inline]
    pub fn elements(&self) -> impl Iterator<Item = &ExprNoTopAlt<I>> {
        std::iter::once(&self.first).chain(self.rest.iter().map(|(_, expr)| expr))
    }

    /// Reads the sequence against the trivia node a file names `trivia`, giving
    /// back first the operands that are elements of their own and then the
    /// mentions of that node the sequence is separated by anyway.
    ///
    /// A rule of nodes reaches every operand after the first over a run of the
    /// trivia of the language, so writing the trivia between two of them says
    /// nothing the run does not: such a mention is a separator rather than an
    /// element, and needs no field. A sequence of nothing but mentions is a rule
    /// *about* the trivia, which has no other operand to separate, so it keeps
    /// them all and folds none.
    ///
    /// This is the one reading of a mention that both the
    /// [analysis](crate::semantics), which notes the folded ones as a
    /// [lint](crate::errors::LintKind::RedundantSeparator), and the
    /// [codegen](crate::codegen), which gives them no field, work from.
    pub fn fold_separators<'a>(
        &'a self,
        trivia: Option<&str>,
    ) -> (Vec<&'a ExprNoTopAlt<I>>, Vec<&'a Ident<I>>) {
        let mut elements: Vec<&ExprNoTopAlt<I>> = Vec::new();
        let mut mentions: Vec<&Ident<I>> = Vec::new();

        for expr in self.elements() {
            match (trivia, expr.bare_ref()) {
                (Some(trivia), Some(ident)) if ident.0.as_str() == trivia => mentions.push(ident),
                _ => elements.push(expr),
            }
        }

        // A sequence that holds nothing but mentions would be left with nothing
        // in it, which is the one case a mention is not a separator.
        if mentions.is_empty() || elements.is_empty() {
            return (self.elements().collect(), Vec::new());
        }

        (elements, mentions)
    }
}

/// An operand with one suffix.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExprWithSuffix<I>
where
    I: UnsynInput,
{
    /// `*`, the operand matches zero or more times.
    Star(
        /// the repeated operand.
        ExprWithoutSuffix<I>,
        /// the star punct `*`
        Star<I>,
    ),
    /// `?`, the operand matches at most once.
    Question(
        /// the optional operand.
        ExprWithoutSuffix<I>,
        /// the question punct `?`
        Question<I>,
    ),
    /// `+`, the operand matches one or more times.
    Plus(
        /// the repeated operand.
        ExprWithoutSuffix<I>,
        /// the plus punct `+`
        Plus<I>,
    ),
    /// `{…}`, the operand matches a bounded number of times.
    Repeat {
        /// the repeated operand.
        target: ExprWithoutSuffix<I>,
        /// the bounds, in braces
        suffix: Brace<I, Repeat<I>>,
    },
    /// `concat`, which asks the backend to join what the operands matched into
    /// one token. Both operands are matched, the second one from where the first
    /// one ended.
    Concat {
        /// what is joined to the operand after the keyword.
        target: ExprWithoutSuffix<I>,
        /// the whitespace before the keyword.
        s: Option<S<I>>,
        /// keyword `concat`,
        #[parserc(crucial)]
        keyword: Concat<I>,
        /// the operand after the keyword.
        suffix: Box<ExprNoTopAlt<I>>,
    },
    /// `followed`, which asks that the operand after the keyword come next in the
    /// source without being part of what matched here. It is therefore tested
    /// after whatever the target consumed, never at the position the target
    /// starts at.
    Followed {
        /// what the operand after the keyword must follow.
        target: ExprWithoutSuffix<I>,
        /// the whitespace before the keyword.
        s: Option<S<I>>,
        /// keyword `followed`,
        #[parserc(crucial)]
        keyword: Followed<I>,
        /// the operand after the keyword.
        suffix: Box<ExprNoTopAlt<I>>,
    },
    /// `except`, which removes what the operand after the keyword matches from
    /// the operand before it. What is removed is matched by the target as a
    /// whole, so the operand is tested at the position the target starts at,
    /// however much the target consumes.
    Except {
        /// what the operand after the keyword is taken out of.
        target: ExprWithoutSuffix<I>,
        /// the whitespace before the keyword.
        s: Option<S<I>>,
        /// keyword `except`,
        #[parserc(crucial)]
        keyword: Except<I>,
        /// the operand to take out, which cannot itself have a suffix.
        tokens: ExprWithoutSuffix<I>,
    },
}

/// An operand that stands alone.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExprWithoutSuffix<I>
where
    I: UnsynInput,
{
    /// `~ […]`, the one character a set does not denote.
    ///
    /// The operand of a complement is always a set, because what `~` matches is
    /// exactly one character: the set is asked whether it fits here, the
    /// character that answers the question is taken, and the complement fails
    /// where the set would have fitted. A complement therefore always consumes
    /// input, which is what lets a line comment read to the end of its line with
    /// `~[LF]*` rather than stopping short of it.
    Tilde(
        /// the tilde punct `~`
        Tilde<I>,
        /// the negated set, whose members are the values refused here.
        Bracket<I, Punctuated<SetItem<I>, Comma<I>>>,
    ),
    /// `<…>`, a call of a matcher the backend provides, as in `<XID_Start>`.
    Call(Angle<I, Ident<I>>),
    /// `(<expr>)`, a group, so that an expression may sit inside a sequence or
    /// take a suffix as one operand.
    Paren(Paren<I, Box<Expr<I>>>),
    /// `[…]`, a set of the characters or strings it lists.
    ///
    /// A set is a union, so the [analysis](crate::semantics) compares its
    /// members by the values they denote rather than by how they are written.
    Set(Bracket<I, Punctuated<SetItem<I>, Comma<I>>>),
    /// A literal string, as in `'a'`.
    Str(LitStr<I>),
    /// A literal code point, as in `U+0009`.
    Unicode(LitUnicode<I>),
    /// A reference to another node by name.
    Ident(Ident<I>),
}

/// One member of a set.
///
/// The four forms denote values of three kinds: one character (`'a'`, `U+0061`,
/// and the range `'a'-'a'`, which is one character too), a sequence of two or
/// more (`'/*'`), an interval of code points (`'a'-'z'`), or the set another
/// node denotes, whose shape no single file knows.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SetItem<I>
where
    I: UnsynInput,
{
    /// `a-b`, the two ends of which the semantic analysis asks to be characters.
    Range(Range<I>),
    /// A literal string.
    Str(LitStr<I>),
    /// A literal code point.
    Unicode(LitUnicode<I>),
    /// A reference to another node by name.
    Ident(Ident<I>),
}

/// The bounds of a set of characters or strings written with a `-`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Range<I>
where
    I: UnsynInput,
{
    /// A range of code points, as in `U+0041-U+005A`.
    Unicode(LitUnicode<I>, #[parserc(crucial)] Minus<I>, LitUnicode<I>),
    /// A range of strings, as in `'0'-'7'`.
    Str(LitStr<I>, #[parserc(crucial)] Minus<I>, LitStr<I>),
}

/// The bounds of a repetition, between the braces that follow an operand.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Repeat<I>
where
    I: UnsynInput,
{
    /// `..n`, at most `n` times.
    RangeTo(DotDot<I>, LitDec<I>),
    /// `n..m`, or `n..` with no bound above.
    RangeFrom(LitDec<I>, DotDot<I>, Option<LitDec<I>>),
    /// `n`, exactly `n` times.
    Count(LitDec<I>),
}

#[cfg(test)]
mod tests {
    use parserc::{Span, syntax::SyntaxInput};

    use crate::{
        input::TokenStream,
        lexical::{
            lit::StrSegment,
            punct::{BraceEnd, BraceStart, BracketEnd, BracketStart},
        },
    };

    use super::*;

    /// `\'a\' | \'b\' \'c\'` is one alternative against a sequence of two operands.
    #[test]
    fn an_alternative_binds_looser_than_a_sequence() {
        let expr = TokenStream::from("'a' | 'b' 'c'")
            .parse::<Expr<_>>()
            .expect("an alternative of two operands should parse");

        // The `|` splits at the top level, and only there.
        assert_eq!(expr.rest.len(), 1);
        assert_eq!(expr.first.to_span(), Span::Range(0..3));

        // The alternative it introduces keeps both of its operands.
        let second = &expr.rest[0].1;

        assert_eq!(second.first.to_span(), Span::Range(6..9));
        assert_eq!(second.rest.len(), 1);
        assert_eq!(second.to_span(), Span::Range(6..13));
        assert_eq!(expr.to_span(), Span::Range(0..13));

        // A sequence has no operator, so the whitespace between its operands is
        // what separates them, and is kept for that reason.
        assert_eq!(second.rest[0].0, Some(S(TokenStream::from((9, " ")))));
    }

    /// Whitespace belongs to whichever of the two operands reached for it first.
    #[test]
    fn a_sequence_keeps_one_copy_of_the_whitespace_it_is_written_with() {
        // The `>` of a call already swallows the space after it, so the operand
        // that follows is written with no whitespace of its own.
        let expr = TokenStream::from("<ident> \'x\'")
            .parse::<Expr<_>>()
            .expect("a call then a string should parse");

        assert_eq!(expr.first.rest.len(), 1);
        assert_eq!(expr.first.rest[0].0, None);
        assert_eq!(expr.first.rest[0].1.to_span(), Span::Range(8..11));
    }

    /// The three repetition punctuations differ only in the punct they carry.
    #[test]
    fn a_repetition_punctuation_follows_one_operand() {
        for src in ["A?", "A*", "A+"] {
            let expr = TokenStream::from(src)
                .parse::<Expr<_>>()
                .expect("an operand with a suffix should parse");

            let ExprNoTopAlt::WithSuffix(suffix) = &expr.first.first else {
                panic!("{src}: {:?}", expr.first.first);
            };

            // The operand comes first, and the suffix is the one that ends it.
            assert_eq!(suffix.to_span(), Span::Range(0..2), "{src}");
        }

        assert_eq!(
            TokenStream::from("A?").parse::<Expr<_>>(),
            Ok(Expr {
                first: ExprNoTopAlts {
                    first: ExprNoTopAlt::WithSuffix(ExprWithSuffix::Question(
                        ExprWithoutSuffix::Ident(Ident(TokenStream::from((0, "A")))),
                        Question(None, TokenStream::from((1, "?")), None),
                    )),
                    rest: vec![],
                },
                rest: vec![],
            })
        );
    }

    /// A suffix takes a single operand, so a second one is not part of it.
    #[test]
    fn one_operand_carrys_at_most_one_suffix() {
        // `?` cannot suffix the repetition that precedes it. The expression ends
        // at the `*`, and what follows it is no longer an expression.
        let expr = TokenStream::from("A*?")
            .parse::<Expr<_>>()
            .expect("the repetition should parse on its own");

        assert_eq!(expr.to_span(), Span::Range(0..2));

        // Two `except` in a row are the same mistake, and here the second one is
        // a keyword, so the statement is told where its `;` should have been.
        assert_eq!(
            TokenStream::from("lexer A -> A except B except C;").parse::<Stmt<_>>(),
            Err(UnsynError::Punct(
                PunctKind::Semi,
                ControlFlow::Fatal,
                Span::Range(21..21)
            ))
        );
    }

    /// `except` reaches only the operand after it, so a `|` is above it.
    #[test]
    fn a_suffix_binds_tighter_than_an_alternative() {
        let expr = TokenStream::from("A except B | C")
            .parse::<Expr<_>>()
            .expect("an exception of one alternative should parse");

        assert_eq!(expr.first.to_span(), Span::Range(0..10));
        assert_eq!(expr.rest.len(), 1);
        assert_eq!(expr.rest[0].1.to_span(), Span::Range(13..14));

        // To hold the alternatives inside the exception they need a group.
        let grouped = TokenStream::from("A except (B | C)")
            .parse::<Expr<_>>()
            .expect("a group should be one operand");

        assert_eq!(grouped.first.to_span(), Span::Range(0..16));
    }

    /// The bounds of a repetition are decimals, and a missing one of them is not
    /// read as zero.
    #[test]
    fn a_repetition_is_bounded_by_decimals() {
        let bounded = [
            ("A{2}", Span::Range(1..4)),
            ("A{1..3}", Span::Range(1..7)),
            ("A{..3}", Span::Range(1..6)),
            ("A{1..}", Span::Range(1..6)),
        ];

        for (src, braces) in bounded {
            let expr = TokenStream::from(src)
                .parse::<Expr<_>>()
                .expect("a bounded repetition should parse");

            let ExprNoTopAlt::WithSuffix(ExprWithSuffix::Repeat { target, suffix }) =
                &expr.first.first
            else {
                panic!("{src}: {:?}", expr.first.first);
            };

            assert_eq!(target.to_span(), Span::Range(0..1), "{src}");
            assert_eq!(suffix.to_span(), braces, "{src}");
        }

        // The three forms keep their decimals and their `..` apart.
        assert_eq!(
            TokenStream::from("A{1..}").parse::<Expr<_>>(),
            Ok(Expr {
                first: ExprNoTopAlts {
                    first: ExprNoTopAlt::WithSuffix(ExprWithSuffix::Repeat {
                        target: ExprWithoutSuffix::Ident(Ident(TokenStream::from((0, "A")))),
                        suffix: Brace {
                            start: BraceStart(None, TokenStream::from((1, "{")), None),
                            end: BraceEnd(None, TokenStream::from((5, "}")), None),
                            body: Repeat::RangeFrom(
                                LitDec(TokenStream::from((2, "1"))),
                                DotDot(None, TokenStream::from((3, "..")), None),
                                None,
                            ),
                        },
                    }),
                    rest: vec![],
                },
                rest: vec![],
            })
        );
    }

    /// A set lists its members between brackets.
    #[test]
    fn a_set_lists_members_separated_by_commas() {
        let expr = TokenStream::from("[\'a\', \'0\'-\'7\', B]")
            .parse::<Expr<_>>()
            .expect("a set of three members should parse");

        let ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Set(set)) = &expr.first.first else {
            panic!("{:?}", expr.first.first);
        };

        assert_eq!(set.body.pairs.len(), 2);
        assert!(set.body.tail.is_some());
        assert_eq!(set.to_span(), Span::Range(0..17));
    }

    /// A statement is its keyword, its name, an arrow, an expression and a `;`.
    #[test]
    fn a_statement_is_a_name_an_expression_and_a_semicolon() {
        assert_eq!(
            TokenStream::from("lexer OCT_DIGIT -> [\'0\'-\'7\'];").parse::<Stmt<_>>(),
            Ok(Stmt::Lexer {
                keyword: Lexer(
                    TokenStream::from((0, "lexer")),
                    Some(S(TokenStream::from((5, " "))))
                ),
                ident: Ident(TokenStream::from((6, "OCT_DIGIT"))),
                arrow_right: ArrowRight(
                    Some(S(TokenStream::from((15, " ")))),
                    TokenStream::from((16, "->")),
                    Some(S(TokenStream::from((18, " ")))),
                ),
                expr: Expr {
                    first: ExprNoTopAlts {
                        first: ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Set(Bracket {
                            start: BracketStart(None, TokenStream::from((19, "[")), None),
                            end: BracketEnd(None, TokenStream::from((27, "]")), None),
                            body: Punctuated {
                                pairs: vec![],
                                tail: Some(Box::new(SetItem::Range(Range::Str(
                                    LitStr {
                                        delimiter_start: TokenStream::from((20, "\'")),
                                        content: vec![StrSegment::CharWithException(
                                            TokenStream::from((21, "0")),
                                        )],
                                        delimiter_end: TokenStream::from((22, "\'")),
                                    },
                                    Minus(None, TokenStream::from((23, "-")), None),
                                    LitStr {
                                        delimiter_start: TokenStream::from((24, "\'")),
                                        content: vec![StrSegment::CharWithException(
                                            TokenStream::from((25, "7")),
                                        )],
                                        delimiter_end: TokenStream::from((26, "\'")),
                                    },
                                )))),
                            },
                        })),
                        rest: vec![],
                    },
                    rest: vec![],
                },
                semi: Semi(None, TokenStream::from((28, ";")), None),
            })
        );
    }

    /// A statement that never reaches its `;` is reported as missing one.
    ///
    /// The expression of a statement ends at the token that cannot continue it,
    /// and `map_expr_err` turns a keyword found there into the `;` that should
    /// have come before it, because that is the only reason a keyword can be
    /// there.
    #[test]
    fn a_statement_without_a_semicolon_is_told_where_one_belongs() {
        assert_eq!(
            TokenStream::from("lexer A -> \'a\' lexer B -> \'b\';").parse::<Stmt<_>>(),
            Err(UnsynError::Punct(
                PunctKind::Semi,
                ControlFlow::Fatal,
                Span::Range(14..14)
            ))
        );

        // A `;` may be separated from its statement, which is not the same mistake.
        assert!(
            TokenStream::from("lexer A -> ['0'-'7'] ;")
                .parse::<Stmt<_>>()
                .is_ok()
        );
    }

    /// An expression that has no operand at all is reported as the node that is
    /// missing one, rather than as whatever combinator gave up first.
    #[test]
    fn an_expression_of_nothing_is_told_what_it_lacks() {
        assert_eq!(
            TokenStream::from("lexer OCT -> ").parse::<Stmt<_>>(),
            Err(UnsynError::Syntax(
                SyntaxKind::ExprNoTopAlt,
                ControlFlow::Fatal,
                Span::Range(13..13),
            )),
        );

        // The operand after a `|` is only one alternative of the expression: it
        // fails softly, the expression ends with the operand before it, and the
        // `|` is then told as the `;` the statement is missing.
        assert_eq!(
            TokenStream::from("lexer OCT -> 'a' |").parse::<Stmt<_>>(),
            Err(UnsynError::Punct(
                PunctKind::Semi,
                ControlFlow::Fatal,
                Span::Range(17..18),
            )),
        );
    }
}

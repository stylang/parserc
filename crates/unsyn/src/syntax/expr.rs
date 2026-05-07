use parserc::{
    ControlFlow,
    syntax::{Punctuated, Syntax},
};

use crate::{
    errors::{PunctKind, SemanticsKind, CompileError},
    input::UnsynInput,
    syntax::Path,
    token::{
        S,
        delimiter::{Angle, Brace, Bracket, Paren},
        ident::Ident,
        keyword::{Concat, Except, Followed, Lexer, Whitespace},
        lit::{LitDec, LitStr, LitUnicode},
        punct::{ArrowRight, Comma, DotDot, Minus, Or, Plus, Question, Semi, Star, Tilde},
    },
};

/// A stmt define a node of syntax tree.
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
        keyword: crate::token::keyword::Syntax<I>,
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

impl<I> Stmt<I>
where
    I: UnsynInput,
{
    /// Get ident of the statement.
    pub fn ident(&self) -> &Ident<I> {
        match self {
            Stmt::Whitespace {
                keyword: _,
                ident,
                arrow_right: _,
                expr: _,
                semi: _,
            } => ident,
            Stmt::Lexer {
                keyword: _,
                ident,
                arrow_right: _,
                expr: _,
                semi: _,
            } => ident,
            Stmt::Syntax {
                keyword: _,
                ident,
                arrow_right: _,
                expr: _,
                semi: _,
            } => ident,
        }
    }
}

/// Node definition expression.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = map_expr_err)]
pub struct Expr<I>
where
    I: UnsynInput,
{
    pub first: ExprNoTopAlts<I>,
    pub rest: Vec<(Or<I>, ExprNoTopAlts<I>)>,
}

#[inline]
fn map_expr_err(err: CompileError) -> CompileError {
    match err {
        CompileError::Semantics(SemanticsKind::Keyword, span) => {
            CompileError::Punct(PunctKind::Semi, ControlFlow::Fatal, span)
        }
        _ => err,
    }
}

/// No top alt expressions list.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExprNoTopAlts<I>
where
    I: UnsynInput,
{
    /// first expr,
    pub first: ExprNoTopAlt<I>,
    /// rest expr list.
    pub rest: Vec<(Option<S<I>>, ExprNoTopAlt<I>)>,
}

/// No top alt expression.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExprNoTopAlt<I>
where
    I: UnsynInput,
{
    WithSuffix(ExprWithSuffix<I>),
    WithoutSuffix(ExprWithoutSuffix<I>),
}

/// Expr with suffix.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExprWithSuffix<I>
where
    I: UnsynInput,
{
    /// star expr.
    Star(
        /// target expr.
        ExprWithoutSuffix<I>,
        /// start punct `*`
        Star<I>,
    ),
    Question(
        /// target expr.
        ExprWithoutSuffix<I>,
        /// question punct `?`
        Question<I>,
    ),
    Plus(
        /// target expr.
        ExprWithoutSuffix<I>,
        /// plus punct `+`
        Plus<I>,
    ),
    Repeat {
        /// target expr.
        target: ExprWithoutSuffix<I>,
        suffix: Brace<I, Repeat<I>>,
    },
    /// Concat right operand
    Concat {
        /// target expr.
        target: ExprWithoutSuffix<I>,
        /// prefix whitespace.
        s: Option<S<I>>,
        /// keyword `followed`,
        #[parserc(crucial)]
        keyword: Concat<I>,
        /// suffix expr.
        suffix: Box<ExprNoTopAlt<I>>,
    },
    /// a followed expression
    Followed {
        /// target expr.
        target: ExprWithoutSuffix<I>,
        /// prefix whitespace.
        s: Option<S<I>>,
        /// keyword `followed`,
        #[parserc(crucial)]
        keyword: Followed<I>,
        /// suffix expr.
        suffix: Box<ExprNoTopAlt<I>>,
    },
    /// A except expression.
    Except {
        /// target expr.
        target: ExprWithoutSuffix<I>,
        /// prefix whitespace.
        s: Option<S<I>>,
        /// keyword `except`,
        #[parserc(crucial)]
        keyword: Except<I>,
        /// expect tokens.
        tokens: ExprWithoutSuffix<I>,
    },
}

/// Expr without suffix.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExprWithoutSuffix<I>
where
    I: UnsynInput,
{
    /// tilde expr.
    Tilde(
        /// tilde punct `~`
        Tilde<I>,
        /// target expr.
        Box<ExprWithoutSuffix<I>>,
    ),
    Call(Angle<I, Ident<I>>),
    /// paren expr `(T)`
    Paren(Paren<I, Box<Expr<I>>>),
    /// a set expression,
    Set(Bracket<I, Punctuated<SetItem<I>, Comma<I>>>),
    /// A literal string expr.
    Str(LitStr<I>),
    /// A literal unicode expr.
    Unicode(LitUnicode<I>),
    /// A path expression.
    Path(Path<I>),
}

/// expr for set item.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SetItem<I>
where
    I: UnsynInput,
{
    /// a range expr.
    Range(Range<I>),
    /// A literal string expr.
    Str(LitStr<I>),
    /// A literal unicode expr.
    Unicode(LitUnicode<I>),
    /// A path expression.
    Path(Path<I>),
}

/// A literal range expression
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Range<I>
where
    I: UnsynInput,
{
    Unicode(LitUnicode<I>, #[parserc(crucial)] Minus<I>, LitUnicode<I>),
    Char(LitStr<I>, #[parserc(crucial)] Minus<I>, LitStr<I>),
}

/// The suffix of repeat expresison.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Repeat<I>
where
    I: UnsynInput,
{
    RangeTo(DotDot<I>, LitDec<I>),
    Range(LitDec<I>, DotDot<I>, LitDec<I>),
    RangeFrom(LitDec<I>, DotDot<I>),
    Times(LitDec<I>),
}

#[cfg(test)]
mod tests {

    use parserc::{
        sourceinput::Source,
        syntax::{Delimiter, SyntaxExt},
    };

    use super::*;

    use crate::{
        input::Chars,
        syntax::PathSegment,
        token::{
            lit::StrSegment,
            punct::{BracketEnd, BracketStart, ParenEnd, ParenStart},
        },
    };

    #[test]
    fn test_question() {
        assert_eq!(
            Chars::new("IDENTIFIER ( '=' ( STRING_LITERAL | RAW_STRING_LITERAL ) )?")
                .parse::<Expr<_>>(),
            Ok(Expr {
                first: ExprNoTopAlts {
                    first: ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Path(Path {
                        leading_sep: None,
                        first: PathSegment::Ident(Ident(Source::new_offset(0, "IDENTIFIER"))),
                        rest: vec![]
                    })),
                    rest: vec![(
                        Some(S(Source::new_offset(10, " "))),
                        ExprNoTopAlt::WithSuffix(ExprWithSuffix::Question(
                            ExprWithoutSuffix::Paren(Delimiter {
                                start: ParenStart(
                                    None,
                                    Source::new_offset(11, "("),
                                    Some(S(Source::new_offset(12, " ")))
                                ),
                                end: ParenEnd(None, Source::new_offset(57, ")"), None),
                                body: Box::new(Expr {
                                    first: ExprNoTopAlts {
                                        first: ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Str(
                                            LitStr {
                                                delimiter_start: Source::new_offset(13, "'"),
                                                content: vec![StrSegment::CharsWithException(
                                                    Source::new_offset(14, "=")
                                                )],
                                                delimiter_end: Source::new_offset(15, "'")
                                            }
                                        )),
                                        rest: vec![(
                                            Some(S(Source::new_offset(16, " "))),
                                            ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Paren(
                                                Delimiter {
                                                    start: ParenStart(
                                                        None,
                                                        Source::new_offset(17, "("),
                                                        Some(S(Source::new_offset(18, " ")))
                                                    ),
                                                    end: ParenEnd(
                                                        Some(S(Source::new_offset(54, " "))),
                                                        Source::new_offset(55, ")"),
                                                        Some(S(Source::new_offset(56, " ")))
                                                    ),
                                                    body: Box::new(Expr {
                                                        first: ExprNoTopAlts {
                                                            first: ExprNoTopAlt::WithoutSuffix(
                                                                ExprWithoutSuffix::Path(Path {
                                                                    leading_sep: None,
                                                                    first: PathSegment::Ident(
                                                                        Ident(Source::new_offset(
                                                                            19,
                                                                            "STRING_LITERAL"
                                                                        ))
                                                                    ),
                                                                    rest: vec![]
                                                                })
                                                            ),
                                                            rest: vec![]
                                                        },
                                                        rest: vec![(
                                                            Or(
                                                                Some(S(Source::new_offset(
                                                                    33, " "
                                                                ))),
                                                                Source::new_offset(34, "|"),
                                                                Some(S(Source::new_offset(
                                                                    35, " "
                                                                )))
                                                            ),
                                                            ExprNoTopAlts {
                                                                first: ExprNoTopAlt::WithoutSuffix(
                                                                    ExprWithoutSuffix::Path(Path {
                                                                        leading_sep: None,
                                                                        first: PathSegment::Ident(
                                                                            Ident(
                                                                                Source::new_offset(
                                                                                    36,
                                                                                    "RAW_STRING_LITERAL"
                                                                                )
                                                                            )
                                                                        ),
                                                                        rest: vec![]
                                                                    })
                                                                ),
                                                                rest: vec![]
                                                            }
                                                        )]
                                                    })
                                                }
                                            ))
                                        )]
                                    },
                                    rest: vec![]
                                })
                            }),
                            Question(None, Source::new_offset(58, "?"), None)
                        ))
                    )]
                },
                rest: vec![]
            })
        );
    }

    #[test]
    fn test_stmt() {
        assert_eq!(
            Chars::new(r#"lexer OCT_DIGIT -> ['0'-'7'];"#).parse::<Stmt<_>>(),
            Ok(Stmt::Lexer {
                keyword: Lexer(
                    Source::new_offset(0, "lexer"),
                    Some(S(Source::new_offset(5, " ")))
                ),
                ident: Ident(Source::new_offset(6, "OCT_DIGIT")),
                arrow_right: ArrowRight(
                    Some(S(Source::new_offset(15, " "))),
                    Source::new_offset(16, "->"),
                    Some(S(Source::new_offset(18, " ")))
                ),
                expr: Expr {
                    first: ExprNoTopAlts {
                        first: ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Set(Delimiter {
                            start: BracketStart(None, Source::new_offset(19, "["), None),
                            end: BracketEnd(None, Source::new_offset(27, "]"), None),
                            body: Punctuated {
                                pairs: vec![],
                                tail: Some(Box::new(SetItem::Range(Range::Char(
                                    LitStr {
                                        delimiter_start: Source::new_offset(20, "'"),
                                        content: vec![StrSegment::CharsWithException(
                                            Source::new_offset(21, "0")
                                        )],
                                        delimiter_end: Source::new_offset(22, "'")
                                    },
                                    Minus(None, Source::new_offset(23, "-"), None),
                                    LitStr {
                                        delimiter_start: Source::new_offset(24, "'"),
                                        content: vec![StrSegment::CharsWithException(
                                            Source::new_offset(25, "7")
                                        )],
                                        delimiter_end: Source::new_offset(26, "'")
                                    }
                                ))))
                            }
                        })),
                        rest: vec![]
                    },
                    rest: vec![]
                },
                semi: Semi(None, Source::new_offset(28, ";"), None)
            })
        );
    }
}

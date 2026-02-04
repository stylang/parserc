use parserc::{
    BeforeSpan, ControlFlow,
    syntax::{Punctuated, Syntax},
};

use crate::{
    errors::{PunctKind, SemanticsKind, UnsynError},
    input::UnsynInput,
    lexical::{
        S,
        delimiter::{Angle, Brace, Bracket, Paren},
        ident::Ident,
        keyword::{Concat, Except, Followed, Lexer, Whitespace},
        lit::{LitDec, LitStr, LitUnicode},
        punct::{ArrowRight, Comma, DotDot, Minus, Or, Plus, Question, Semi, Star, Tilde},
    },
    syntax::Path,
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
fn map_expr_err(err: UnsynError) -> UnsynError {
    match err {
        UnsynError::Semantics(SemanticsKind::Keyword, span) => {
            UnsynError::Punct(PunctKind::Semi, ControlFlow::Fatal, span.before())
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
    Str(LitStr<I>, #[parserc(crucial)] Minus<I>, LitStr<I>),
}

/// The suffix of repeat expresison.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Repeat<I>
where
    I: UnsynInput,
{
    RangeTo(DotDot<I>, LitDec<I>),
    RangeFrom(LitDec<I>, DotDot<I>, Option<LitDec<I>>),
    Count(LitDec<I>),
}

#[cfg(test)]
mod tests {
    use parserc::syntax::SyntaxInput;

    use crate::input::TokenStream;

    use super::*;

    #[test]
    fn test_question() {
        println!(
            "{:?}",
            TokenStream::from("IDENTIFIER ( '=' ( STRING_LITERAL | RAW_STRING_LITERAL ) )?")
                .parse::<Expr<_>>()
        );
    }

    #[test]
    fn test_stmt() {
        println!(
            "{:?}",
            TokenStream::from(r#"lexer OCT_DIGIT -> ['0'-'7'];"#).parse::<Stmt<_>>()
        );
    }
}

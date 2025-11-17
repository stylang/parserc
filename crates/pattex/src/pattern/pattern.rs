use parserc::Parser;
use parserc::syntax::{Delimiter, Syntax};

use crate::errors::CompileError;
use crate::input::PatternInput;
use crate::pattern::{
    Class, Dot, Escape, Or, ParenEnd, ParenStart, Plus, Question, Repeat, Star, is_token_char,
};

/// Pattern of a sequence of characters.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(
    token = |c:char| { c == '-' || !is_token_char(c) },
    map_err = CompileError::PatternChars.map()
)]
pub struct PatternChars<I>(pub I)
where
    I: PatternInput;

/// A non-root pattern sequence.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SubPattern<I>
where
    I: PatternInput,
{
    /// A sub-pattern of a sequence of characters.
    Chars(PatternChars<I>),
    /// A escape sub-pattern.
    Escap(Escape<I>),
    /// A capture of sub-pattern sequence.
    Capture(Delimiter<ParenStart<I>, ParenEnd<I>, Vec<SubPattern<I>>>),
    /// A repeat sub-pattern.
    Repeat(Repeat<I>),
    /// A start sub-pattern.
    Star(Star<I>),
    /// A question sub-pattern.
    Question(Question<I>),
    /// A plus sub-pattern.
    Plus(Plus<I>),
    /// A character class sub-pattern.
    Class(Class<I>),
    /// A '|' sub-pattern.
    Or(Or<I>),
    /// A `.` sub-pattern.
    Dot(Dot<I>),
}

/// Pattern sequence.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pattern<I>(pub Vec<SubPattern<I>>)
where
    I: PatternInput;

impl<I> Syntax<I> for Pattern<I>
where
    I: PatternInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        let mut subpatterns = vec![];
        while !input.is_empty() {
            subpatterns.push(SubPattern::into_parser().fatal().parse(input)?);
        }

        Ok(Self(subpatterns))
    }

    #[inline]
    fn to_span(&self) -> parserc::Span {
        self.0.to_span()
    }
}

#[cfg(test)]
mod tests {
    use parserc::syntax::{Char, Delimiter, InputSyntaxExt};

    use crate::{
        input::TokenStream,
        pattern::{
            BackSlash, BracketEnd, BracketStart, Caret, Class, ClassChars, Digits, Escape,
            EscapeKind, ParenEnd, ParenStart, PatternChars, Plus, Question, Repeat, Star,
            SubPattern,
        },
    };

    #[test]
    fn chars() {
        assert_eq!(
            TokenStream::from("://").parse(),
            Ok(SubPattern::Chars(PatternChars(TokenStream::from("://"))))
        );
    }

    #[test]
    fn capture() {
        assert_eq!(
            TokenStream::from("(abc)").parse(),
            Ok(SubPattern::Capture(Delimiter {
                start: ParenStart(TokenStream::from("(")),
                end: ParenEnd(TokenStream::from((4, ")"))),
                body: vec![SubPattern::Chars(PatternChars(TokenStream::from((
                    1, "abc"
                ))))]
            }))
        );
    }

    #[test]
    fn class() {
        assert_eq!(
            TokenStream::from(r"[^\f\thello0-9]*").parse(),
            Ok(vec![
                SubPattern::Class(Class(Delimiter {
                    start: BracketStart(TokenStream::from("[")),
                    end: BracketEnd(TokenStream::from((14, "]"))),
                    body: (
                        Some(Caret(TokenStream::from((1, "^")))),
                        vec![
                            ClassChars::Escape(Escape {
                                backslash: BackSlash(TokenStream::from((2, r"\"))),
                                kind: EscapeKind::FF(Char(TokenStream::from((3, "f"))))
                            }),
                            ClassChars::Escape(Escape {
                                backslash: BackSlash(TokenStream::from((4, r"\"))),
                                kind: EscapeKind::TF(Char(TokenStream::from((5, "t"))))
                            }),
                            ClassChars::Sequnce(TokenStream::from((6, "hello"))),
                            ClassChars::Range {
                                from: '0',
                                to: '9',
                                input: TokenStream::from((11, "0-9"))
                            }
                        ]
                    )
                })),
                SubPattern::Star(Star(TokenStream::from((15, "*"))))
            ])
        )
    }

    #[test]
    fn repeat() {
        assert_eq!(
            TokenStream::from("abc{2}").parse(),
            Ok(vec![
                SubPattern::Chars(PatternChars(TokenStream::from("abc"))),
                SubPattern::Repeat(Repeat::Repeat {
                    n: Digits {
                        value: 2,
                        input: TokenStream::from((4, "2"))
                    },
                    input: TokenStream::from((3, "{2}"))
                })
            ])
        );

        assert_eq!(
            TokenStream::from("abc*").parse(),
            Ok(vec![
                SubPattern::Chars(PatternChars(TokenStream::from("abc"))),
                SubPattern::Star(Star(TokenStream::from((3, "*"))))
            ])
        );

        assert_eq!(
            TokenStream::from("abc?").parse(),
            Ok(vec![
                SubPattern::Chars(PatternChars(TokenStream::from("abc"))),
                SubPattern::Question(Question(TokenStream::from((3, "?"))))
            ])
        );

        assert_eq!(
            TokenStream::from("abc+").parse(),
            Ok(vec![
                SubPattern::Chars(PatternChars(TokenStream::from("abc"))),
                SubPattern::Plus(Plus(TokenStream::from((3, "+"))))
            ])
        );
    }
}

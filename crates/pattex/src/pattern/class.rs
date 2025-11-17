use std::cmp;

use parserc::{
    ControlFlow, Parser, Span,
    syntax::{Delimiter, Syntax},
    take_while_range_from,
};

use crate::{
    errors::{CompileError, RegexError},
    input::PatternInput,
    pattern::{BracketEnd, BracketStart, Caret, Escape, is_token_char},
};

/// Char in character class.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ClassChars<I>
where
    I: PatternInput,
{
    /// Escape char sequence.
    Escape(Escape<I>),
    /// A sequence pattern chars.
    Sequnce(I),
    /// A range chars belike: `A-Z`,`0-9`
    Range { from: char, to: char, input: I },
}

impl<I> Syntax<I> for ClassChars<I>
where
    I: PatternInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        if let Some(escape) = Escape::into_parser().ok().parse(input)? {
            return Ok(Self::Escape(escape));
        }

        let mut content = input.clone();

        let sequnce = take_while_range_from(1, |c: char| !is_token_char(c))
            .parse(&mut content)
            .map_err(CompileError::CharSequence.map())?;

        let mut iter = content.iter();

        if let Some('-') = iter.next() {
            if sequnce.len() == 1 {
                let Some(to) = iter.next() else {
                    return Err(RegexError::Compile(
                        CompileError::CharRange,
                        ControlFlow::Fatal,
                        Span::Range(
                            content.start() - 1..cmp::min(content.end(), content.start() + 1),
                        ),
                    ));
                };

                let from = sequnce.iter().next().unwrap();

                if !(from < to) {
                    return Err(RegexError::Compile(
                        CompileError::CharRange,
                        ControlFlow::Fatal,
                        Span::Range(content.start() - 1..content.start() + 2),
                    ));
                }

                return Ok(Self::Range {
                    from,
                    to,
                    input: input.split_to(3),
                });
            } else {
                return Ok(Self::Sequnce(input.split_to(sequnce.len() - 1)));
            }
        }

        return Ok(Self::Sequnce(sequnce));
    }

    #[inline]
    fn to_span(&self) -> parserc::Span {
        match self {
            ClassChars::Escape(escape) => escape.to_span(),
            ClassChars::Sequnce(input) => input.to_span(),
            ClassChars::Range {
                from: _,
                to: _,
                input,
            } => input.to_span(),
        }
    }
}

/// Character class.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Class<I>(
    pub Delimiter<BracketStart<I>, BracketEnd<I>, (Option<Caret<I>>, Vec<ClassChars<I>>)>,
)
where
    I: PatternInput;

#[cfg(test)]
mod tests {
    use parserc::{
        ControlFlow, Span,
        syntax::{Char, Delimiter, InputSyntaxExt},
    };

    use crate::{
        errors::{CompileError, RegexError},
        input::TokenStream,
        pattern::{
            BackSlash, BracketEnd, BracketStart, Caret, Class, ClassChars, Escape, EscapeKind,
            FixedDigits,
        },
    };

    #[test]
    fn test_chars() {
        assert_eq!(
            TokenStream::from("1234").parse(),
            Ok(ClassChars::Sequnce(TokenStream::from("1234")))
        );

        assert_eq!(
            TokenStream::from("1234-9").parse(),
            Ok(ClassChars::Sequnce(TokenStream::from("123")))
        );

        assert_eq!(
            TokenStream::from("a-z").parse(),
            Ok(ClassChars::Range {
                from: 'a',
                to: 'z',
                input: TokenStream::from("a-z")
            })
        );

        assert_eq!(
            TokenStream::from("0-9").parse(),
            Ok(ClassChars::Range {
                from: '0',
                to: '9',
                input: TokenStream::from("0-9")
            })
        );

        assert_eq!(
            TokenStream::from(r"\123a-z").parse(),
            Ok(ClassChars::Escape(Escape {
                backslash: BackSlash(TokenStream::from((0, r"\"))),
                kind: EscapeKind::BackReference(FixedDigits(TokenStream::from((1, "12")))),
            }))
        );

        assert_eq!(
            TokenStream::from("z-a").parse::<ClassChars<_>>(),
            Err(RegexError::Compile(
                CompileError::CharRange,
                ControlFlow::Fatal,
                Span::Range(0..3)
            ))
        );

        assert_eq!(
            TokenStream::from("z-").parse::<ClassChars<_>>(),
            Err(RegexError::Compile(
                CompileError::CharRange,
                ControlFlow::Fatal,
                Span::Range(0..2)
            ))
        );
    }

    #[test]
    fn test_char_class() {
        assert_eq!(
            TokenStream::from(r"[^\f\thello0-9]").parse(),
            Ok(Class(Delimiter {
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
            }))
        )
    }
}

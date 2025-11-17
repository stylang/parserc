use parserc::ControlFlow;
use parserc::syntax::{Delimiter, InputSyntaxExt, Syntax};

use crate::errors::{CompileError, RegexError};
use crate::input::PatternInput;
use crate::pattern::{
    Caret, Class, Dollar, Dot, Escape, Or, ParenEnd, ParenStart, Plus, Question, Repeat, Star,
    is_token_char,
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
pub struct Pattern<I>
where
    I: PatternInput,
{
    /// Match the start of the input string.
    pub start: Option<Caret<I>>,
    /// A sequence of sub-patterns.
    pub patterns: Vec<SubPattern<I>>,
    /// Match the end of the input string.
    pub end: Option<Dollar<I>>,
}

impl<I> Syntax<I> for Pattern<I>
where
    I: PatternInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        let start = input.parse()?;

        let patterns = input.parse()?;

        let end = input.parse()?;

        if !input.is_empty() {
            return Err(RegexError::Compile(
                CompileError::Unparsing,
                ControlFlow::Fatal,
                input.to_span(),
            ));
        }

        Ok(Self {
            start,
            patterns,
            end,
        })
    }

    #[inline]
    fn to_span(&self) -> parserc::Span {
        self.start.to_span() + self.patterns.to_span() + self.end.to_span()
    }
}

#[cfg(test)]
mod tests {
    use parserc::syntax::{Char, Delimiter, InputSyntaxExt};

    use crate::{
        input::TokenStream,
        pattern::{
            BackSlash, BracketEnd, BracketStart, Caret, Class, ClassChars, Digits, Dollar, Dot,
            Escape, EscapeKind, Minus, Or, ParenEnd, ParenStart, Pattern, PatternChars, Plus,
            Question, Repeat, Star, SubPattern,
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

    #[test]
    fn test_pattern() {
        let pattern = r"^(http|https)://[a-zA-Z0-9\-\.]+\.[a-zA-Z]{2,3}(/\S*)?$";
        assert_eq!(
            TokenStream::from(pattern).parse(),
            Ok(Pattern {
                start: Some(Caret(TokenStream {
                    offset: 0,
                    value: "^"
                })),
                patterns: vec![
                    SubPattern::Capture(Delimiter {
                        start: ParenStart(TokenStream {
                            offset: 1,
                            value: "("
                        }),
                        end: ParenEnd(TokenStream {
                            offset: 12,
                            value: ")"
                        }),
                        body: vec![
                            SubPattern::Chars(PatternChars(TokenStream {
                                offset: 2,
                                value: "http"
                            })),
                            SubPattern::Or(Or(TokenStream {
                                offset: 6,
                                value: "|"
                            })),
                            SubPattern::Chars(PatternChars(TokenStream {
                                offset: 7,
                                value: "https"
                            }))
                        ]
                    }),
                    SubPattern::Chars(PatternChars(TokenStream {
                        offset: 13,
                        value: "://"
                    })),
                    SubPattern::Class(Class(Delimiter {
                        start: BracketStart(TokenStream {
                            offset: 16,
                            value: "["
                        }),
                        end: BracketEnd(TokenStream {
                            offset: 30,
                            value: "]"
                        }),
                        body: (
                            None,
                            vec![
                                ClassChars::Range {
                                    from: 'a',
                                    to: 'z',
                                    input: TokenStream {
                                        offset: 17,
                                        value: "a-z"
                                    }
                                },
                                ClassChars::Range {
                                    from: 'A',
                                    to: 'Z',
                                    input: TokenStream {
                                        offset: 20,
                                        value: "A-Z"
                                    }
                                },
                                ClassChars::Range {
                                    from: '0',
                                    to: '9',
                                    input: TokenStream {
                                        offset: 23,
                                        value: "0-9"
                                    }
                                },
                                ClassChars::Escape(Escape {
                                    backslash: BackSlash(TokenStream {
                                        offset: 26,
                                        value: "\\"
                                    }),
                                    kind: EscapeKind::Minus(Minus(TokenStream {
                                        offset: 27,
                                        value: "-"
                                    }))
                                }),
                                ClassChars::Escape(Escape {
                                    backslash: BackSlash(TokenStream {
                                        offset: 28,
                                        value: "\\"
                                    }),
                                    kind: EscapeKind::Dot(Dot(TokenStream {
                                        offset: 29,
                                        value: "."
                                    }))
                                },)
                            ]
                        )
                    })),
                    SubPattern::Plus(Plus(TokenStream {
                        offset: 31,
                        value: "+"
                    })),
                    SubPattern::Escap(Escape {
                        backslash: BackSlash(TokenStream {
                            offset: 32,
                            value: "\\"
                        }),
                        kind: EscapeKind::Dot(Dot(TokenStream {
                            offset: 33,
                            value: "."
                        }))
                    },),
                    SubPattern::Class(Class(Delimiter {
                        start: BracketStart(TokenStream {
                            offset: 34,
                            value: "["
                        }),
                        end: BracketEnd(TokenStream {
                            offset: 41,
                            value: "]"
                        }),
                        body: (
                            None,
                            vec![
                                ClassChars::Range {
                                    from: 'a',
                                    to: 'z',
                                    input: TokenStream {
                                        offset: 35,
                                        value: "a-z"
                                    }
                                },
                                ClassChars::Range {
                                    from: 'A',
                                    to: 'Z',
                                    input: TokenStream {
                                        offset: 38,
                                        value: "A-Z"
                                    }
                                }
                            ]
                        )
                    })),
                    SubPattern::Repeat(Repeat::Range {
                        n: Digits {
                            value: 2,
                            input: TokenStream {
                                offset: 43,
                                value: "2"
                            }
                        },
                        m: Digits {
                            value: 3,
                            input: TokenStream {
                                offset: 45,
                                value: "3"
                            }
                        },
                        input: TokenStream {
                            offset: 42,
                            value: "{2,3}"
                        }
                    }),
                    SubPattern::Capture(Delimiter {
                        start: ParenStart(TokenStream {
                            offset: 47,
                            value: "("
                        }),
                        end: ParenEnd(TokenStream {
                            offset: 52,
                            value: ")"
                        }),
                        body: vec![
                            SubPattern::Chars(PatternChars(TokenStream {
                                offset: 48,
                                value: "/"
                            })),
                            SubPattern::Escap(Escape {
                                backslash: BackSlash(TokenStream {
                                    offset: 49,
                                    value: "\\"
                                }),
                                kind: EscapeKind::NonS(Char(TokenStream {
                                    offset: 50,
                                    value: "S"
                                }))
                            },),
                            SubPattern::Star(Star(TokenStream {
                                offset: 51,
                                value: "*"
                            }))
                        ]
                    }),
                    SubPattern::Question(Question(TokenStream {
                        offset: 53,
                        value: "?"
                    }))
                ],
                end: Some(Dollar(TokenStream {
                    offset: 54,
                    value: "$"
                }))
            })
        );
    }
}

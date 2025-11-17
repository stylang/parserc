use parserc::syntax::{Char, Syntax};

use crate::{
    errors::CompileError,
    input::PatternInput,
    pattern::{
        BackSlash, BraceStart, BracketStart, Caret, Dollar, Dot, FixedDigits, FixedHexDigits,
        Minus, Or, ParenStart, Plus, Question, Star,
    },
};

/// Escape token sequence.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Escape<I>
where
    I: PatternInput,
{
    /// prefix `\`
    #[parserc(crucial)]
    pub backslash: BackSlash<I>,
    /// escape character sequence.
    pub kind: EscapeKind<I>,
}

/// Escape token sequence.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = CompileError::Escape.map())]
pub enum EscapeKind<I>
where
    I: PatternInput,
{
    /// `\\`
    BackSlash(BackSlash<I>),
    /// `\^`
    Caret(Caret<I>),
    /// `\*`
    Star(Star<I>),
    /// `\$`
    Dollar(Dollar<I>),
    /// `\?`
    Question(Question<I>),
    /// `\+`
    Plus(Plus<I>),
    /// `\-`
    Minus(Minus<I>),
    /// `\.`
    Dot(Dot<I>),
    /// `\|`
    Or(Or<I>),
    /// `\{`
    BraceStart(BraceStart<I>),
    /// `\[`
    BracketStart(BracketStart<I>),
    /// `\(`
    ParenStart(ParenStart<I>),
    ///  \b
    Boundery(Char<I, 'b'>),
    ///  \B
    NonBoundery(Char<I, 'B'>),
    ///  \d
    Digit(Char<I, 'd'>),
    ///  \D
    NonDigit(Char<I, 'D'>),
    /// \f
    FF(Char<I, 'f'>),
    /// \n
    LF(Char<I, 'n'>),
    /// \r
    CR(Char<I, 'r'>),
    ///  \s
    S(Char<I, 's'>),
    ///  \S
    NonS(Char<I, 'S'>),
    ///  \t
    TF(Char<I, 't'>),
    ///  \v
    VF(Char<I, 'v'>),
    ///  \w
    Word(Char<I, 'w'>),
    ///  \W
    NonWord(Char<I, 'W'>),
    /// backreference `\1..`
    BackReference(FixedDigits<I, 2>),
    /// \xnn
    Hex(
        #[parserc(crucial)] Char<I, 'x'>,
        #[parserc(map_err = CompileError::EscapeHex.map())] FixedHexDigits<I, 2>,
    ),
    /// \unnnn
    Unicode(
        #[parserc(crucial)] Char<I, 'u'>,
        #[parserc(map_err = CompileError::EscapeUnicode.map())] FixedHexDigits<I, 4>,
    ),
}

#[cfg(test)]
mod test {
    use parserc::{ControlFlow, Span, syntax::InputSyntaxExt};

    use super::*;
    use crate::{errors::RegexError, input::TokenStream};

    #[test]
    fn test_escape() {
        macro_rules! make_test {
            ($ty:ident,$input:literal,$match:literal) => {
                (
                    TokenStream::from($input),
                    Escape {
                        backslash: BackSlash(TokenStream::from("\\")),
                        kind: EscapeKind::$ty($ty(TokenStream::from((1, $match)))),
                    },
                )
            };
        }

        let tests = [
            make_test!(Or, r"\|", r"|"),
            make_test!(BackSlash, r"\\", r"\"),
            make_test!(Caret, r"\^", r"^"),
            make_test!(Dollar, r"\$", r"$"),
            make_test!(Star, r"\*", r"*"),
            make_test!(Plus, r"\+", r"+"),
            make_test!(Question, r"\?", r"?"),
            make_test!(BraceStart, r"\{", r"{"),
            make_test!(BracketStart, r"\[", r"["),
            make_test!(ParenStart, r"\(", r"("),
            make_test!(Dot, r"\.", r"."),
        ];

        for (mut input, token) in tests {
            assert_eq!(input.parse(), Ok(token));
        }

        macro_rules! make_test {
            ($ty:ident,$input:literal,$match:literal) => {
                (
                    TokenStream::from($input),
                    Escape {
                        backslash: BackSlash(TokenStream::from("\\")),
                        kind: EscapeKind::$ty(Char(TokenStream::from((1, $match)))),
                    },
                )
            };
        }

        let tests = [
            make_test!(Boundery, r"\b", r"b"),
            make_test!(NonBoundery, r"\B", r"B"),
            make_test!(Digit, r"\d", r"d"),
            make_test!(NonDigit, r"\D", r"D"),
            make_test!(FF, r"\f", r"f"),
            make_test!(LF, r"\n", r"n"),
            make_test!(CR, r"\r", r"r"),
            make_test!(S, r"\s", r"s"),
            make_test!(NonS, r"\S", r"S"),
            make_test!(TF, r"\t", r"t"),
            make_test!(VF, r"\v", r"v"),
            make_test!(Word, r"\w", r"w"),
            make_test!(NonWord, r"\W", r"W"),
        ];

        for (mut input, token) in tests {
            assert_eq!(input.parse(), Ok(token));
        }

        assert_eq!(
            TokenStream::from(r"\123h").parse(),
            Ok(Escape {
                backslash: BackSlash(TokenStream::from(r"\")),
                kind: EscapeKind::BackReference(FixedDigits(TokenStream::from((1, "12"))))
            },)
        );

        assert_eq!(
            TokenStream::from(r"\xa0b").parse(),
            Ok(Escape {
                backslash: BackSlash(TokenStream::from(r"\")),
                kind: EscapeKind::Hex(
                    Char(TokenStream::from((1, "x"))),
                    FixedHexDigits(TokenStream::from((2, "a0")))
                ),
            },)
        );

        assert_eq!(
            TokenStream::from(r"\u00A0h").parse(),
            Ok(Escape {
                backslash: BackSlash(TokenStream::from(r"\")),
                kind: EscapeKind::Unicode(
                    Char(TokenStream::from((1, "u"))),
                    FixedHexDigits(TokenStream::from((2, "00A0")))
                ),
            })
        );
    }

    #[test]
    fn invalid_escapes() {
        assert_eq!(
            TokenStream::from(r"\u").parse::<Escape<_>>(),
            Err(RegexError::Compile(
                CompileError::EscapeUnicode,
                ControlFlow::Fatal,
                Span::Range(2..2)
            ))
        );

        assert_eq!(
            TokenStream::from(r"\x1ga").parse::<Escape<_>>(),
            Err(RegexError::Compile(
                CompileError::EscapeHex,
                ControlFlow::Fatal,
                Span::Range(2..3)
            ))
        );

        assert_eq!(
            TokenStream::from(r"\a").parse::<Escape<_>>(),
            Err(RegexError::Compile(
                CompileError::Escape,
                ControlFlow::Fatal,
                Span::Range(1..2)
            ))
        );
    }
}

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
#[syntax(map_err = CompileError::Escape.map())]
pub enum Escape<I>
where
    I: PatternInput,
{
    /// `\\`
    BackSlash(BackSlash<I>, BackSlash<I>),
    /// `\^`
    Caret(BackSlash<I>, Caret<I>),
    /// `\*`
    Star(BackSlash<I>, Star<I>),
    /// `\$`
    Dollar(BackSlash<I>, Dollar<I>),
    /// `\?`
    Question(BackSlash<I>, Question<I>),
    /// `\+`
    Plus(BackSlash<I>, Plus<I>),
    /// `\-`
    Minus(BackSlash<I>, Minus<I>),
    /// `\.`
    Dot(BackSlash<I>, Dot<I>),
    /// `\|`
    Or(BackSlash<I>, Or<I>),
    /// `\{`
    BraceStart(BackSlash<I>, BraceStart<I>),
    /// `\[`
    BracketStart(BackSlash<I>, BracketStart<I>),
    /// `\(`
    ParenStart(BackSlash<I>, ParenStart<I>),
    ///  \b
    Boundery(BackSlash<I>, Char<I, 'b'>),
    ///  \B
    NonBoundery(BackSlash<I>, Char<I, 'B'>),
    ///  \d
    Digit(BackSlash<I>, Char<I, 'd'>),
    ///  \D
    NonDigit(BackSlash<I>, Char<I, 'D'>),
    /// \f
    FF(BackSlash<I>, Char<I, 'f'>),
    /// \n
    LF(BackSlash<I>, Char<I, 'n'>),
    /// \r
    CR(BackSlash<I>, Char<I, 'r'>),
    ///  \s
    S(BackSlash<I>, Char<I, 's'>),
    ///  \S
    NonS(BackSlash<I>, Char<I, 'S'>),
    ///  \t
    TF(BackSlash<I>, Char<I, 't'>),
    ///  \v
    VF(BackSlash<I>, Char<I, 'v'>),
    ///  \w
    Word(BackSlash<I>, Char<I, 'w'>),
    ///  \W
    NonWord(BackSlash<I>, Char<I, 'W'>),
    /// backreference `\1..`
    BackReference(BackSlash<I>, FixedDigits<I, 2>),
    /// \xnn
    Hex(BackSlash<I>, Char<I, 'x'>, FixedHexDigits<I, 2>),
    /// \unnnn
    Unicode(BackSlash<I>, Char<I, 'u'>, FixedHexDigits<I, 4>),
}

#[cfg(test)]
mod test {
    use parserc::syntax::InputSyntaxExt;

    use super::*;
    use crate::input::TokenStream;

    #[test]
    fn test_escape() {
        macro_rules! make_test {
            ($ty:ident,$input:literal,$match:literal) => {
                (
                    TokenStream::from($input),
                    Escape::$ty(
                        BackSlash(TokenStream::from("\\")),
                        $ty(TokenStream::from((1, $match))),
                    ),
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
                    Escape::$ty(
                        BackSlash(TokenStream::from("\\")),
                        Char(TokenStream::from((1, $match))),
                    ),
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
            Ok(Escape::BackReference(
                BackSlash(TokenStream::from(r"\")),
                FixedDigits(TokenStream::from((1, "12")))
            ),)
        );

        assert_eq!(
            TokenStream::from(r"\xa0b").parse(),
            Ok(Escape::Hex(
                BackSlash(TokenStream::from(r"\")),
                Char(TokenStream::from((1, "x"))),
                FixedHexDigits(TokenStream::from((2, "a0")))
            ),)
        );

        assert_eq!(
            TokenStream::from(r"\u00A0h").parse(),
            Ok(Escape::Unicode(
                BackSlash(TokenStream::from(r"\")),
                Char(TokenStream::from((1, "u"))),
                FixedHexDigits(TokenStream::from((2, "00A0")))
            ),)
        );
    }
}

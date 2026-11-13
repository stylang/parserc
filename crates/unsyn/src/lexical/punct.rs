//! Punctuation tokens.
//!
//! A punct is matched as a literal, and — unlike a plain character class — it
//! owns the whitespace on both sides of it: the token is
//! `(leading run, punct, trailing run)`, so a grammar may write `A ;B` and `A;
//! B` alike.
//!
//! Because the runs are optional, a punct never fails for a missing separator;
//! it fails only when its own characters are not there.

macro_rules! define_punct {
    ($ident: ident, $value: literal) => {
        #[doc = "define punct `"]
        #[doc = stringify!($value)]
        #[doc = "`"]
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $ident<I>(pub Option<super::S<I>>, pub I, pub Option<super::S<I>>)
        where
            I: crate::input::UnsynInput;

        impl<I> parserc::syntax::Syntax<I> for $ident<I>
        where
            I: crate::input::UnsynInput,
        {
            /// Matches the punct together with the whitespace around it.
            #[inline]
            fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
                use parserc::Parser;
                use parserc::syntax::SyntaxInput;

                Ok(Self(
                    input.parse()?,
                    parserc::keyword($value)
                        .parse(input)
                        .map_err(crate::errors::PunctKind::$ident.map())?,
                    input.parse()?,
                ))
            }

            #[inline]
            fn to_span(&self) -> parserc::Span {
                self.0.to_span() + self.1.to_span()
            }
        }
    };
}

define_punct!(Plus, "+");
define_punct!(Minus, "-");
define_punct!(Star, "*");
define_punct!(Question, "?");
define_punct!(Tilde, "~");
define_punct!(Or, "|");
define_punct!(ParenStart, "(");
define_punct!(ParenEnd, ")");
define_punct!(BracketStart, "[");
define_punct!(BracketEnd, "]");
define_punct!(BraceStart, "{");
define_punct!(BraceEnd, "}");
define_punct!(Lt, "<");
define_punct!(Gt, ">");
define_punct!(Comma, ",");
define_punct!(ArrowRight, "->");
define_punct!(Semi, ";");

#[cfg(test)]
mod tests {
    use parserc::{AsStr, Span, syntax::Syntax};

    use crate::{
        errors::{PunctKind, UnsynError},
        input::TokenStream,
    };

    use super::*;

    #[test]
    fn a_punct_owns_the_whitespace_on_both_sides() {
        let mut input = TokenStream::from(" ; next");

        let semi = Semi::parse(&mut input).expect("the punct should parse");

        assert_eq!(semi.0.expect("a leading run").0.as_str(), " ");
        assert_eq!(semi.1.as_str(), ";");
        assert_eq!(semi.2.expect("a trailing run").0.as_str(), " ");
        assert_eq!(input.as_str(), "next");
    }

    #[test]
    fn a_punct_needs_no_whitespace_at_all() {
        let mut input = TokenStream::from(";");

        let semi = Semi::parse(&mut input).expect("the punct should parse");

        assert!(semi.0.is_none());
        assert!(semi.2.is_none());
        assert_eq!(input.as_str(), "");
    }

    #[test]
    fn a_multi_character_punct_is_matched_whole() {
        let mut input = TokenStream::from("->");

        assert_eq!(ArrowRight::parse(&mut input).unwrap().1.as_str(), "->");

        // The second character of `->` is not optional: a lone `-` is a `Minus`,
        // and a `->` that is not there fails with what it expected.
        let mut input = TokenStream::from("-");

        assert!(Minus::parse(&mut input).is_ok());

        let mut input = TokenStream::from("-");

        assert_eq!(
            ArrowRight::parse(&mut input),
            Err(UnsynError::Punct(
                PunctKind::ArrowRight,
                parserc::ControlFlow::Recovable,
                Span::Range(0..1)
            ))
        );
    }

    #[test]
    fn the_span_of_a_punct_ends_with_its_own_characters() {
        let mut input = TokenStream::from(" ; ");

        let semi = Semi::parse(&mut input).unwrap();

        // The separator that follows the punct is not part of it, so that a
        // diagnostic never points past the token it talks about.
        assert_eq!(semi.to_span(), Span::Range(0..2));
    }
}

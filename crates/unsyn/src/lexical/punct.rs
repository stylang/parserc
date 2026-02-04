//! punct tokens for `unsyn` language

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
define_punct!(PathSep, "::");
define_punct!(Comma, ",");
define_punct!(ArrowRight, "->");
define_punct!(Semi, ";");
define_punct!(DotDot, "..");

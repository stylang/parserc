//! keywords for `unsyn` language.

macro_rules! define_keyword {
    ($ident: ident, $value: literal) => {
        #[doc = "define keyword `"]
        #[doc = stringify!($value)]
        #[doc = "`"]
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $ident<I>(pub I, pub Option<super::S<I>>)
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

                let keyword = parserc::keyword($value)
                    .parse(input)
                    .map_err(crate::errors::KeywordKind::$ident.map())?;

                if let Some(c) = input.iter().next() {
                    if c.is_alphanumeric() || c == '_' {
                        return Err(crate::errors::UnsynError::Keyword(
                            crate::errors::KeywordKind::$ident,
                            parserc::ControlFlow::Recovable,
                            keyword.to_span(),
                        ));
                    }
                }

                Ok(Self(keyword, input.parse()?))
            }

            #[inline]
            fn to_span(&self) -> parserc::Span {
                self.0.to_span() + self.1.to_span()
            }
        }
    };
}

define_keyword!(Whitespace, "whitespace");
define_keyword!(Lexer, "lexer");
define_keyword!(Syntax, "syntax");
define_keyword!(Followed, "followed");
define_keyword!(Concat, "concat");
define_keyword!(Except, "except");
define_keyword!(Use, "use");
define_keyword!(Super, "super");
define_keyword!(Crate, "crate");
define_keyword!(As, "as");
define_keyword!(This, "this");
define_keyword!(Mod, "mod");

//! Keywords of the `unsyn` language.
//!
//! A keyword is matched as a literal prefix, which is not enough on its own:
//! `lexerX` is one identifier, not the keyword `lexer` followed by `X`. So a
//! keyword also checks the character that follows it and rejects the match when
//! that character could continue an identifier — which keeps [`Ident`](super::ident::Ident)
//! and the keywords from ever splitting the same word between them.
//!
//! Unlike a punct, a keyword owns only the whitespace *after* it: a definition
//! always starts at the keyword, so there is nothing before it that belongs to
//! the keyword's own span.

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

#[cfg(test)]
mod tests {
    use parserc::{AsStr, ControlFlow, Span, syntax::SyntaxInput};

    use crate::{
        errors::{KeywordKind, UnsynError},
        input::TokenStream,
    };

    use super::*;

    #[test]
    fn a_keyword_owns_the_whitespace_after_it() {
        let mut input = TokenStream::from("lexer ABC");

        let keyword = input.parse::<Lexer<_>>().expect("the keyword should parse");

        assert_eq!(keyword.0.as_str(), "lexer");
        assert_eq!(keyword.1.expect("a trailing run").0.as_str(), " ");
        assert_eq!(input.as_str(), "ABC");
    }

    #[test]
    fn a_keyword_may_end_the_input() {
        let mut input = TokenStream::from("whitespace");

        let keyword = input
            .parse::<Whitespace<_>>()
            .expect("the keyword should parse");

        assert_eq!(keyword.0.as_str(), "whitespace");
        assert!(keyword.1.is_none());
    }

    #[test]
    fn a_keyword_can_not_be_the_head_of_a_longer_word() {
        // `lexerX` and `lexer_` are identifiers; reading `lexer` out of them
        // would leave `X` behind and change the meaning of the file.
        for source in ["lexerX", "lexer_", "lexer9"] {
            let mut input = TokenStream::from(source);

            assert_eq!(
                input.parse::<Lexer<_>>(),
                Err(UnsynError::Keyword(
                    KeywordKind::Lexer,
                    ControlFlow::Recovable,
                    Span::Range(0..5),
                )),
                "{source}"
            );
        }
    }

    #[test]
    fn a_keyword_is_case_sensitive() {
        for source in ["Lexer", "LEXER"] {
            let mut input = TokenStream::from(source);

            let error = input.parse::<Lexer<_>>().expect_err("a name");

            assert!(
                matches!(error, UnsynError::Keyword(KeywordKind::Lexer, _, _)),
                "{source}: {error}"
            );
        }
    }
}

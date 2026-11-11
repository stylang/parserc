//! ident token of `unsyn` language.

use parserc::{Parser, next_if, syntax::Syntax, take_while};
use unicode_ident::{is_xid_continue, is_xid_start};

use crate::{
    errors::{SemanticsKind, SyntaxKind, UnsynError},
    input::UnsynInput,
};

/// A identifier except a keyword.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ident<I>(pub I)
where
    I: UnsynInput;

impl<I> Syntax<I> for Ident<I>
where
    I: UnsynInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        let mut content = input.clone();

        _ = next_if(|c| c == '_' || is_xid_start(c))
            .parse(input)
            .map_err(SyntaxKind::Ident.map())?;

        let rest = take_while(|c| is_xid_continue(c)).parse(input)?;

        let content = content.split_to(1 + rest.len());

        match content.as_str() {
            "whitespace" | "lexer" | "syntax" | "followed" | "except" | "concat" => {
                return Err(UnsynError::Semantics(
                    SemanticsKind::Keyword,
                    content.to_span(),
                ));
            }
            _ => {}
        }

        Ok(Self(content))
    }

    #[inline]
    fn to_span(&self) -> parserc::Span {
        self.0.to_span()
    }
}

#[cfg(test)]
mod tests {
    use parserc::syntax::SyntaxInput;

    use crate::input::TokenStream;

    use super::*;

    #[test]
    fn test_reserved_keywords() {
        // A keyword of the language can not be used as a node name.
        for keyword in [
            "whitespace",
            "lexer",
            "syntax",
            "followed",
            "concat",
            "except",
        ] {
            assert_eq!(
                TokenStream::from(keyword).parse::<Ident<_>>(),
                Err(UnsynError::Semantics(
                    SemanticsKind::Keyword,
                    parserc::Span::Range(0..keyword.len())
                )),
                "{keyword}"
            );
        }
    }

    #[test]
    fn test_ident() {
        assert_eq!(
            TokenStream::from("WHITESPACE_").parse::<Ident<_>>(),
            Ok(Ident(TokenStream::from((0, "WHITESPACE_"))))
        );

        // A name may start with `_` but is not a keyword.
        assert!(TokenStream::from("_x").parse::<Ident<_>>().is_ok());

        assert!(TokenStream::from("1abc").parse::<Ident<_>>().is_err());
    }
}

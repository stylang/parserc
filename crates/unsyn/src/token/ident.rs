//! ident token of `unsyn` language.

use parserc::{
    Parser,
    combinators::{next_if, take_while},
    sourceinput::{Span, ToSpan},
    syntax::Syntax,
};
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
    fn parse(input: &mut I) -> Result<Self, <I as parserc::sourceinput::Input>::Error> {
        let mut content = input.clone();

        _ = next_if(|c| c == '_' || is_xid_start(c))
            .parse(input)
            .map_err(SyntaxKind::Ident.map())?;

        let rest = take_while(|c| is_xid_continue(c)).parse(input)?;

        let content = content.split_to(1 + rest.len());

        match content.as_str() {
            "whitespace" | "lexer" | "syntax" | "followed" | "concat" | "except" | "use"
            | "super" | "crate" | "as" | "self" | "mod" => {
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
    fn to_span(&self) -> Span {
        self.0.to_span()
    }
}

#[cfg(test)]
mod tests {
    use parserc::{sourceinput::Span, syntax::SyntaxExt};

    use crate::{
        errors::{SemanticsKind, UnsynError},
        input::Chars,
        token::ident::Ident,
    };

    #[test]
    fn test_ident() {
        let keywords = [
            "whitespace",
            "lexer",
            "syntax",
            "followed",
            "concat",
            "except",
            "use",
            "super",
            "crate",
            "as",
            "self",
            "mod",
        ];

        for kw in keywords {
            assert_eq!(
                Chars::new(kw).parse::<Ident<_>>(),
                Err(UnsynError::Semantics(
                    SemanticsKind::Keyword,
                    Span::from(0..kw.len()),
                ))
            );
        }
    }
}

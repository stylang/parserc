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
            "lexer" | "syntax" | "followed" | "except" | "use" | "super" | "crate" | "concat" => {
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

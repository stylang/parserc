//! lexical `comments`,

use parserc::{ControlFlow, Parser, keyword, next, syntax::Syntax, take_till, take_while};

use crate::{
    errors::{SyntaxKind, UnsynError},
    input::UnsynInput,
};

/// A line comment, see [`The Rust Reference`]
///
/// [`The Rust Reference`]: https://doc.rust-lang.org/reference/comments.html#railroad-LINE_COMMENT
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineComment<I>(pub I)
where
    I: UnsynInput;

impl<I> Syntax<I> for LineComment<I>
where
    I: UnsynInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        let mut content = input.clone();

        let header = take_while(|c| c == '/').parse(input)?;

        match header.len() {
            0 | 1 | 3 => {
                return Err(UnsynError::Syntax(
                    SyntaxKind::LineComment,
                    ControlFlow::Recovable,
                    header.to_span(),
                ));
            }
            2 => match input.iter().next() {
                Some('!') | Some('\n') => {
                    return Err(UnsynError::Syntax(
                        SyntaxKind::LineComment,
                        ControlFlow::Recovable,
                        content.to_span_at(3),
                    ));
                }
                _ => {}
            },
            _ => {}
        }

        let body = take_till(|c| c == '\n').parse(input)?;

        Ok(Self(content.split_to(body.len() + header.len())))
    }

    #[inline]
    fn to_span(&self) -> parserc::Span {
        self.0.to_span()
    }
}

/// An inner line document, see [`The Rust Reference`]
///
/// [`The Rust Reference`]: https://doc.rust-lang.org/reference/comments.html#railroad-INNER_LINE_DOC
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InnerLineDoc<I>(pub I)
where
    I: UnsynInput;

impl<I> Syntax<I> for InnerLineDoc<I>
where
    I: UnsynInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        let mut content = input.clone();

        keyword("//!")
            .parse(input)
            .map_err(SyntaxKind::InnerLineDoc.map())?;

        let rest = take_while(|c| c != '\r' && c != '\n').parse(input)?;

        Ok(Self(content.split_to(3 + rest.len())))
    }

    #[inline]
    fn to_span(&self) -> parserc::Span {
        self.0.to_span()
    }
}

/// An outer line document, see [`The Rust Reference`]
///
/// [`The Rust Reference`]: https://doc.rust-lang.org/reference/comments.html#railroad-OUTER_LINE_DOC
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OuterLineDoc<I>(pub I)
where
    I: UnsynInput;

impl<I> Syntax<I> for OuterLineDoc<I>
where
    I: UnsynInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        let mut content = input.clone();

        keyword("///").parse(input)?;

        if let Some('/') = input.iter().next() {
            return Err(UnsynError::Syntax(
                SyntaxKind::OuterLineDoc,
                ControlFlow::Recovable,
                content.to_span_at(4),
            ));
        }

        let rest = take_while(|c| c != '\r' && c != '\n').parse(input)?;

        Ok(Self(content.split_to(3 + rest.len())))
    }

    #[inline]
    fn to_span(&self) -> parserc::Span {
        self.0.to_span()
    }
}

/// a block comment, see [`The Rust Reference`]
///
/// [`The Rust Reference`]: https://doc.rust-lang.org/reference/comments.html#railroad-BLOCK_COMMENT
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockComment<I>(pub I)
where
    I: UnsynInput;

impl<I> Syntax<I> for BlockComment<I>
where
    I: UnsynInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        let mut content = input.clone();

        keyword("/*")
            .parse(input)
            .map_err(SyntaxKind::BlockComment.map())?;

        let mut iter = input.iter();

        match iter.next() {
            Some('!') => {
                return Err(UnsynError::Syntax(
                    SyntaxKind::BlockComment,
                    ControlFlow::Recovable,
                    content.to_span_at(3),
                ));
            }
            Some('*') => match iter.next() {
                Some('*') => {}
                Some('/') => return Ok(Self(content.split_to(4))),
                _ => {
                    return Err(UnsynError::Syntax(
                        SyntaxKind::BlockComment,
                        ControlFlow::Recovable,
                        content.to_span_at(3),
                    ));
                }
            },
            _ => {}
        }

        let mut offset = 2usize;

        loop {
            if let Some(nest) = BlockCommentOrDoc::into_parser().ok().parse(input)? {
                offset += nest.len();
                continue;
            }

            let rest = take_till(|c| c == '*' || c == '/').parse(input)?;

            offset += rest.len();

            if input.is_empty() {
                return Err(UnsynError::Syntax(
                    SyntaxKind::BlockComment,
                    ControlFlow::Recovable,
                    content.to_span_at(offset),
                ));
            }

            if let Some(_) = keyword("*/").ok().parse(input)? {
                offset += 2;
                break;
            } else if let Some(_) = next('*').ok().parse(input)? {
                offset += 1;
            }
        }

        Ok(Self(content.split_to(offset)))
    }

    #[inline]
    fn to_span(&self) -> parserc::Span {
        self.0.to_span()
    }
}

/// An inner block document, see [`The Rust Reference`]
///
/// [`The Rust Reference`]: https://doc.rust-lang.org/reference/comments.html#railroad-INNER_BLOCK_DOC
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InnerBlockDoc<I>(pub I)
where
    I: UnsynInput;

impl<I> Syntax<I> for InnerBlockDoc<I>
where
    I: UnsynInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        let mut content = input.clone();

        keyword("/*!")
            .parse(input)
            .map_err(SyntaxKind::InnerBlockDoc.map())?;

        let mut offset = 3usize;

        loop {
            if let Some(nest) = BlockCommentOrDoc::into_parser().ok().parse(input)? {
                offset += nest.len();
                continue;
            }

            let rest = take_till(|c| c == '*' || c == '/').parse(input)?;

            offset += rest.len();

            if input.is_empty() {
                return Err(UnsynError::Syntax(
                    SyntaxKind::InnerBlockDoc,
                    ControlFlow::Recovable,
                    content.to_span_at(offset),
                ));
            }

            if let Some(_) = keyword("*/").ok().parse(input)? {
                offset += 2;
                break;
            } else if let Some(_) = next('*').ok().parse(input)? {
                offset += 1;
            }
        }

        Ok(Self(content.split_to(offset)))
    }

    #[inline]
    fn to_span(&self) -> parserc::Span {
        self.0.to_span()
    }
}

/// An outer block document, see [`The Rust Reference`]
///
/// [`The Rust Reference`]: https://doc.rust-lang.org/reference/comments.html#railroad-OUTER_BLOCK_DOC
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OuterBlockDoc<I>(pub I)
where
    I: UnsynInput;

impl<I> Syntax<I> for OuterBlockDoc<I>
where
    I: UnsynInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        let mut content = input.clone();

        keyword("/**")
            .parse(input)
            .map_err(SyntaxKind::OuterBlockDoc.map())?;

        if let Some('*') = input.iter().next() {
            return Err(UnsynError::Syntax(
                SyntaxKind::OuterBlockDoc,
                ControlFlow::Recovable,
                content.to_span_at(3),
            ));
        }

        let mut offset = 3usize;

        loop {
            if let Some(nest) = BlockCommentOrDoc::into_parser().ok().parse(input)? {
                offset += nest.len();
                continue;
            }

            let rest = take_till(|c| c == '*' || c == '/').parse(input)?;

            offset += rest.len();

            if input.is_empty() {
                return Err(UnsynError::Syntax(
                    SyntaxKind::OuterBlockDoc,
                    ControlFlow::Recovable,
                    content.to_span_at(offset),
                ));
            }

            if let Some(_) = keyword("*/").ok().parse(input)? {
                offset += 2;
                break;
            } else if let Some(_) = next('*').ok().parse(input)? {
                offset += 1;
            }
        }

        Ok(Self(content.split_to(offset)))
    }

    #[inline]
    fn to_span(&self) -> parserc::Span {
        self.0.to_span()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum BlockCommentOrDoc<I>
where
    I: UnsynInput,
{
    Comment(BlockComment<I>),
    Outer(OuterBlockDoc<I>),
    Inner(InnerBlockDoc<I>),
}

impl<I> BlockCommentOrDoc<I>
where
    I: UnsynInput,
{
    #[inline]
    fn len(&self) -> usize {
        match self {
            BlockCommentOrDoc::Comment(c) => c.0.len(),
            BlockCommentOrDoc::Outer(c) => c.0.len(),
            BlockCommentOrDoc::Inner(c) => c.0.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use parserc::{ControlFlow, Span, syntax::SyntaxInput};

    use crate::{
        errors::{SyntaxKind, UnsynError},
        input::TokenStream,
        lexical::comments::{
            BlockComment, InnerBlockDoc, InnerLineDoc, LineComment, OuterBlockDoc,
        },
    };

    #[test]
    fn test_line_comment() {
        let line_comments = [
            ("//", "//"),
            ("// -hello", "// -hello"),
            ("// hello\n", "// hello"),
            ("////", "////"),
            ("//// -hello", "//// -hello"),
            ("//// -hello\n", "//// -hello"),
            ("///////", "///////"),
        ];

        for (input, expect) in line_comments {
            assert_eq!(
                TokenStream::from(input).parse::<LineComment<_>>(),
                Ok(LineComment(TokenStream::from(expect)))
            );
        }

        let non_line_comments = [("//!\n", "//!"), ("/// hello", "///"), ("/ hello", "/")];

        for (input, unexpect) in non_line_comments {
            assert_eq!(
                TokenStream::from(input).parse::<LineComment<_>>(),
                Err(UnsynError::Syntax(
                    SyntaxKind::LineComment,
                    ControlFlow::Recovable,
                    Span::Range(0..unexpect.len())
                )),
            );
        }

        assert_eq!(
            TokenStream::from("").parse::<LineComment<_>>(),
            Err(UnsynError::Syntax(
                SyntaxKind::LineComment,
                ControlFlow::Recovable,
                Span::Range(0..0)
            )),
        )
    }

    #[test]
    fn test_inner_line_doc() {
        let inner_line_docs = [
            ("//! hello", "//! hello"),
            ("//!\r", "//!"),
            ("//!! hello\n", "//!! hello"),
        ];

        for (input, expect) in inner_line_docs {
            assert_eq!(
                TokenStream::from(input).parse::<InnerLineDoc<_>>(),
                Ok(InnerLineDoc(TokenStream::from(expect)))
            );
        }

        let none_inner_line_docs = [("/// hello", "///"), ("/! hello", "//!")];

        for (input, unexpect) in none_inner_line_docs {
            assert_eq!(
                TokenStream::from(input).parse::<InnerLineDoc<_>>(),
                Err(UnsynError::Syntax(
                    SyntaxKind::InnerLineDoc,
                    ControlFlow::Recovable,
                    Span::Range(0..unexpect.len())
                ))
            );
        }
    }

    #[test]
    fn test_inner_block_doc() {
        let inner_line_docs = [
            ("/*! hello\n/**/*/", "/*! hello\n/**/*/"),
            ("/*! hello\n/**hello*/*/", "/*! hello\n/**hello*/*/"),
        ];

        for (input, expect) in inner_line_docs {
            assert_eq!(
                TokenStream::from(input).parse::<InnerBlockDoc<_>>(),
                Ok(InnerBlockDoc(TokenStream::from(expect)))
            );
        }
    }

    #[test]
    fn test_block_comment() {
        let block_comments = [
            ("/*   - Only a comment */", "/*   - Only a comment */"),
            ("/*** - Only a comment */", "/*** - Only a comment */"),
            ("/***/", "/***/"),
            ("/**/", "/**/"),
        ];

        for (input, expect) in block_comments {
            assert_eq!(
                TokenStream::from(input).parse::<BlockComment<_>>(),
                Ok(BlockComment(TokenStream::from(expect)))
            );
        }

        let non_block_comments = [
            ("/*! innner block doc */", "/*!"),
            ("/** Outer block doc */", "/**"),
        ];

        for (input, unexpect) in non_block_comments {
            assert_eq!(
                TokenStream::from(input).parse::<BlockComment<_>>(),
                Err(UnsynError::Syntax(
                    SyntaxKind::BlockComment,
                    ControlFlow::Recovable,
                    Span::Range(0..unexpect.len())
                ))
            );
        }
    }

    #[test]
    fn test_outer_block_doc() {
        let inner_line_docs = [
            ("/** hello\n/**/*/", "/** hello\n/**/*/"),
            ("/** hello\n/**hello*/*/", "/** hello\n/**hello*/*/"),
        ];

        for (input, expect) in inner_line_docs {
            assert_eq!(
                TokenStream::from(input).parse::<OuterBlockDoc<_>>(),
                Ok(OuterBlockDoc(TokenStream::from(expect)))
            );
        }

        let non_outer_block_docs = [("/***/", "/**")];

        for (input, unexpect) in non_outer_block_docs {
            assert_eq!(
                TokenStream::from(input).parse::<OuterBlockDoc<_>>(),
                Err(UnsynError::Syntax(
                    SyntaxKind::OuterBlockDoc,
                    ControlFlow::Recovable,
                    Span::Range(0..unexpect.len())
                ))
            );
        }
    }
}

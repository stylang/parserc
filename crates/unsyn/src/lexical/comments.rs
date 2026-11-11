//! The comment and documentation tokens.
//!
//! Six forms are recognised, told apart by how they open:
//!
//! | opens with     | token                        |
//! |----------------|------------------------------|
//! | `//`           | [`LineComment`]              |
//! | `//!`          | [`InnerLineDoc`]             |
//! | `///`          | [`OuterLineDoc`]             |
//! | `/*`           | [`BlockComment`]             |
//! | `/*!`          | [`InnerBlockDoc`]            |
//! | `/**`          | [`OuterBlockDoc`]            |
//!
//! The four doc forms are matched by the *third* character, so a block comment
//! never swallows a document and a document never swallows a comment: `/*!` is
//! an inner document, `/**/` is an empty block comment, and `////` is a comment
//! rather than an empty document, as in Rust.
//!
//! A block form nests, and an opening is a commitment: every `/*` within a group
//! needs a `*/` of its own, so `/* a /* nested */ still the comment */` is one
//! token. An opening whose closing never arrives leaves the group unterminated
//! even when a `*/` comes later, so `/*/*/ */` is a broken comment rather than
//! one that ends at that `*/`, which is how Rust reads them.
//!
//! The three `lexer` rules of `bootstrap.uns` say the same thing by keeping `/*`
//! out of the text of a body, so that an opening there can only be read by the
//! recursive alternative. One difference is left open: those rules also keep `CR`
//! out of a document body, which the scanner here does not check.

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

        keyword("///")
            .parse(input)
            .map_err(SyntaxKind::OuterLineDoc.map())?;

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

/// Scans the body of a block group whose opening was already taken.
///
/// Block groups nest the way Rust's do: every `/*` inside needs a `*/` of its
/// own, and a `*/` closes one group rather than the token that contains it. So
/// an opening whose closing never arrives leaves the whole group unterminated,
/// even when a later `*/` follows it: `/*/*/ */` is a broken comment rather than
/// a comment that ends at that later `*/`. A `/` that is not part of an opening
/// is only text, so `/*/ */` does close.
///
/// `opened` is the length of the prefix already taken, which is also where the
/// count starts, and the result is the length of the whole group, its closing
/// `*/` included. `unterminated` builds the report for a group that ran out of
/// input, over the text it had to swallow to find nothing.
#[inline]
fn scan_block<I, F>(input: &mut I, opened: usize, unterminated: F) -> Result<usize, UnsynError>
where
    I: UnsynInput,
    F: FnOnce(usize) -> UnsynError,
{
    let mut offset = opened;
    let mut depth = 1usize;

    loop {
        // Anything that is neither pair is text, and skipping it in one piece is
        // what leaves the interesting decisions to the three branches below.
        offset += take_till(|c| c == '*' || c == '/').parse(input)?.len();

        let Some(current) = input.iter().next() else {
            return Err(unterminated(offset));
        };

        if keyword("*/").ok().parse(input)?.is_some() {
            offset += 2;
            depth -= 1;

            if depth == 0 {
                return Ok(offset);
            }
        } else if keyword("/*").ok().parse(input)?.is_some() {
            // A group opened here has to close before this one does: nothing
            // else in this loop takes a `/*`, so an opening is a commitment
            // rather than one alternative among several.
            offset += 2;
            depth += 1;
        } else {
            // A lone `*` or `/` is text. It is also the only progress this
            // position can make, which is why taking it matters.
            next(current).parse(input)?;
            offset += 1;
        }
    }
}

/// A block comment, see [`The Rust Reference`]
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

        let offset = scan_block(input, 2usize, |offset| {
            UnsynError::Syntax(
                SyntaxKind::BlockComment,
                ControlFlow::Recovable,
                content.to_span_at(offset),
            )
        })?;

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

        let offset = scan_block(input, 3usize, |offset| {
            UnsynError::Syntax(
                SyntaxKind::InnerBlockDoc,
                ControlFlow::Recovable,
                content.to_span_at(offset),
            )
        })?;

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

        let offset = scan_block(input, 3usize, |offset| {
            UnsynError::Syntax(
                SyntaxKind::OuterBlockDoc,
                ControlFlow::Recovable,
                content.to_span_at(offset),
            )
        })?;

        Ok(Self(content.split_to(offset)))
    }

    #[inline]
    fn to_span(&self) -> parserc::Span {
        self.0.to_span()
    }
}

#[cfg(test)]
mod tests {
    use parserc::{ControlFlow, Span, syntax::SyntaxInput};

    use crate::{
        errors::{SyntaxKind, UnsynError},
        input::TokenStream,
        lexical::comments::{
            BlockComment, InnerBlockDoc, InnerLineDoc, LineComment, OuterBlockDoc, OuterLineDoc,
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

    #[test]
    fn test_outer_line_doc() {
        let outer_line_docs = [
            ("/// hello", "/// hello"),
            ("///", "///"),
            ("/// hello\n", "/// hello"),
            ("///\r\n", "///"),
            ("///\n", "///"),
        ];

        for (input, expect) in outer_line_docs {
            assert_eq!(
                TokenStream::from(input).parse::<OuterLineDoc<_>>(),
                Ok(OuterLineDoc(TokenStream::from(expect))),
                "{input}"
            );
        }

        // A fourth slash belongs to this document, so the token after `///` may
        // not be one: `//// x` is a line comment, not a document about `/ x`. It
        // is rejected past the `///` it did read.
        assert_eq!(
            TokenStream::from("//// hello").parse::<OuterLineDoc<_>>(),
            Err(UnsynError::Syntax(
                SyntaxKind::OuterLineDoc,
                ControlFlow::Recovable,
                Span::Range(0..4)
            ))
        );

        // A document that does not open with `///` is not one at all, and the
        // report names the token that was expected.
        for input in ["//! hello", "// hello"] {
            assert_eq!(
                TokenStream::from(input).parse::<OuterLineDoc<_>>(),
                Err(UnsynError::Syntax(
                    SyntaxKind::OuterLineDoc,
                    ControlFlow::Recovable,
                    Span::Range(0..3)
                )),
                "{input}"
            );
        }
    }

    #[test]
    fn a_block_comment_keeps_its_nesting() {
        let nested = [
            ("/* a /* b */ c */", "/* a /* b */ c */"),
            ("/* /* /* */ */ */", "/* /* /* */ */ */"),
            // A document nested in a comment does not end it.
            ("/* /** doc */ */", "/* /** doc */ */"),
            ("/* /*! doc */ */", "/* /*! doc */ */"),
            // A `*/` that does not open a group is only text inside it.
            ("/* a * b */", "/* a * b */"),
            ("/* a / b */", "/* a / b */"),
            // The inner `/* */` is absorbed, so it is the outer `*/` that ends
            // the token.
            ("/* /* */ */", "/* /* */ */"),
            // The same with no space to spare: the first `*/` closes the group
            // the second `/*` opened, and the second `*/` closes this one.
            ("/*/*/ */*/", "/*/*/ */*/"),
        ];

        // `/**/` is already a complete (empty) block, so the scanner stops there
        // and leaves the rest of the input alone.
        assert_eq!(
            TokenStream::from("/**/ */").parse::<BlockComment<_>>(),
            Ok(BlockComment(TokenStream::from("/**/")))
        );

        for (input, expect) in nested {
            assert_eq!(
                TokenStream::from(input).parse::<BlockComment<_>>(),
                Ok(BlockComment(TokenStream::from(expect))),
                "{input}"
            );
        }
    }

    #[test]
    fn a_lone_slash_is_only_text_inside_a_block() {
        let block_comments = [("/* * / * */", "/* * / * */")];

        for (input, expect) in block_comments {
            assert_eq!(
                TokenStream::from(input).parse::<BlockComment<_>>(),
                Ok(BlockComment(TokenStream::from(expect))),
                "{input}"
            );
        }

        // These three never reach a `*/`, so they are reported rather than
        // scanned forever.
        for input in ["/*/", "/* /", "/* a / b"] {
            assert!(
                TokenStream::from(input).parse::<BlockComment<_>>().is_err(),
                "{input}"
            );

            assert!(
                TokenStream::from(input)
                    .parse::<OuterBlockDoc<_>>()
                    .is_err(),
                "{input}"
            );
        }
    }

    /// An opening inside a group has to close before the group that holds it.
    #[test]
    fn an_unclosed_opening_leaves_the_group_that_contains_it_open() {
        // Every `/*` below opens a group whose `*/` never arrives, so the `*/`
        // that does arrive closes that group and leaves the one around it open.
        let unterminated = [
            ("/*/*/", SyntaxKind::BlockComment, 5),
            ("/*/*/ */", SyntaxKind::BlockComment, 8),
            ("/* /* */", SyntaxKind::BlockComment, 8),
            ("/* /*! */", SyntaxKind::BlockComment, 9),
            ("/*! /*/", SyntaxKind::InnerBlockDoc, 7),
            ("/** /*/", SyntaxKind::OuterBlockDoc, 7),
        ];

        for (input, kind, len) in unterminated {
            let parsed = match kind {
                SyntaxKind::BlockComment => {
                    TokenStream::from(input).parse::<BlockComment<_>>().err()
                }
                SyntaxKind::InnerBlockDoc => {
                    TokenStream::from(input).parse::<InnerBlockDoc<_>>().err()
                }
                SyntaxKind::OuterBlockDoc => {
                    TokenStream::from(input).parse::<OuterBlockDoc<_>>().err()
                }
                _ => unreachable!("{input}"),
            };

            assert_eq!(
                parsed,
                Some(UnsynError::Syntax(
                    kind,
                    ControlFlow::Recovable,
                    Span::Range(0..len)
                )),
                "{input}"
            );
        }

        // A group that does close belongs to the one that contains it, which is
        // the reading the reports above rest on.
        assert_eq!(
            TokenStream::from("/* /*! */ */").parse::<BlockComment<_>>(),
            Ok(BlockComment(TokenStream::from("/* /*! */ */")))
        );
    }

    #[test]
    fn an_unterminated_block_is_reported_where_it_stopped() {
        // Each form runs out of input while it looks for the `*/` that closes it,
        // and the report covers everything it had to swallow to find nothing.
        assert_eq!(
            TokenStream::from("/* never closed").parse::<BlockComment<_>>(),
            Err(UnsynError::Syntax(
                SyntaxKind::BlockComment,
                ControlFlow::Recovable,
                Span::Range(0..15)
            ))
        );

        assert_eq!(
            TokenStream::from("/*! never closed").parse::<InnerBlockDoc<_>>(),
            Err(UnsynError::Syntax(
                SyntaxKind::InnerBlockDoc,
                ControlFlow::Recovable,
                Span::Range(0..16)
            ))
        );

        assert_eq!(
            TokenStream::from("/** never closed").parse::<OuterBlockDoc<_>>(),
            Err(UnsynError::Syntax(
                SyntaxKind::OuterBlockDoc,
                ControlFlow::Recovable,
                Span::Range(0..16)
            ))
        );
    }
}

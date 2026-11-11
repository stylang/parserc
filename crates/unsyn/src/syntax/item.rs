//! The items a source file is made of.
//!
//! A file is a [`Crate`]: the inner documents that describe it, then the [`Item`]s
//! that follow. An item is an outer document, a run of whitespace, or a statement
//! that defines a node, so nothing in a file is left unaccounted for.
//!
//! Whitespace is an item rather than something skipped between items, which is
//! what lets every span of the tree be read back from the tree itself. An outer
//! document is the exception: it carries the whitespace that precedes it, so that
//! it stays attached to the statement it documents.
//!
//! A file is read to its end, and text no item can read makes the file broken
//! rather than short (see [`Crate`]).

use parserc::{
    ControlFlow, Span,
    syntax::{Syntax, SyntaxInput},
};

use crate::{
    errors::{SyntaxKind, UnsynError},
    input::UnsynInput,
    lexical::{
        S,
        comments::{InnerBlockDoc, InnerLineDoc, OuterBlockDoc, OuterLineDoc},
    },
    syntax::Stmt,
};

/// An inner document of a source file, the `//!` and `/*!` forms.
///
/// These describe the file, so they come before anything it defines.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InnerDoc<I>
where
    I: UnsynInput,
{
    /// A `/*!` form, with the whitespace that precedes it.
    Block(Option<S<I>>, InnerBlockDoc<I>),
    /// A `//!` form, with the whitespace that precedes it.
    Line(Option<S<I>>, InnerLineDoc<I>),
}

/// An outer document of a statement, the `///` and `/**` forms.
///
/// These belong to the statement that follows them, which is why they are items
/// of the file rather than part of the statement.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OuterDoc<I>
where
    I: UnsynInput,
{
    /// A `/**` form, with the whitespace that precedes it.
    Block(Option<S<I>>, OuterBlockDoc<I>),
    /// A `///` form, with the whitespace that precedes it.
    Line(Option<S<I>>, OuterLineDoc<I>),
}

/// One item of a source file.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Item<I>
where
    I: UnsynInput,
{
    /// A document that belongs to the statement after it.
    OuterDoc(OuterDoc<I>),
    /// Whitespace between two items.
    S(S<I>),
    /// A statement that defines a node.
    Stmt(Stmt<I>),
}

/// A parsed source file.
///
/// A file is read to its end: the two lists stop where an item fails to match,
/// and whatever is left over then belongs to no rule of the grammar, so it is
/// reported as [`SyntaxKind::Eof`] rather than handed back to the caller.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Crate<I>
where
    I: UnsynInput,
{
    /// The inner documents of the file, in the order they were written.
    pub inner_docs: Vec<InnerDoc<I>>,
    /// The items that follow them, in the order they were written.
    pub items: Vec<Item<I>>,
}

impl<I> Syntax<I> for Crate<I>
where
    I: UnsynInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        let inner_docs = input.parse()?;
        let items = input.parse()?;

        // A list of items ends where the next one does not match, which is only
        // the end of the file when nothing is left to read.
        if !input.is_empty() {
            return Err(UnsynError::Syntax(
                SyntaxKind::Eof,
                ControlFlow::Fatal,
                input.to_span(),
            ));
        }

        Ok(Self { inner_docs, items })
    }

    #[inline]
    fn to_span(&self) -> Span {
        Span::None
            .union(&self.inner_docs.to_span())
            .union(&self.items.to_span())
    }
}

#[cfg(test)]
mod tests {
    use parserc::{Span, syntax::SyntaxInput};

    use crate::{
        input::TokenStream,
        lexical::{
            S,
            comments::{InnerLineDoc, OuterLineDoc},
            ident::Ident,
            keyword::Whitespace,
            lit::{LitStr, StrSegment},
            punct::{ArrowRight, Semi},
        },
        syntax::{Expr, ExprNoTopAlt, ExprNoTopAlts, ExprWithoutSuffix},
    };

    use super::*;

    #[test]
    fn an_empty_file_is_a_crate_with_nothing_in_it() {
        assert_eq!(
            TokenStream::from("").parse::<Crate<_>>(),
            Ok(Crate {
                inner_docs: vec![],
                items: vec![],
            })
        );
    }

    /// A file of nothing but comments still has a place for every one of them.
    #[test]
    fn a_file_may_hold_only_docs_and_whitespace() {
        let file = TokenStream::from("//! about the file\n/// about nothing\n")
            .parse::<Crate<_>>()
            .expect("docs alone should parse");

        assert_eq!(
            file.inner_docs,
            vec![InnerDoc::Line(
                None,
                InnerLineDoc(TokenStream::from((0, "//! about the file"))),
            )],
        );

        // An outer document carries the whitespace that precedes it, which is what
        // keeps it attached to the statement it documents.
        assert_eq!(
            file.items,
            vec![
                Item::OuterDoc(OuterDoc::Line(
                    Some(S(TokenStream::from((18, "\n")))),
                    OuterLineDoc(TokenStream::from((19, "/// about nothing"))),
                )),
                // Then the newline that ends that document is an item of its own.
                Item::S(S(TokenStream::from((36, "\n")))),
            ],
        );
    }

    /// The whitespace between statements is an item, not something skipped.
    #[test]
    fn whitespace_between_items_is_an_item() {
        let file = TokenStream::from("//! a\n\nwhitespace W -> ' ';")
            .parse::<Crate<_>>()
            .expect("a doc, a blank line and a statement should parse");

        assert_eq!(
            file.inner_docs,
            vec![InnerDoc::Line(
                None,
                InnerLineDoc(TokenStream::from((0, "//! a"))),
            )],
        );

        assert_eq!(
            file.items,
            vec![
                Item::S(S(TokenStream::from((5, "\n\n")))),
                Item::Stmt(Stmt::Whitespace {
                    keyword: Whitespace(
                        TokenStream::from((7, "whitespace")),
                        Some(S(TokenStream::from((17, " ")))),
                    ),
                    ident: Ident(TokenStream::from((18, "W"))),
                    arrow_right: ArrowRight(
                        Some(S(TokenStream::from((19, " ")))),
                        TokenStream::from((20, "->")),
                        Some(S(TokenStream::from((22, " ")))),
                    ),
                    expr: Expr {
                        first: ExprNoTopAlts {
                            first: ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Str(LitStr {
                                delimiter_start: TokenStream::from((23, "'")),
                                content: vec![StrSegment::CharWithException(TokenStream::from((
                                    24, " ",
                                )))],
                                delimiter_end: TokenStream::from((25, "'")),
                            })),
                            rest: vec![],
                        },
                        rest: vec![],
                    },
                    semi: Semi(None, TokenStream::from((26, ";")), None),
                }),
            ],
        );
    }

    /// A statement is what the file's items exist to hold.
    #[test]
    fn a_statement_is_an_item_of_its_own() {
        let file = TokenStream::from("syntax S -> 'a';")
            .parse::<Crate<_>>()
            .expect("a statement should parse");

        assert_eq!(file.items.len(), 1);

        let Item::Stmt(stmt) = &file.items[0] else {
            panic!("{:?}", file.items[0]);
        };

        assert_eq!(stmt.to_span(), Span::Range(0..16));
        assert_eq!(file.items[0].to_span(), Span::Range(0..16));
    }

    /// Nothing an item cannot read is an error: the file was not read to its
    /// end, and what is left over is reported as lying where the file should.
    #[test]
    fn a_file_that_does_not_end_where_its_items_do_is_broken() {
        assert_eq!(
            TokenStream::from("}}} ").parse::<Crate<_>>(),
            Err(UnsynError::Syntax(
                SyntaxKind::Eof,
                ControlFlow::Fatal,
                Span::Range(0..4),
            )),
        );

        // The statement is read, and so is the whitespace before the stray `}`:
        // it belongs to the semicolon, and only the `}` is left over.
        assert_eq!(
            TokenStream::from("syntax S -> 'a'; }").parse::<Crate<_>>(),
            Err(UnsynError::Syntax(
                SyntaxKind::Eof,
                ControlFlow::Fatal,
                Span::Range(17..18),
            )),
        );

        // The same file without it ends where its items do.
        assert!(
            TokenStream::from("syntax S -> 'a';")
                .parse::<Crate<_>>()
                .is_ok(),
            "a file of one statement should be read to its end"
        );
    }

    /// A document is attached to the statement after it by position alone.
    #[test]
    fn a_doc_before_a_statement_is_the_item_that_precedes_it() {
        let file = TokenStream::from("/// a name\nsyntax S -> 'a';")
            .parse::<Crate<_>>()
            .expect("a doc and a statement should parse");

        assert!(
            matches!(file.items[0], Item::OuterDoc(_)),
            "{:?}",
            file.items[0]
        );
        assert!(matches!(file.items[1], Item::S(_)), "{:?}", file.items[1]);
        assert!(
            matches!(file.items[2], Item::Stmt(_)),
            "{:?}",
            file.items[2]
        );
        assert_eq!(
            file.items.last().map(Item::to_span),
            Some(Span::Range(11..27)),
        );
    }
}

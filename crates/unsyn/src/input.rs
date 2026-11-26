//! The input types for `unsyn` parsing.

use std::fmt::Debug;

use parserc::{AsBytes, AsStr, Find, Input, StartWith};

/// The capabilities an input stream needs to parse `unsyn`.
///
/// The lexical layer of the language — the code a build wrote for `unsyn.us` —
/// is written against these traits instead of against one concrete stream, so
/// that a caller may feed the parser
/// anything it likes — a borrowed `&str`, a rope, a slice of a larger file — as
/// long as it can:
///
/// - [`Input<Item = char>`]: be consumed one character at a time and split, and
///   report failures as [`ParseError`](crate::error::ParseError);
/// - [`AsStr`] and [`AsBytes`]: expose what is left of it, which the multi
///   character keywords (`whitespace`, `*/`, `\\x`) and the fixed-width escape
///   checks need;
/// - [`StartWith`]: test a literal prefix without consuming it;
/// - [`Find`]: look ahead for a literal, which the comment scanners use;
/// - [`Clone`], [`Debug`] and [`PartialEq`]: snapshot a position and compare
///   tokens, both of which the recursive-descent parsers and the tests rely on.
///
/// The error a stream reports is the one a build wrote for the spec —
/// [`ParseError`](crate::error::ParseError) — and not the
/// [`UnsynError`](crate::error::UnsynError) the
/// passes below collect, which is that error wrapped around the families no
/// grammar states. A pass that both reads a file and judges it returns the
/// wrapper, and `?` carries a reading failure into it on its own.
pub trait UnsynInput:
    Input<Item = char, Error = crate::error::ParseError>
    + AsBytes
    + AsStr
    + StartWith<&'static str>
    + StartWith<&'static [u8]>
    + Find<&'static str>
    + Find<&'static [u8]>
    + Clone
    + Debug
    + PartialEq
{
}

/// The [`UnsynInput`] over a borrowed [`str`].
///
/// A token keeps the slice it was read from and the offset it started at, so
/// every node of the CST can report a [`Span`](parserc::Span) back into the
/// source text.
pub type TokenStream<'a> = parserc::chars::TokenStream<'a, crate::error::ParseError>;

impl<'a> UnsynInput for TokenStream<'a> {}

#[cfg(test)]
mod tests {
    use parserc::{AsBytes, AsStr, Find, Input, Span, StartWith, ToSpan};

    use super::*;

    #[test]
    fn a_token_carries_its_offset_and_content() {
        let mut input = TokenStream::from("lexer ABC -> 'a';");

        let head = input.split_to(5);

        assert_eq!(head.as_str(), "lexer");
        assert_eq!(head.to_span(), Span::Range(0..5));

        // The rest of the stream is what is left of it.
        assert_eq!(input.as_str(), " ABC -> 'a';");
        assert_eq!(input.to_span(), Span::Range(5..17));
    }

    #[test]
    fn the_byte_view_agrees_with_the_char_view() {
        // The DSL allows non-ASCII in a string literal, so the two lengths
        // differ and a scanner must not mix them up.
        let input = TokenStream::from("'～' ABC");

        assert_eq!(input.as_str().chars().count(), 7);
        assert_eq!(input.as_bytes().len(), 9);
    }

    #[test]
    fn prefixes_are_tested_without_consuming() {
        let input = TokenStream::from("-> rest");

        // A prefix match reports its length, not a boolean.
        assert_eq!(input.starts_with("->"), Some(2));
        assert_eq!(input.starts_with("-<"), None);
        assert_eq!(input.as_str(), "-> rest");

        assert_eq!(input.find("rest"), Some(3));
        assert_eq!(input.find("missing"), None);
    }

    #[test]
    fn an_empty_stream_has_an_empty_span() {
        let input = TokenStream::from("");

        assert_eq!(input.len(), 0);
        assert!(input.is_empty());
        assert_eq!(input.to_span(), Span::Range(0..0));
    }
}

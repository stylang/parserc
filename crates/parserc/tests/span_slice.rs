//! `as_str` / `as_bytes`: reading the source text a span covers.
//!
//! These are the two functions that let anything report its own text without
//! rebuilding it from parts, so what they do at the edges — a value that lies
//! behind the stream, a span wider than what is left, an open region — is what
//! a caller ends up depending on.

use parserc::{
    AsStr, Input, Span, ToSpan, bytes,
    chars::TokenStream,
    syntax::{Byte, Char, Syntax, SyntaxInput},
};

/// A node of two parts with the trivia between them kept, so that its span is
/// wider than either field and the space is inside the node rather than outside.
#[derive(Debug, PartialEq, Syntax)]
struct Pair<I>(
    #[parserc(keyword = "ab")] pub I,
    pub Option<S<I>>,
    #[parserc(keyword = "cd")] pub I,
)
where
    I: parserc::chars::CharsInput;

/// The trivia of the language under test: a run of spaces.
#[derive(Debug, PartialEq, Eq, Clone, Syntax)]
#[parserc(take_while = |c: char| c == ' ')]
pub struct S<I>(pub I)
where
    I: parserc::chars::CharsInput;

type Chars<'a> = TokenStream<'a>;

#[test]
fn a_span_reads_back_exactly_the_text_it_covers() {
    let source = "ab cd ef";
    let whole = Chars::from(source);

    let mut at = Chars::from(source);
    let pair = at.parse::<Pair<_>>().unwrap();

    // The node kept two keywords; the text is the region between them and the
    // space between the pair, which no single field of it holds.
    assert_eq!(pair.to_span(), Span::Range(0..5));
    assert_eq!(parserc::as_str(&whole, &pair), "ab cd");
}

#[test]
fn a_field_is_only_a_window_on_its_own_part() {
    // Measured, not assumed: `keyword` and friends return the text they took by
    // `split_to`, which leaves the field's stream holding that token and nothing
    // after it. So a field is a window on its own part, not on the node around
    // it, and asking for the node's whole text through one of its fields gets
    // the field back. Only a stream that still covers the region — the source,
    // or a stream that has not been consumed past it — answers for the node.
    let source = "ab cd ef";
    let whole = Chars::from(source);
    let mut at = Chars::from(source);
    let pair = at.parse::<Pair<_>>().unwrap();

    assert_eq!(pair.to_span(), Span::Range(0..5));
    assert_eq!(pair.0.to_span(), Span::Range(0..2));
    assert_eq!(pair.2.to_span(), Span::Range(3..5));

    // Clipped to the field's own reach, which is all it can answer for.
    assert_eq!(parserc::as_str(&pair.0, &pair), "ab");
    assert_eq!(parserc::as_str(&pair.2, &pair), "cd");

    // And the whole still reads back in full.
    assert_eq!(parserc::as_str(&whole, &pair), "ab cd");
}

#[test]
fn a_token_reads_back_its_own_text() {
    let source = "ab cd";
    let whole = Chars::from(source);

    let mut at = Chars::from(source);
    let first = at.parse::<Char<_, 'a'>>().unwrap();

    assert_eq!(parserc::as_str(&whole, &first), "a");
}

#[test]
fn a_value_behind_the_stream_answers_empty() {
    let source = "ab cd ef";
    let mut at = Chars::from(source);
    let pair = at.parse::<Pair<_>>().unwrap();

    // `at` has been consumed past the pair it just read, so the pair's region is
    // no longer `at`'s to name. Nothing lies about it: the answer is empty.
    assert_eq!(at.as_str(), " ef");
    assert_eq!(parserc::as_str(&at, &pair), "");

    // The whole source still answers for it.
    let whole = Chars::from(source);
    assert_eq!(parserc::as_str(&whole, &pair), "ab cd");
}

#[test]
fn a_span_wider_than_the_stream_is_cut_to_it() {
    // A node read from a prefix of the source, reported into the whole of it.
    let piece = Chars::from("ab");
    let at = piece.clone();
    let pair = {
        let mut inner = Chars::from("ab cd");
        inner.parse::<Pair<_>>().unwrap()
    };

    assert_eq!(parserc::as_str(&at, &pair), "ab");
    assert_eq!(parserc::as_str(&piece, &pair), "ab");
}

#[test]
fn an_open_region_answers_empty() {
    struct Open;

    impl ToSpan for Open {
        fn to_span(&self) -> Span {
            Span::RangeFrom(2..)
        }
    }

    // An open region is closed by the stream it is read against, which is the
    // only bound it can have: `2..` over six bytes is the last four of them.
    let whole = Chars::from("abcdef");
    assert_eq!(parserc::as_str(&whole, &Open), "cdef");

    struct Nothing;

    impl ToSpan for Nothing {
        fn to_span(&self) -> Span {
            Span::None
        }
    }

    assert_eq!(parserc::as_str(&whole, &Nothing), "");
}

#[test]
fn multibyte_offsets_are_bytes_not_characters() {
    // A `Span` counts bytes and a character stream splits on them, so a node
    // after a multibyte character is still found by its own span.
    let source = "éab cd";
    let whole = Chars::from(source);

    // Skip the multibyte character by its own width, which is two bytes.
    let width = 'é'.len_utf8();
    assert_eq!(width, 2, "the character is multibyte");

    let mut at = Chars::from(source);
    let _ = at.split_to(width);
    let pair = at.parse::<Pair<_>>().unwrap();

    assert_eq!(parserc::as_str(&whole, &pair), "ab cd");
}

#[test]
fn bytes_read_the_same_region_as_text() {
    let source = "ab cd";
    let whole: bytes::TokenStream<parserc::Kind> = bytes::TokenStream::from(source);

    // A byte token reports its own span, so the helper finds the same one byte
    // that the text helper finds one character.
    let mut at: bytes::TokenStream<parserc::Kind> = bytes::TokenStream::from(source);
    let first = at.parse::<Byte<_, b'a'>>().unwrap();

    assert_eq!(parserc::as_bytes(&whole, &first), b"a");
    assert_eq!(parserc::as_str(&whole, &first), "a");
}

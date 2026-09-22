use std::{cmp, fmt::Debug};

use crate::{ParseError, Span};

/// Extension trait matching a `needle` at the start of an [`Input`].
pub trait StartWith<Needle> {
    /// Returns the matched length when `self` starts with `needle`, otherwise `None`.
    fn starts_with(&self, needle: Needle) -> Option<usize>;
}

/// Extension trait searching for a `needle` inside an [`Input`].
pub trait Find<Needle> {
    /// Returns the offset of the first occurrence of `needle`, or `None`.
    fn find(&self, needle: Needle) -> Option<usize>;
}

/// Exposes the remaining input as raw bytes.
pub trait AsBytes {
    /// Returns the remaining input as a byte slice.
    fn as_bytes(&self) -> &[u8];
}

/// Reports the encoded length of an item.
pub trait Length {
    /// Returns the encoded length in bytes.
    fn len(&self) -> usize;
}

impl Length for &str {
    fn len(&self) -> usize {
        str::len(self)
    }
}

impl Length for &[u8] {
    fn len(&self) -> usize {
        <[u8]>::len(self)
    }
}

/// Exposes the remaining input as a `&str`.
pub trait AsStr {
    /// Returns the remaining input as a `&str` slice.
    fn as_str(&self) -> &str;
}

/// A single item of an [`Input`] sequence.
pub trait Item: PartialEq + Clone + Copy + Debug {
    /// Returns the encoded length of this item in bytes.
    fn len(&self) -> usize;
}

impl Item for u8 {
    #[inline(always)]
    fn len(&self) -> usize {
        1
    }
}

impl Item for char {
    #[inline(always)]
    fn len(&self) -> usize {
        self.len_utf8()
    }
}

/// A source code stream consumed by parsers.
pub trait Input: PartialEq + Debug {
    /// Sequence item of this stream.
    type Item: Item;
    /// Parsing error type.
    type Error: ParseError;
    /// Iterator type returns by [`iter`](Input::iter).
    type Iter: Iterator<Item = Self::Item>;
    /// Iterator type returns by [`iter_indices`](Input::iter_indices).
    type IterIndices: Iterator<Item = (usize, Self::Item)>;

    /// Returns the remaining length of the input in bytes.
    fn len(&self) -> usize;

    /// Splits the input at `at`, consuming and returning the prefix `[0, at)`.
    ///
    /// Afterwards `self` contains elements `[at, len)` and the returned `Self`
    /// contains elements `[0, at)`.
    fn split_to(&mut self, at: usize) -> Self;

    /// Splits the input at `at`, keeping the prefix `[0, at)` and returning the rest.
    ///
    /// Afterwards `self` contains elements `[0, at)` and the returned `Self`
    /// contains elements `[at, len)`.
    fn split_off(&mut self, at: usize) -> Self;

    /// Returns an iterator over the remaining items.
    fn iter(&self) -> Self::Iter;

    /// Returns an iterator over `(offset, item)` pairs of the remaining input.
    fn iter_indices(&self) -> Self::IterIndices;

    /// Returns the start position of this input in the whole source code.
    fn start(&self) -> usize;

    /// Returns the end position of this input in the whole source code.
    fn end(&self) -> usize;

    /// Returns true if this input length == 0.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the region of this input in the whole source code.
    #[inline]
    fn to_span(&self) -> Span {
        Span::Range(self.start()..self.end())
    }

    /// Returns a span covering `at` bytes from the start of this input,
    /// clamped to its end.
    #[inline]
    fn to_span_at(&self, at: usize) -> Span {
        Span::Range(self.start()..cmp::min(self.start() + at, self.end()))
    }
}

/// Extension trait exposing [`to_span`](ToSpan::to_span).
pub trait ToSpan {
    /// Returns the region of this input.
    fn to_span(&self) -> Span;
}

impl<I> ToSpan for Option<I>
where
    I: Input,
{
    #[inline]
    fn to_span(&self) -> Span {
        match self {
            Some(input) => input.to_span(),
            None => Span::None,
        }
    }
}

/// Byte-oriented input implementations.
#[cfg(feature = "input")]
pub mod bytes {
    use std::{iter::Enumerate, marker::PhantomData, str::Bytes};

    use memchr::memmem;

    use crate::Kind;

    use super::*;
    /// [`Input`] over [`u8`] items with byte search helpers.
    pub trait BytesInput:
        Input<Item = u8>
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

    /// A `&str`-backed [`Input`] over [`u8`] items that tracks its absolute offset.
    #[derive(Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct TokenStream<'a, Error = Kind> {
        /// Absolute offset of this stream inside the whole source.
        pub offset: usize,
        /// The remaining segment of the source.
        pub value: &'a str,
        /// Marker for the error type of this stream.
        _marker: PhantomData<Error>,
    }

    impl<'a, E> Clone for TokenStream<'a, E> {
        fn clone(&self) -> Self {
            Self {
                offset: self.offset,
                value: self.value,
                _marker: Default::default(),
            }
        }
    }

    impl<'a, E> Debug for TokenStream<'a, E> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TokenStream::from(({},{:?}))", self.offset, self.value)
        }
    }

    impl<'a, E> PartialEq for TokenStream<'a, E> {
        fn eq(&self, other: &Self) -> bool {
            self.offset == other.offset && self.value == other.value
        }
    }

    impl<'a, E> From<&'a str> for TokenStream<'a, E> {
        fn from(value: &'a str) -> Self {
            TokenStream {
                offset: 0,
                value,
                _marker: Default::default(),
            }
        }
    }

    impl<'a, E> From<(usize, &'a str)> for TokenStream<'a, E> {
        fn from(value: (usize, &'a str)) -> Self {
            TokenStream {
                offset: value.0,
                value: value.1,
                _marker: Default::default(),
            }
        }
    }

    impl<'a, E> Input for TokenStream<'a, E>
    where
        E: ParseError,
    {
        type Item = u8;

        type Error = E;

        type Iter = Bytes<'a>;

        type IterIndices = Enumerate<Self::Iter>;

        #[inline]
        fn len(&self) -> usize {
            self.value.len()
        }

        #[inline]
        fn split_to(&mut self, at: usize) -> Self {
            let (first, last) = self.value.split_at(at);

            self.value = last;
            let offset = self.offset;
            self.offset += at;

            TokenStream {
                offset,
                value: first,
                _marker: Default::default(),
            }
        }

        #[inline]
        fn split_off(&mut self, at: usize) -> Self {
            let (first, last) = self.value.split_at(at);

            self.value = first;

            TokenStream {
                offset: self.offset + at,
                value: last,
                _marker: Default::default(),
            }
        }

        #[inline]
        fn iter(&self) -> Self::Iter {
            self.value.bytes()
        }

        #[inline]
        fn iter_indices(&self) -> Self::IterIndices {
            self.iter().enumerate()
        }

        #[inline]
        fn start(&self) -> usize {
            self.offset
        }

        #[inline]
        fn end(&self) -> usize {
            self.offset + self.value.len()
        }
    }

    impl<'a, E> AsBytes for TokenStream<'a, E> {
        #[inline]
        fn as_bytes(&self) -> &[u8] {
            self.value.as_bytes()
        }
    }

    impl<'a, E> AsStr for TokenStream<'a, E> {
        #[inline]
        fn as_str(&self) -> &str {
            self.value
        }
    }

    impl<'a, E> StartWith<&str> for TokenStream<'a, E> {
        #[inline]
        fn starts_with(&self, needle: &str) -> Option<usize> {
            if self.as_bytes().starts_with(needle.as_bytes()) {
                Some(needle.len())
            } else {
                None
            }
        }
    }

    impl<'a, E> StartWith<&[u8]> for TokenStream<'a, E> {
        #[inline]
        fn starts_with(&self, needle: &[u8]) -> Option<usize> {
            if self.as_bytes().starts_with(needle) {
                Some(needle.len())
            } else {
                None
            }
        }
    }

    impl<'a, const N: usize, E> StartWith<&[u8; N]> for TokenStream<'a, E> {
        #[inline]
        fn starts_with(&self, needle: &[u8; N]) -> Option<usize> {
            if self.as_bytes().starts_with(needle) {
                Some(needle.len())
            } else {
                None
            }
        }
    }

    impl<'a, E> Find<&str> for TokenStream<'a, E> {
        #[inline]
        fn find(&self, needle: &str) -> Option<usize> {
            memmem::find(self.as_bytes(), needle.as_bytes())
        }
    }

    impl<'a, E> Find<&[u8]> for TokenStream<'a, E> {
        #[inline]
        fn find(&self, needle: &[u8]) -> Option<usize> {
            memmem::find(self.as_bytes(), needle)
        }
    }

    impl<'a, const N: usize, E> Find<&[u8; N]> for TokenStream<'a, E> {
        #[inline]
        fn find(&self, needle: &[u8; N]) -> Option<usize> {
            memmem::find(self.as_bytes(), needle)
        }
    }

    impl<'a, E> BytesInput for TokenStream<'a, E> where E: ParseError + Clone {}
}

/// [`char`] input implementations.
#[cfg(feature = "input")]
pub mod chars {
    use std::{
        marker::PhantomData,
        str::{CharIndices, Chars},
    };

    use memchr::memmem;

    use crate::Kind;

    use super::*;
    /// [`Input`] over [`char`] items with search helpers.
    pub trait CharsInput:
        Input<Item = char>
        + AsBytes
        + AsStr
        + StartWith<&'static str>
        + Find<&'static str>
        + Clone
        + Debug
        + PartialEq
    {
    }

    /// A `&str`-backed [`Input`] over [`char`] items that tracks its absolute offset.
    #[derive(Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct TokenStream<'a, Error = Kind> {
        /// Absolute offset of this stream inside the whole source.
        pub offset: usize,
        /// The remaining segment of the source.
        pub value: &'a str,
        /// Marker for the error type of this stream.
        _marker: PhantomData<Error>,
    }

    impl<'a, E> Clone for TokenStream<'a, E> {
        fn clone(&self) -> Self {
            Self {
                offset: self.offset,
                value: self.value,
                _marker: Default::default(),
            }
        }
    }

    impl<'a, E> Debug for TokenStream<'a, E> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TokenStream::from(({},{:?}))", self.offset, self.value)
        }
    }

    impl<'a, E> PartialEq for TokenStream<'a, E> {
        fn eq(&self, other: &Self) -> bool {
            self.offset == other.offset && self.value == other.value
        }
    }

    impl<'a, E> From<&'a str> for TokenStream<'a, E> {
        fn from(value: &'a str) -> Self {
            TokenStream {
                offset: 0,
                value,
                _marker: Default::default(),
            }
        }
    }

    impl<'a, E> From<(usize, &'a str)> for TokenStream<'a, E> {
        fn from(value: (usize, &'a str)) -> Self {
            TokenStream {
                offset: value.0,
                value: value.1,
                _marker: Default::default(),
            }
        }
    }

    impl<'a, E> Input for TokenStream<'a, E>
    where
        E: ParseError,
    {
        type Item = char;

        type Error = E;

        type Iter = Chars<'a>;

        type IterIndices = CharIndices<'a>;

        #[inline]
        fn len(&self) -> usize {
            self.value.len()
        }

        #[inline]
        fn split_to(&mut self, at: usize) -> Self {
            let (first, last) = self.value.split_at(at);

            self.value = last;
            let offset = self.offset;
            self.offset += at;

            TokenStream {
                offset,
                value: first,
                _marker: Default::default(),
            }
        }

        #[inline]
        fn split_off(&mut self, at: usize) -> Self {
            let (first, last) = self.value.split_at(at);

            self.value = first;

            TokenStream {
                offset: self.offset + at,
                value: last,
                _marker: Default::default(),
            }
        }

        #[inline]
        fn iter(&self) -> Self::Iter {
            self.value.chars()
        }

        #[inline]
        fn iter_indices(&self) -> Self::IterIndices {
            self.value.char_indices()
        }

        #[inline]
        fn start(&self) -> usize {
            self.offset
        }

        #[inline]
        fn end(&self) -> usize {
            self.offset + self.value.len()
        }
    }

    impl<'a, E> AsBytes for TokenStream<'a, E> {
        #[inline]
        fn as_bytes(&self) -> &[u8] {
            self.value.as_bytes()
        }
    }

    impl<'a, E> AsStr for TokenStream<'a, E> {
        #[inline]
        fn as_str(&self) -> &str {
            self.value
        }
    }

    impl<'a, E> StartWith<&str> for TokenStream<'a, E> {
        #[inline]
        fn starts_with(&self, needle: &str) -> Option<usize> {
            if self.as_bytes().starts_with(needle.as_bytes()) {
                Some(needle.len())
            } else {
                None
            }
        }
    }

    impl<'a, E> StartWith<&[u8]> for TokenStream<'a, E> {
        #[inline]
        fn starts_with(&self, needle: &[u8]) -> Option<usize> {
            if self.as_bytes().starts_with(needle) {
                Some(needle.len())
            } else {
                None
            }
        }
    }

    impl<'a, const N: usize, E> StartWith<&[u8; N]> for TokenStream<'a, E> {
        #[inline]
        fn starts_with(&self, needle: &[u8; N]) -> Option<usize> {
            if self.as_bytes().starts_with(needle) {
                Some(needle.len())
            } else {
                None
            }
        }
    }

    impl<'a, E> Find<&str> for TokenStream<'a, E> {
        #[inline]
        fn find(&self, needle: &str) -> Option<usize> {
            memmem::find(self.as_bytes(), needle.as_bytes())
        }
    }

    impl<'a, E> Find<&[u8]> for TokenStream<'a, E> {
        #[inline]
        fn find(&self, needle: &[u8]) -> Option<usize> {
            memmem::find(self.as_bytes(), needle)
        }
    }

    impl<'a, const N: usize, E> Find<&[u8; N]> for TokenStream<'a, E> {
        #[inline]
        fn find(&self, needle: &[u8; N]) -> Option<usize> {
            memmem::find(self.as_bytes(), needle)
        }
    }

    impl<'a, E> CharsInput for TokenStream<'a, E> where E: ParseError + Clone {}
}

// The `TokenStream` inputs live behind the `input` feature.
#[cfg(all(test, feature = "input"))]
mod tests {
    use super::*;

    type Bytes = bytes::TokenStream<'static>;
    type Chars = chars::TokenStream<'static>;

    #[test]
    fn length_and_item_sizes_follow_the_encoding() {
        let text: &str = "abc";
        let raw: &[u8] = b"abc";

        assert_eq!(Length::len(&text), 3);
        assert_eq!(Length::len(&raw), 3);

        assert_eq!(Item::len(&b'a'), 1);
        assert_eq!(Item::len(&'a'), 1);
        assert_eq!(Item::len(&'é'), 2);
    }

    #[test]
    fn bytes_stream_tracks_absolute_offsets() {
        let input = Bytes::from("abc");

        assert_eq!(input.start(), 0);
        assert_eq!(input.end(), 3);
        assert_eq!(input.len(), 3);
        assert_eq!(input.to_span(), Span::Range(0..3));

        let input = Bytes::from((5, "abc"));

        assert_eq!(input.start(), 5);
        assert_eq!(input.end(), 8);
        assert!(!input.is_empty());
        assert!(Bytes::from("").is_empty());
    }

    #[test]
    fn split_to_returns_the_prefix_and_moves_the_rest() {
        let mut input = Bytes::from((5, "abcd"));

        let prefix = input.split_to(2);

        assert_eq!(prefix.as_str(), "ab");
        assert_eq!(prefix.start(), 5);
        assert_eq!(prefix.end(), 7);
        assert_eq!(input.as_str(), "cd");
        assert_eq!(input.start(), 7);
        assert_eq!(input.end(), 9);
    }

    #[test]
    fn split_off_keeps_the_prefix() {
        let mut input = Bytes::from((5, "abcd"));

        let suffix = input.split_off(2);

        assert_eq!(input.as_str(), "ab");
        assert_eq!(input.start(), 5);
        assert_eq!(suffix.as_str(), "cd");
        assert_eq!(suffix.start(), 7);
        assert_eq!(suffix.end(), 9);
    }

    #[test]
    fn iterators_cover_the_remaining_items() {
        let mut input = Bytes::from((5, "abc"));
        input.split_to(1);

        let items: Vec<u8> = input.iter().collect();
        let indexed: Vec<(usize, u8)> = input.iter_indices().collect();

        assert_eq!(items, b"bc".to_vec());
        // Indices are relative to the remaining slice, offsets stay absolute.
        assert_eq!(indexed, vec![(0, b'b'), (1, b'c')]);
        assert_eq!(input.start(), 6);
    }

    #[test]
    fn views_expose_bytes_and_str() {
        let input = Bytes::from("abc");

        assert_eq!(input.as_bytes(), &b"abc"[..]);
        assert_eq!(input.as_str(), "abc");
    }

    #[test]
    fn starts_with_reports_the_matched_length() {
        let input = Bytes::from("abc");

        assert_eq!(StartWith::<&str>::starts_with(&input, "ab"), Some(2));
        assert_eq!(StartWith::<&str>::starts_with(&input, "z"), None);
        assert_eq!(StartWith::<&[u8]>::starts_with(&input, &b"ab"[..]), Some(2));
        assert_eq!(StartWith::<&[u8]>::starts_with(&input, &b"bc"[..]), None);
        assert_eq!(StartWith::<&[u8; 2]>::starts_with(&input, b"ab"), Some(2));
    }

    #[test]
    fn find_reports_the_first_slice_relative_offset() {
        let input = Bytes::from((5, "abcabc"));

        assert_eq!(Find::<&str>::find(&input, "bc"), Some(1));
        assert_eq!(Find::<&str>::find(&input, "z"), None);
        assert_eq!(Find::<&[u8]>::find(&input, &b"ca"[..]), Some(2));
        assert_eq!(Find::<&[u8; 2]>::find(&input, b"ca"), Some(2));
    }

    #[test]
    fn to_span_at_clamps_to_the_end() {
        let input = Bytes::from((5, "a"));

        assert_eq!(input.to_span_at(0), Span::Range(5..5));
        assert_eq!(input.to_span_at(3), Span::Range(5..6));
        assert_eq!(Bytes::from((5, "abc")).to_span_at(2), Span::Range(5..7));
    }

    #[test]
    fn to_span_of_an_optional_input_is_none_without_a_value() {
        let input = Bytes::from((5, "abc"));

        assert_eq!(Some(input).to_span(), Span::Range(5..8));
        assert_eq!(Option::<Bytes>::None.to_span(), Span::None);
    }

    #[test]
    fn chars_stream_splits_on_char_boundaries() {
        let mut input = Chars::from((5, "é!"));

        let prefix = input.split_to(2);

        assert_eq!(prefix.as_str(), "é");
        assert_eq!(prefix.to_span(), Span::Range(5..7));
        assert_eq!(input.as_str(), "!");
        assert_eq!(input.start(), 7);
    }

    #[test]
    fn chars_stream_iterates_over_chars() {
        let input = Chars::from("éa");

        let items: Vec<char> = input.iter().collect();
        let indexed: Vec<(usize, char)> = input.iter_indices().collect();

        assert_eq!(items, vec!['é', 'a']);
        // Char indices carry byte offsets inside the remaining slice.
        assert_eq!(indexed, vec![(0, 'é'), (2, 'a')]);
    }

    #[test]
    fn chars_stream_searches_with_str_needles() {
        let input = Chars::from("héllo");

        // Needle lengths and offsets are measured in bytes.
        assert_eq!(StartWith::<&str>::starts_with(&input, "hé"), Some(3));
        assert_eq!(StartWith::<&str>::starts_with(&input, "x"), None);
        assert_eq!(Find::<&str>::find(&input, "ll"), Some(3));
    }
}

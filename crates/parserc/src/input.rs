use std::{cmp, fmt::Debug};

use crate::{ParseError, Span};

/// An extension trait provides extra `starts_with` func to `Input`.
pub trait StartWith<Needle> {
    /// Convert the input type to a byte slice
    fn starts_with(&self, needle: Needle) -> Option<usize>;
}

/// An extension trait providers extra `find` func to `Input`.
pub trait Find<Needle> {
    /// Returns the index of the first occurrence of the given needle.
    fn find(&self, needle: Needle) -> Option<usize>;
}

/// Convert `Input` as `&[u8]`
pub trait AsBytes {
    /// Convert the input type to a byte slice
    fn as_bytes(&self) -> &[u8];
}

/// A trait to fetch item length.
pub trait Length {
    /// Returns item length.
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

/// Convert `Input` as `&str`
pub trait AsStr {
    /// Convert the input type to a str slice
    fn as_str(&self) -> &str;
}

/// The item type of the input sequence.
pub trait Item: PartialEq + Clone + Copy + Debug {
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

/// Input sequence for source code.
pub trait Input: PartialEq + Debug {
    /// Sequeue item.
    type Item: Item;
    /// Parsing error type.
    type Error: ParseError;
    /// Iterator type returns by [`iter`](Input::iter).
    type Iter: Iterator<Item = Self::Item>;
    /// Iterator type returns by [`iter_indices`](Input::iter_indices).
    type IterIndices: Iterator<Item = (usize, Self::Item)>;

    // Returns current input sequence length.
    fn len(&self) -> usize;

    /// Split the input into two at the given index.
    ///
    /// Afterwards self contains elements [at, len), and the returned BytesMut contains elements [0, at).
    fn split_to(&mut self, at: usize) -> Self;

    /// Split the input into two at the given index.
    ///
    /// Afterwards self contains elements [0, at), and the returned `Self` contains elements [at, capacity).
    fn split_off(&mut self, at: usize) -> Self;

    /// Returns an immutable iterator over source code chars.
    fn iter(&self) -> Self::Iter;

    /// Returns an immutable iterator over source code chars.
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

    /// Returns the region from `start` of this input to `at` position.
    #[inline]
    fn to_span_at(&self, at: usize) -> Span {
        Span::Range(self.start()..cmp::min(self.start() + at, self.end()))
    }
}

/// bytes input implementation.
#[cfg(feature = "input")]
pub mod bytes {
    use std::{iter::Enumerate, marker::PhantomData, str::Bytes};

    use memchr::memmem;

    use crate::Kind;

    use super::*;
    /// Input for bytes.
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

    /// `BytesInput` implementation.
    #[derive(Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct TokenStream<'a, Error = Kind> {
        /// offset in the whole token stream.
        pub offset: usize,
        /// current segement string int the whole token stream.
        pub value: &'a str,
        /// Error for this input.
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

/// chars input implementation.
#[cfg(feature = "input")]
pub mod chars {
    use std::{
        marker::PhantomData,
        str::{CharIndices, Chars},
    };

    use memchr::memmem;

    use crate::Kind;

    use super::*;
    /// Input for bytes.
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

    /// `BytesInput` implementation.
    #[derive(Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct TokenStream<'a, Error = Kind> {
        /// offset in the whole token stream.
        pub offset: usize,
        /// current segement string int the whole token stream.
        pub value: &'a str,
        /// Error for this input.
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

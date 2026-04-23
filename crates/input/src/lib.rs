//！`input` represents the source data stream to be parsed

#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{
    cmp,
    fmt::Debug,
    hash::Hash,
    iter::{Copied, Enumerate},
    marker::PhantomData,
    slice::Iter,
};

use memchr::memmem;

/// Type alias for [sourcespan::Span<usize>] in this crate.
pub type Span = sourcespan::Span<usize>;

/// Values yielded by the input stream.
pub trait Item: PartialEq + Eq + PartialOrd + Ord + Hash + Copy + Debug {
    /// Returns the length of this item in bytes.
    fn len(self) -> usize;
}

impl Item for char {
    /// Returns the number of bytes this char would need if encoded in UTF-8.
    #[inline(always)]
    fn len(self) -> usize {
        self.len_utf8()
    }
}

impl Item for u8 {
    #[inline]
    fn len(self) -> usize {
        1
    }
}

/// An extension trait add `len` fn to types.
pub trait Length {
    /// Returns the length in bytes.
    fn len(&self) -> usize;

    /// Returns true if this  length == 0.
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An extension `trait` adds a `to_span` function to types.
pub trait ToSpan {
    /// Return the span of the object in the source code.
    fn to_span(&self) -> Span;
}

/// A source code stream must implement this trait.
pub trait Input {
    /// Error raised by combinators.
    type Error;

    /// Value yielded by this `Input`.
    type Item: Item;

    /// Iterator type returns by [`iter`](Input::iter).
    type Iter: Iterator<Item = Self::Item>;

    /// Iterator type returns by [`iter_indices`](Input::iter_indices).
    type IterIndices: Iterator<Item = (usize, Self::Item)>;

    /// Result type of fn [`split_at`](Input::split_at)
    type Split: Input;

    /// Split the input into two at the given index.
    fn split_at(self, mid: usize) -> (Self::Split, Self::Split)
    where
        Self: Sized;

    /// Returns an immutable iterator over source code chars.
    fn iter(&self) -> Self::Iter;

    /// Returns an immutable iterator over source code chars.
    fn iter_indices(&self) -> Self::IterIndices;

    /// Returns the start position of this input in the whole source code.
    fn start(&self) -> usize;

    /// Returns the end position of this input in the whole source code.
    fn end(&self) -> usize;

    /// Returns the subspan of the current input fragment
    #[inline]
    fn to_span_with(&self, len: usize) -> Span {
        Span::from(self.start()..cmp::min(self.start() + len, self.end()))
    }
}

impl<I> ToSpan for I
where
    I: Input,
{
    #[inline(always)]
    fn to_span(&self) -> Span {
        Span::from(self.start()..self.end())
    }
}

/// An extension trait that add fn `split_to` to `Input`
pub trait SplitTo: Input<Split = Self> + Clone {
    /// Split the input into two at the given index.
    ///
    /// Afterwards self contains elements [at, len), and the returned `Self` contains elements [0, at).
    #[inline]
    fn split_to(&mut self, at: usize) -> Self {
        let (lhs, rhs) = self.clone().split_at(at);

        *self = rhs;

        lhs
    }
}

impl<I> SplitTo for I where I: Input<Split = Self> + Clone {}

/// An extension trait that add fn `split_off` to `Input`
pub trait SplitOff: Input<Split = Self> + Clone {
    /// Split the input into two at the given index.
    ///
    /// Afterwards self contains elements [0, at), and the returned `Self` contains elements [at, capacity).
    #[inline]
    fn split_off(&mut self, at: usize) -> Self {
        let (lhs, rhs) = self.clone().split_at(at);

        *self = lhs;

        rhs
    }
}

impl<I> SplitOff for I where I: Input<Split = Self> + Clone {}

/// Convert `Input` as `&[u8]`
pub trait AsBytes: Length {
    /// Convert the input type to a byte slice
    fn as_bytes(&self) -> &[u8];
}

impl AsBytes for &str {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        str::as_bytes(self)
    }
}

impl AsBytes for &[u8] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

impl Length for &[u8] {
    /// Returns the length of this item in bytes.
    #[inline(always)]
    fn len(&self) -> usize {
        self.as_bytes().len()
    }
}

impl<const N: usize> AsBytes for &[u8; N] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<const N: usize> Length for &[u8; N] {
    /// Returns the length of this item in bytes.
    #[inline(always)]
    fn len(&self) -> usize {
        self.as_bytes().len()
    }
}

/// Convert `Input` as `&str`
pub trait AsStr: Length {
    /// Convert the input type to a str slice
    fn as_str(&self) -> &str;
}

impl AsStr for &str {
    #[inline(always)]
    fn as_str(&self) -> &str {
        self
    }
}

impl Length for &str {
    /// Returns the length of this item in bytes.
    #[inline(always)]
    fn len(&self) -> usize {
        str::len(&self)
    }
}

impl AsStr for String {
    #[inline]
    fn as_str(&self) -> &str {
        String::as_str(&self)
    }
}

impl Length for String {
    /// Returns the length of this item in bytes.
    #[inline(always)]
    fn len(&self) -> usize {
        str::len(&self)
    }
}

/// An extension trait that add `start_with` fn to [`Input`]
pub trait StartWith<Needle>: Input {
    /// Returns match length if needle is a prefix of the `Input` or equal to the `Input`.
    fn start_with(&self, prefix: Needle) -> bool;
}

impl<I, Needle> StartWith<Needle> for I
where
    I: Input + AsBytes,
    Needle: AsBytes,
{
    #[inline]
    fn start_with(&self, prefix: Needle) -> bool {
        self.as_bytes().starts_with(prefix.as_bytes())
    }
}

/// An extension trait that add `find` fn to [`Input`]
pub trait Find<Needle>: Input {
    /// Returns the index of the first occurrence of the given needle.
    fn find(&self, prefix: Needle) -> Option<usize>;
}

impl<I, Needle> Find<Needle> for I
where
    I: Input + AsBytes,
    Needle: AsBytes,
{
    #[inline]
    fn find(&self, needle: Needle) -> Option<usize> {
        memmem::find(self.as_bytes(), needle.as_bytes())
    }
}

/// A segement of source stream that implement `Input` stream.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Source<S, E> {
    /// start offset in the source file/data.
    offset: usize,
    /// segment of source data.
    segment: S,
    /// error type marker.
    _marker: PhantomData<E>,
}

impl<S, E> Source<S, E> {
    /// Creates a new source code slice starting at `beginning`
    #[inline]
    pub fn new(segment: S) -> Self {
        Self {
            offset: 0,
            segment,
            _marker: PhantomData,
        }
    }

    /// Create a new source code slice starting at `offset`.
    #[inline]
    pub fn new_offset(offset: usize, segment: S) -> Self {
        Self {
            offset,
            segment,
            _marker: PhantomData,
        }
    }
}

impl<S, E> PartialEq for Source<S, E>
where
    S: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset
            && self.segment == other.segment
            && self._marker == other._marker
    }
}

impl<S, E> Eq for Source<S, E> where S: Eq {}

impl<S, E> Debug for Source<S, E>
where
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Source::offset({:?},{:?})", self.offset, self.segment)
    }
}

impl<S, E> Clone for Source<S, E>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            offset: self.offset.clone(),
            segment: self.segment.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, E> Length for Source<S, E>
where
    S: Length,
{
    #[inline]
    fn len(&self) -> usize {
        self.segment.len()
    }
}

impl<S, E> From<(usize, S)> for Source<S, E> {
    #[inline]
    fn from((offset, segment): (usize, S)) -> Self {
        Self {
            offset,
            segment,
            _marker: PhantomData,
        }
    }
}

impl<S, E> AsBytes for Source<S, E>
where
    S: AsBytes,
{
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.segment.as_bytes()
    }
}

impl<S, E> AsStr for Source<S, E>
where
    S: AsStr,
{
    #[inline]
    fn as_str(&self) -> &str {
        self.segment.as_str()
    }
}

#[cfg(feature = "chars")]
impl<'a, E> Input for Source<&'a str, E> {
    type Error = E;

    type Item = char;

    type Iter = std::str::Chars<'a>;

    type IterIndices = std::str::CharIndices<'a>;

    type Split = Self;

    #[inline]
    fn split_at(self, mid: usize) -> (Self, Self)
    where
        Self: Sized,
    {
        let (lhs, rhs) = self.segment.split_at(mid);

        ((self.offset, lhs).into(), (self.offset + mid, rhs).into())
    }

    #[inline]
    fn iter(&self) -> Self::Iter {
        self.segment.chars()
    }

    #[inline]
    fn iter_indices(&self) -> Self::IterIndices {
        self.segment.char_indices()
    }

    #[inline]
    fn start(&self) -> usize {
        self.offset
    }

    #[inline]
    fn end(&self) -> usize {
        self.offset + self.segment.len()
    }
}

#[cfg(feature = "bytes")]
impl<'a, E> Input for Source<&'a [u8], E> {
    type Error = E;

    type Item = u8;

    type Iter = Copied<Iter<'a, u8>>;

    type IterIndices = Enumerate<Self::Iter>;

    type Split = Self;

    #[inline]
    fn split_at(self, mid: usize) -> (Self, Self)
    where
        Self: Sized,
    {
        let (lhs, rhs) = self.segment.split_at(mid);

        ((self.offset, lhs).into(), (self.offset + mid, rhs).into())
    }

    #[inline]
    fn iter(&self) -> Self::Iter {
        self.segment.iter().copied()
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
        self.offset + self.segment.len()
    }
}

#[cfg(feature = "bytes")]
impl<'a, const N: usize, E> Input for Source<&'a [u8; N], E> {
    type Error = E;

    type Item = u8;

    type Iter = Copied<Iter<'a, u8>>;

    type IterIndices = Enumerate<Self::Iter>;

    type Split = Source<&'a [u8], E>;

    #[inline]
    fn split_at(self, mid: usize) -> (Self::Split, Self::Split)
    where
        Self: Sized,
    {
        let (lhs, rhs) = self.segment.split_at(mid);

        ((self.offset, lhs).into(), (self.offset + mid, rhs).into())
    }

    #[inline]
    fn iter(&self) -> Self::Iter {
        self.segment.iter().copied()
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
        self.offset + self.segment.len()
    }
}

/// Facade trait for [`Bytes`] stream.
#[cfg(feature = "bytes")]
pub trait BytesInput<E>:
    Input<Item = u8, Error = E>
    + Length
    + AsBytes
    + StartWith<&'static str>
    + Find<&'static str>
    + Clone
    + Debug
    + Eq
{
}

/// An `Input` stream wrapper for `&[u8]`
#[cfg(feature = "bytes")]
pub type Bytes<'a, E> = Source<&'a [u8], E>;

/// An `Input` stream wrapper for `&[u8;N]`
#[cfg(feature = "bytes")]
pub type BytesArray<'a, const N: usize, E> = Source<&'a [u8; N], E>;

#[cfg(feature = "bytes")]
impl<'a, E> BytesInput<E> for Bytes<'a, E> {}

/// Facade trait for [`Chars`] stream.
#[cfg(feature = "chars")]
pub trait CharsInput<E>:
    Input<Item = char, Error = E>
    + Length
    + AsBytes
    + AsStr
    + StartWith<&'static str>
    + Find<&'static str>
    + Clone
    + Debug
    + Eq
{
}

/// An `Input` stream wrapper for `&str`
#[cfg(feature = "chars")]
pub type Chars<'a, E> = Source<&'a str, E>;

#[cfg(feature = "chars")]
impl<'a, E> CharsInput<E> for Chars<'a, E> {}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use crate::{Find, Input, Length, Source, Span, SplitOff, SplitTo, StartWith, ToSpan};

    #[test]
    fn len() {
        fn assert_len<I>(input: I, expect: usize)
        where
            I: Input<Error = ()> + Length,
        {
            assert_eq!(input.len(), expect);
        }

        assert_len(Source::new("hello world"), 11);
        assert_len(Source::new(b"hello world".as_slice()), 11);

        assert_len(Source::from((10usize, "hello world")), 11);
        assert_len(Source::from((10usize, b"hello world".as_slice())), 11);
    }

    #[test]
    fn split_to() {
        fn assert_split_to<I>(mut input: I, mid: usize, lhs: I, rhs: I)
        where
            I: Input<Error = (), Split = I> + Clone + PartialEq + Debug,
        {
            let to = input.split_to(mid);

            assert_eq!(input, lhs);
            assert_eq!(to, rhs);
        }

        assert_split_to(
            Source::new(b"hello  world".as_slice()),
            5,
            (5, b"  world".as_slice()).into(),
            (0, b"hello".as_slice()).into(),
        );

        assert_split_to(
            Source::new("hello  world"),
            5,
            (5, "  world").into(),
            (0, "hello").into(),
        );
    }

    #[test]
    fn split_off() {
        fn assert_split_off<I>(mut input: I, mid: usize, lhs: I, rhs: I)
        where
            I: Input<Error = (), Split = I> + Clone + PartialEq + Debug,
        {
            let to = input.split_off(mid);

            assert_eq!(input, lhs);
            assert_eq!(to, rhs);
        }

        assert_split_off(
            Source::new(b"hello  world".as_slice()),
            5,
            (0, b"hello".as_slice()).into(),
            (5, b"  world".as_slice()).into(),
        );

        assert_split_off(
            Source::new("hello  world"),
            5,
            (0, "hello").into(),
            (5, "  world").into(),
        );
    }

    #[test]
    fn to_span() {
        fn assert_to_span<I>(input: I, expect: Span)
        where
            I: Input<Error = ()> + ToSpan,
        {
            assert_eq!(input.to_span(), expect);
        }

        assert_to_span(Source::new("hello  world"), Span::from(0..12));
        assert_to_span(Source::new(b"hello  world".as_slice()), Span::from(0..12));
    }

    #[test]
    fn to_span_with() {
        fn assert_to_span_with<I>(input: I, from: usize, expect: Span)
        where
            I: Input<Error = ()>,
        {
            assert_eq!(input.to_span_with(from), expect);
        }

        assert_to_span_with(Source::new("hello  world"), 20, Span::from(0..12));
        assert_to_span_with(
            Source::new(b"hello  world".as_slice()),
            10,
            Span::from(0..10),
        );
    }

    #[test]
    fn start_with() {
        fn assert_start_with<I, Needle>(input: I, prefix: Needle)
        where
            I: Input<Error = ()> + StartWith<Needle>,
        {
            assert!(input.start_with(prefix));
        }

        assert_start_with(Source::from((10, "hello")), "he");
        assert_start_with(Source::from((10, "hello")), b"he");

        assert_start_with(Source::from((10, b"hello")), "he");
        assert_start_with(Source::from((10, b"hello")), b"he");
    }

    #[test]
    fn find() {
        fn assert_find<I, Needle>(input: I, needle: Needle, offset: Option<usize>)
        where
            I: Input<Error = ()> + Find<Needle>,
        {
            assert_eq!(input.find(needle), offset);
        }

        assert_find(Source::from((10, "hello")), "lo", Some(3));
        assert_find(Source::from((10, "hello")), b"lo", Some(3));

        assert_find(Source::from((10, b"hello")), "lo", Some(3));
        assert_find(Source::from((10, b"hello")), b"lo", Some(3));

        assert_find(Source::from((10, "hello")), "loo", None);
        assert_find(Source::from((10, "hello")), b"lo;", None);
    }
}

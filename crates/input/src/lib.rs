//！ `input` represents the source data stream to be parsed

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

/// A source code stream must implement this trait.
pub trait Input: Clone {
    /// Error raised by combinators.
    type Error;
    /// Value yielded by this `Input`.
    type Item: Item;
    /// Iterator type returns by [`iter`](Input::iter).
    type Iter: Iterator<Item = Self::Item>;
    /// Iterator type returns by [`iter_indices`](Input::iter_indices).
    type IterIndices: Iterator<Item = (usize, Self::Item)>;

    // Returns the length of this input stream in bytes.
    fn len(&self) -> usize;

    /// Split the input into two at the given index.
    fn split_at(self, mid: usize) -> (Self, Self)
    where
        Self: Sized;

    /// Split the input into two at the given index.
    ///
    /// Afterwards self contains elements [at, len), and the returned `Self` contains elements [0, at).
    #[inline]
    fn split_to(&mut self, at: usize) -> Self {
        let (lhs, rhs) = self.clone().split_at(at);

        *self = rhs;

        lhs
    }

    /// Split the input into two at the given index.
    ///
    /// Afterwards self contains elements [0, at), and the returned `Self` contains elements [at, capacity).
    #[inline]
    fn split_off(&mut self, at: usize) -> Self {
        let (lhs, rhs) = self.clone().split_at(at);

        *self = lhs;

        rhs
    }

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

    /// Returns the `span` of the current input fragment in the entire source code.
    #[inline]
    fn to_span(&self) -> Span {
        Span::from(self.start()..self.end())
    }

    /// Returns the subspan of the current input fragment starting at `offset` in the entire source code.
    #[inline]
    fn to_span_from(&self, offset: usize) -> Span {
        Span::from(self.start()..cmp::min(self.start() + offset, self.end()))
    }
}

/// Convert `Input` as `&[u8]`
pub trait AsBytes {
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

/// Convert `Input` as `&str`
pub trait AsStr {
    /// Convert the input type to a str slice
    fn as_str(&self) -> &str;
}

impl AsStr for &str {
    #[inline(always)]
    fn as_str(&self) -> &str {
        self
    }
}

impl AsStr for String {
    #[inline]
    fn as_str(&self) -> &str {
        String::as_str(&self)
    }
}

/// An extension trait that add `start_with` fn to [`Input`]
pub trait StartWith<Needle> {
    /// Returns match length if needle is a prefix of the `Input` or equal to the `Input`.
    fn start_with(&self, prefix: Needle) -> bool;
}

impl<I, Needle> StartWith<Needle> for I
where
    I: AsBytes,
    Needle: AsBytes,
{
    #[inline]
    fn start_with(&self, prefix: Needle) -> bool {
        self.as_bytes().starts_with(prefix.as_bytes())
    }
}

/// An extension trait that add `find` fn to [`Input`]
pub trait Find<Needle> {
    /// Returns the index of the first occurrence of the given needle.
    fn find(&self, prefix: Needle) -> Option<usize>;
}

impl<I, Needle> Find<Needle> for I
where
    I: AsBytes,
    Needle: AsBytes,
{
    #[inline]
    fn find(&self, needle: Needle) -> Option<usize> {
        memmem::find(self.as_bytes(), needle.as_bytes())
    }
}

/// A segement of source stream that implement `Input` stream.
pub struct Source<S, E> {
    /// start offset in the source file/data.
    offset: usize,
    /// sgement of source data.
    sgement: S,
    /// error type marker.
    _marker: PhantomData<E>,
}

impl<S, E> PartialEq for Source<S, E>
where
    S: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset
            && self.sgement == other.sgement
            && self._marker == other._marker
    }
}

impl<S, E> Eq for Source<S, E> where S: Eq {}

impl<S, E> Debug for Source<S, E>
where
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Source")
            .field("offset", &self.offset)
            .field("sgement", &self.sgement)
            .field("_marker", &self._marker)
            .finish()
    }
}

impl<S, E> Clone for Source<S, E>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            offset: self.offset.clone(),
            sgement: self.sgement.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, E> From<S> for Source<S, E> {
    #[inline]
    fn from(value: S) -> Self {
        Self {
            offset: 0,
            sgement: value,
            _marker: PhantomData,
        }
    }
}

impl<S, E> From<(usize, S)> for Source<S, E> {
    #[inline]
    fn from((offset, value): (usize, S)) -> Self {
        Self {
            offset,
            sgement: value,
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
        self.sgement.as_bytes()
    }
}

impl<S, E> AsStr for Source<S, E>
where
    S: AsStr,
{
    #[inline]
    fn as_str(&self) -> &str {
        self.sgement.as_str()
    }
}

#[cfg(feature = "chars")]
impl<'a, E> Input for Source<&'a str, E> {
    type Error = E;

    type Item = char;

    type Iter = std::str::Chars<'a>;

    type IterIndices = std::str::CharIndices<'a>;

    #[inline]
    fn len(&self) -> usize {
        self.sgement.len()
    }

    #[inline]
    fn split_at(self, mid: usize) -> (Self, Self)
    where
        Self: Sized,
    {
        let (lhs, rhs) = self.sgement.split_at(mid);

        ((self.offset, lhs).into(), (self.offset + mid, rhs).into())
    }

    #[inline]
    fn iter(&self) -> Self::Iter {
        self.sgement.chars()
    }

    #[inline]
    fn iter_indices(&self) -> Self::IterIndices {
        self.sgement.char_indices()
    }

    #[inline]
    fn start(&self) -> usize {
        self.offset
    }

    #[inline]
    fn end(&self) -> usize {
        self.offset + self.sgement.len()
    }
}

#[cfg(feature = "bytes")]
impl<'a, E> Input for Source<&'a [u8], E> {
    type Error = E;

    type Item = u8;

    type Iter = Copied<Iter<'a, u8>>;

    type IterIndices = Enumerate<Self::Iter>;

    #[inline]
    fn len(&self) -> usize {
        self.sgement.len()
    }

    #[inline]
    fn split_at(self, mid: usize) -> (Self, Self)
    where
        Self: Sized,
    {
        let (lhs, rhs) = self.sgement.split_at(mid);

        ((self.offset, lhs).into(), (self.offset + mid, rhs).into())
    }

    #[inline]
    fn iter(&self) -> Self::Iter {
        self.sgement.iter().copied()
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
        self.offset + self.sgement.len()
    }
}

/// Facade trait for [`Bytes`] stream.
#[cfg(feature = "bytes")]
pub trait BytesInput<E>:
    Input<Item = u8, Error = E>
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

#[cfg(feature = "bytes")]
impl<'a, E> BytesInput<E> for Bytes<'a, E> {}

/// Facade trait for [`Chars`] stream.
#[cfg(feature = "chars")]
pub trait CharsInput<E>:
    Input<Item = char, Error = E>
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

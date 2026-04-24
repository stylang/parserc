use std::{
    marker::PhantomData,
    option,
    slice::{Iter, IterMut},
};

use sourceinput::{Input, Span, SplitTo, ToSpan};

use crate::{Kind, ParseError, Parser, combinators::next};

struct SyntaxParser<S, T>(PhantomData<S>, PhantomData<T>);

impl<I, T> Parser<I> for SyntaxParser<I, T>
where
    I: Input,
    I::Error: ParseError,
    T: Syntax<I>,
{
    type Output = T;

    #[inline]
    fn parse(self, input: &mut I) -> Result<Self::Output, I::Error> {
        T::parse(input)
    }
}

/// A node of `concrete syntax tree` should implement this trait.
pub trait Syntax<I>: Sized
where
    I: Input,
    I::Error: ParseError,
{
    /// Parse the next node use this `cst` parser.
    fn parse(input: &mut I) -> Result<Self, I::Error>;

    /// Return the span of the CST node in the source code.
    fn to_span(&self) -> Span;

    /// Generate a Parser wrapper for the CST node.
    #[inline(always)]
    fn into_parser() -> impl Parser<I, Output = Self> {
        SyntaxParser(PhantomData, PhantomData)
    }
}

/// An extension trait for [`Input`] to provide the `parse` method.
pub trait SyntaxExt: Input {
    /// Parse the next output using a [`Syntax`] parser.
    #[inline]
    fn parse<S>(&mut self) -> Result<S, Self::Error>
    where
        Self: Sized,
        Self::Error: ParseError,
        S: Syntax<Self>,
    {
        S::parse(self)
    }
}

impl<I> SyntaxExt for I where I: Input {}

impl<T, I> Syntax<I> for PhantomData<T>
where
    I: Input,
    I::Error: ParseError,
{
    #[inline(always)]
    fn parse(_input: &mut I) -> Result<Self, I::Error> {
        Ok(Self::default())
    }

    #[inline(always)]
    fn to_span(&self) -> Span {
        Span::None
    }
}

impl<T, I> Syntax<I> for Option<T>
where
    T: Syntax<I>,
    I: Input + Clone,
    I::Error: ParseError,
{
    #[inline(always)]
    fn parse(input: &mut I) -> Result<Self, I::Error> {
        T::into_parser().ok().parse(input)
    }

    #[inline]
    fn to_span(&self) -> Span {
        self.as_ref().map_or(Span::None, |value| value.to_span())
    }
}

impl<T, I> Syntax<I> for Box<T>
where
    T: Syntax<I>,
    I: Input + Clone,
    I::Error: ParseError,
{
    #[inline(always)]
    fn parse(input: &mut I) -> Result<Self, I::Error> {
        T::into_parser().boxed().parse(input)
    }

    #[inline]
    fn to_span(&self) -> Span {
        self.as_ref().to_span()
    }
}

impl<T, I> Syntax<I> for Vec<T>
where
    T: Syntax<I>,
    I: Input + Clone,
    I::Error: ParseError,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, I::Error> {
        let mut elms = vec![];
        loop {
            let elm = T::into_parser().ok().parse(input)?;

            let Some(elm) = elm else {
                break;
            };

            elms.push(elm);
        }

        Ok(elms)
    }

    #[inline]
    fn to_span(&self) -> Span {
        let first = self.first().map_or(Span::None, |v| v.to_span());
        let last = self.last().map_or(Span::None, |v| v.to_span());

        first.join(&last)
    }
}

/// A sytanx node to match a char.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Char<I, const C: char>(pub I)
where
    I: Input<Item = char>;

impl<I, const C: char> Syntax<I> for Char<I, C>
where
    I: Input<Item = char> + SplitTo + ToSpan,
    I::Error: ParseError,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as Input>::Error> {
        next(C).map(|input| Self(input)).parse(input)
    }

    #[inline]
    fn to_span(&self) -> Span {
        self.0.to_span()
    }
}

/// A sytanx node to match a byte.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Byte<I, const C: u8>(pub I)
where
    I: Input<Item = u8>;

impl<I, const C: u8> Syntax<I> for Byte<I, C>
where
    I: Input<Item = u8> + SplitTo,
    I::Error: ParseError,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as Input>::Error> {
        next(C).map(|input| Self(input)).parse(input)
    }

    #[inline]
    fn to_span(&self) -> Span {
        self.0.to_span()
    }
}

/// A short syntax for grouping token that surrounds a syntax body.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Delimiter<Start, End, Body> {
    /// Syntax start token.
    pub start: Start,
    /// Syntax end token.
    pub end: End,
    /// Syntax body.
    pub body: Body,
}

impl<I, Start, End, Body> Syntax<I> for Delimiter<Start, End, Body>
where
    I: Input + Clone,
    I::Error: ParseError,
    Start: Syntax<I>,
    End: Syntax<I>,
    Body: Syntax<I>,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, I::Error> {
        let start = Start::parse(input)?;

        let body = Body::into_parser().parse(input)?;

        let end = End::into_parser().fatal().parse(input)?;

        Ok(Self { start, body, end })
    }

    #[inline]
    fn to_span(&self) -> Span {
        self.start.to_span() + self.end.to_span()
    }
}

/// Parse up to `N` CST child nodes of type `T`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Limits<T, const N: usize>(pub Vec<T>);

impl<I, T, const N: usize> Syntax<I> for Limits<T, N>
where
    I: Input + Clone,
    I::Error: ParseError,
    T: Syntax<I>,
{
    fn parse(input: &mut I) -> Result<Self, <I as Input>::Error> {
        assert!(N > 0);

        let mut children = vec![];

        while let Some(v) = T::into_parser().ok().parse(input)? {
            children.push(v);

            // N > 0
            if children.len() == N {
                break;
            }
        }

        Ok(Self(children))
    }

    #[inline(always)]
    fn to_span(&self) -> Span {
        self.0.to_span()
    }
}

/// Parse at least `N` CST child nodes of type `T`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AtLeast<T, const N: usize>(pub Vec<T>);

impl<I, T, const N: usize> Syntax<I> for AtLeast<T, N>
where
    I: Input + Clone,
    I::Error: ParseError,
    T: Syntax<I>,
{
    fn parse(input: &mut I) -> Result<Self, <I as Input>::Error> {
        assert!(N > 0);

        let mut children = vec![];

        while let Some(v) = T::into_parser().ok().parse(input)? {
            children.push(v);
        }

        if children.len() < N {
            Err(Kind::AtLeast(crate::ControlFlow::Recovable, children.to_span()).into())
        } else {
            Ok(Self(children))
        }
    }

    #[inline(always)]
    fn to_span(&self) -> Span {
        self.0.to_span()
    }
}

/// Parse CST child nodes; the count must be in the range [Lower, Upper).
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Range<T, const LOWER: usize, const UPPER: usize>(pub Vec<T>);

impl<I, T, const LOWER: usize, const UPPER: usize> Syntax<I> for Range<T, LOWER, UPPER>
where
    I: Input + Clone,
    I::Error: ParseError,
    T: Syntax<I>,
{
    fn parse(input: &mut I) -> Result<Self, <I as Input>::Error> {
        assert!(!(LOWER > UPPER));

        let mut children = vec![];

        while let Some(v) = T::into_parser().ok().parse(input)? {
            if !(children.len() < UPPER) {
                break;
            }

            children.push(v);
        }

        if children.len() < LOWER {
            Err(Kind::Range(crate::ControlFlow::Recovable, children.to_span()).into())
        } else {
            Ok(Self(children))
        }
    }

    #[inline]
    fn to_span(&self) -> Span {
        self.0.to_span()
    }
}

/// A punctuated sequence of syntax tree nodes of type T separated by punctuation of type P.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Punctuated<T, P> {
    /// (T,P) pairs
    pub pairs: Vec<(T, P)>,
    /// individual tail `T`
    pub tail: Option<Box<T>>,
}

impl<T, P> Punctuated<T, P> {
    /// Returns an iterator over [`Punctuated`] of type T.
    #[inline]
    pub fn iter(&self) -> PunctIter<'_, T, P> {
        PunctIter {
            iter: self.pairs.iter(),
            tail: self.tail.as_ref().map(Box::as_ref).into_iter(),
        }
    }

    /// Returns an mutable iterator over [`Punctuated`] of type T.
    #[inline]
    pub fn iter_mut(&mut self) -> PunctIterMut<'_, T, P> {
        PunctIterMut {
            iter: self.pairs.iter_mut(),
            tail: self.tail.as_mut().map(Box::as_mut).into_iter(),
        }
    }

    /// returns the sequence length.
    #[inline]
    pub fn len(&self) -> usize {
        self.pairs.len() + self.tail.as_ref().map_or(0, |_| 1)
    }

    /// Returns true if the punctuated sequence length is 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a, T, P> IntoIterator for &'a Punctuated<T, P> {
    type Item = &'a T;

    type IntoIter = PunctIter<'a, T, P>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        Punctuated::iter(self)
    }
}

impl<'a, T, P> IntoIterator for &'a mut Punctuated<T, P> {
    type Item = &'a mut T;

    type IntoIter = PunctIterMut<'a, T, P>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        Punctuated::iter_mut(self)
    }
}

/// Iterator over [`Punctuated`] of type T
pub struct PunctIter<'a, T, P> {
    iter: Iter<'a, (T, P)>,
    tail: option::IntoIter<&'a T>,
}

impl<'a, T, P> Iterator for PunctIter<'a, T, P> {
    type Item = &'a T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let Some((next, _)) = self.iter.next() else {
            return self.tail.next();
        };

        Some(next)
    }
}

/// Iterator over [`Punctuated`] of type T
pub struct PunctIterMut<'a, T, P> {
    iter: IterMut<'a, (T, P)>,
    tail: option::IntoIter<&'a mut T>,
}

impl<'a, T, P> Iterator for PunctIterMut<'a, T, P> {
    type Item = &'a mut T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let Some((next, _)) = self.iter.next() else {
            return self.tail.next();
        };

        Some(next)
    }
}

impl<T, P, I> Syntax<I> for Punctuated<T, P>
where
    I: Input + Clone,
    I::Error: ParseError,
    T: Syntax<I>,
    P: Syntax<I>,
{
    fn parse(input: &mut I) -> Result<Self, I::Error> {
        let mut pairs = vec![];

        loop {
            let t = T::into_parser().ok().parse(input)?;

            let Some(t) = t else {
                return Ok(Self { pairs, tail: None });
            };

            let p = P::into_parser().ok().parse(input)?;

            let Some(p) = p else {
                return Ok(Self {
                    pairs,
                    tail: Some(Box::new(t)),
                });
            };

            pairs.push((t, p));
        }
    }

    #[inline]
    fn to_span(&self) -> Span {
        self.pairs.to_span() + self.tail.to_span()
    }
}

/// When merging two abstract syntax trees,
/// it first attempts to match the left subtree;
/// if unsuccessful, it proceeds to match the right subtree.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Or<F, S> {
    First(F),
    Second(S),
}

impl<I, F, S> Syntax<I> for Or<F, S>
where
    I: Input + Clone,
    I::Error: ParseError,
    F: Syntax<I>,
    S: Syntax<I>,
{
    fn parse(input: &mut I) -> Result<Self, I::Error> {
        let Some(first) = F::into_parser().ok().parse(input)? else {
            let s = S::parse(input)?;

            return Ok(Self::Second(s));
        };

        Ok(Self::First(first))
    }

    #[inline]
    fn to_span(&self) -> Span {
        match self {
            Or::First(v) => v.to_span(),
            Or::Second(v) => v.to_span(),
        }
    }
}

// implement Syntax for tuple (T1,T2,...) where T1: Syntax, T2: Syntax, ...
parserc_derive::derive_tuple_syntax!(16);

#[cfg(feature = "derive")]
pub use parserc_derive::Syntax;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Kind;

    type Bytes<'a> = sourceinput::Bytes<'a, Kind>;

    #[test]
    fn test_range() {
        assert_eq!(
            Range::<Byte<_, b'c'>, 0, 0>::parse(&mut Bytes::new(b"cccccc")),
            Ok(Range(vec![]))
        );

        assert_eq!(
            Range::<Byte<_, b'c'>, 0, 4>::parse(&mut Bytes::new(b"cccccc")),
            Ok(Range(vec![
                Byte(Bytes::from((0, b"c".as_slice()))),
                Byte(Bytes::from((1, b"c".as_slice()))),
                Byte(Bytes::from((2, b"c".as_slice()))),
                Byte(Bytes::from((3, b"c".as_slice())))
            ]))
        );

        assert_eq!(
            Range::<Byte<_, b'c'>, 1, 4>::parse(&mut Bytes::new(b"cccccc")),
            Ok(Range(vec![
                Byte(Bytes::from((0, b"c".as_slice()))),
                Byte(Bytes::from((1, b"c".as_slice()))),
                Byte(Bytes::from((2, b"c".as_slice()))),
                Byte(Bytes::from((3, b"c".as_slice())))
            ]))
        );

        assert_eq!(
            Range::<Byte<_, b'c'>, 3, 4>::parse(&mut Bytes::new(b"cc")),
            Err(Kind::Range(crate::ControlFlow::Recovable, Span::from(0..2)))
        );
    }

    #[test]
    fn test_limits() {
        assert_eq!(
            Limits::<Byte<_, b'c'>, 4>::parse(&mut Bytes::new(b"cccccc")),
            Ok(Limits(vec![
                Byte(Bytes::from((0, b"c".as_slice()))),
                Byte(Bytes::from((1, b"c".as_slice()))),
                Byte(Bytes::from((2, b"c".as_slice()))),
                Byte(Bytes::from((3, b"c".as_slice())))
            ]))
        );

        assert_eq!(
            Limits::<Byte<_, b'c'>, 4>::parse(&mut Bytes::new(b"cc")),
            Ok(Limits(vec![
                Byte(Bytes::from((0, b"c".as_slice()))),
                Byte(Bytes::from((1, b"c".as_slice()))),
            ]))
        );

        assert_eq!(
            Limits::<Byte<_, b'c'>, 4>::parse(&mut Bytes::new(b"")),
            Ok(Limits(vec![]))
        );
    }

    #[test]
    fn test_at_least() {
        assert_eq!(
            AtLeast::<Byte<_, b'c'>, 2>::parse(&mut Bytes::new(b"cccc")),
            Ok(AtLeast(vec![
                Byte(Bytes::from((0, b"c".as_slice()))),
                Byte(Bytes::from((1, b"c".as_slice()))),
                Byte(Bytes::from((2, b"c".as_slice()))),
                Byte(Bytes::from((3, b"c".as_slice())))
            ]))
        );

        assert_eq!(
            AtLeast::<Byte<_, b'c'>, 4>::parse(&mut Bytes::new(b"cc")),
            Err(Kind::AtLeast(
                crate::ControlFlow::Recovable,
                Span::from(0..2)
            ))
        );
    }
}

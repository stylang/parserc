use std::marker::PhantomData;

use sourceinput::{Input, Span, SplitTo};

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
    I: Input<Item = char> + SplitTo,
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
        assert!(LOWER > 0);
        assert!(LOWER < UPPER);

        let mut children = vec![];

        while let Some(v) = T::into_parser().ok().parse(input)? {
            if children.len() == UPPER {
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

//! Abstract syntax tree (AST) support: the [`Syntax`] trait and ready-made node
//! combinators.
//!
//! Implement [`Syntax`] for AST structs and enums (or derive it with
//! `#[derive(Syntax)]`) to parse them straight from an [`Input`], then compose
//! nodes with [`Delimiter`], [`Punctuated`], [`Or`], [`LimitsTo`] and friends.

use std::{fmt::Debug, marker::PhantomData};

use crate::{ControlFlow, Kind, Span, next};
use crate::{input::Input, parser::Parser};

/// Extension trait parsing a [`Syntax`] value from an [`Input`].
pub trait SyntaxInput: Input {
    /// Parses `S` from the current position of `self`.
    #[inline]
    fn parse<S>(&mut self) -> Result<S, Self::Error>
    where
        Self: Sized,
        S: Syntax<Self>,
    {
        S::parse(self)
    }
}

impl<I> SyntaxInput for I where I: Input {}

/// A node of the abstract syntax tree.
///
/// Implement it for AST structs/enums (or use the `Syntax` derive macro) to
/// parse them from an [`Input`].
pub trait Syntax<I>: Sized
where
    I: Input,
{
    /// Parses `input` at its current position and builds a new node.
    fn parse(input: &mut I) -> Result<Self, I::Error>;

    /// Returns the source region covered by this node.
    fn to_span(&self) -> Span;

    /// Returns a [`Parser`] that parses `Self`.
    fn into_parser() -> impl Parser<I, Output = Self> {
        SyntaxParser(Default::default(), Default::default())
    }
}

struct SyntaxParser<S, T>(PhantomData<S>, PhantomData<T>);

impl<I, T> Parser<I> for SyntaxParser<I, T>
where
    I: Input,
    T: Syntax<I>,
{
    type Output = T;

    #[inline]
    fn parse(self, input: &mut I) -> Result<Self::Output, I::Error> {
        T::parse(input)
    }
}

impl<T, I> Syntax<I> for PhantomData<T>
where
    I: Input,
{
    #[inline]
    fn parse(_input: &mut I) -> Result<Self, I::Error> {
        Ok(Self::default())
    }

    #[inline]
    fn to_span(&self) -> Span {
        Span::None
    }
}

impl<T, I> Syntax<I> for Option<T>
where
    T: Syntax<I>,
    I: Input + Clone,
{
    #[inline]
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
{
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
{
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

        first.union(&last)
    }
}

/// A syntax node matching a single [`char`] literal `C`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Char<I, const C: char>(pub I)
where
    I: Input;

impl<I, const C: char> Syntax<I> for Char<I, C>
where
    I: Input<Item = char>,
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

/// A syntax node matching a single [`u8`] literal `C`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Byte<I, const C: u8>(pub I)
where
    I: Input;

impl<I, const C: u8> Syntax<I> for Byte<I, C>
where
    I: Input<Item = u8>,
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

/// Groups a `Body` between a `Start` and an `End` node.
///
/// Once `Start` matches, a missing `End` is promoted to a
/// [`ControlFlow::Fatal`] error: an opened delimiter must be closed.
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
    Start: Syntax<I>,
    End: Syntax<I>,
    Body: Syntax<I>,
{
    fn parse(input: &mut I) -> Result<Self, I::Error> {
        let start = Start::parse(input)?;

        let body = Body::into_parser().parse(input)?;

        let end = End::into_parser().fatal().parse(input)?;

        Ok(Self { start, body, end })
    }

    #[inline]
    fn to_span(&self) -> Span {
        let start = self.start.to_span();
        let end = self.end.to_span();

        start.union(&end)
    }
}

/// Accepts `T` only when its span is at most `N` units long.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LimitsTo<T, const N: usize>(pub T);

impl<I, T, const N: usize> Syntax<I> for LimitsTo<T, N>
where
    I: Input,
    T: Syntax<I>,
{
    fn parse(input: &mut I) -> Result<Self, <I as Input>::Error> {
        let start = input.to_span();

        let t = T::parse(input)?;

        let span = t.to_span();

        let len = match span {
            sourcespan::Span::None => 0,
            sourcespan::Span::Range(range) => range.len(),
            sourcespan::Span::RangeTo(range_to) => range_to.end,
            _ => {
                return Err(Kind::LimitsTo(ControlFlow::Recovable, start).into());
            }
        };

        if len > N {
            return Err(Kind::LimitsTo(ControlFlow::Recovable, start).into());
        }

        Ok(Self(t))
    }

    fn to_span(&self) -> Span {
        self.0.to_span()
    }
}

/// Accepts `T` only when its span length lies in `LOWER..HIGHER`
/// (i.e. `LOWER <= length < HIGHER`).
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Limits<T, const LOWER: usize, const HIGHER: usize>(pub T);

impl<I, T, const LOWER: usize, const HIGHER: usize> Syntax<I> for Limits<T, LOWER, HIGHER>
where
    I: Input,
    T: Syntax<I>,
{
    fn parse(input: &mut I) -> Result<Self, <I as Input>::Error> {
        let start = input.to_span();

        let t = T::parse(input)?;

        let span = t.to_span();

        let len = match span {
            sourcespan::Span::None => 0,
            sourcespan::Span::Range(range) => range.len(),
            sourcespan::Span::RangeTo(range_to) => range_to.end,
            _ => {
                return Err(Kind::Limits(ControlFlow::Recovable, start).into());
            }
        };

        if len < LOWER || !(len < HIGHER) {
            return Err(Kind::Limits(ControlFlow::Recovable, start).into());
        }

        Ok(Self(t))
    }

    fn to_span(&self) -> Span {
        self.0.to_span()
    }
}

/// Accepts `T` only when its span is at least `LOWER` units long.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LimitsFrom<T, const LOWER: usize>(pub T);

impl<I, T, const LOWER: usize> Syntax<I> for LimitsFrom<T, LOWER>
where
    I: Input,
    T: Syntax<I>,
{
    fn parse(input: &mut I) -> Result<Self, <I as Input>::Error> {
        let start = input.to_span();

        let t = T::parse(input)?;

        let span = t.to_span();

        let len = match span {
            sourcespan::Span::None => 0,
            sourcespan::Span::Range(range) => range.len(),
            sourcespan::Span::RangeTo(range_to) => range_to.end,
            _ => {
                return Err(Kind::LimitsFrom(ControlFlow::Recovable, start).into());
            }
        };

        if len < LOWER {
            return Err(Kind::LimitsFrom(ControlFlow::Recovable, start).into());
        }

        Ok(Self(t))
    }

    fn to_span(&self) -> Span {
        self.0.to_span()
    }
}

/// A punctuated sequence of syntax tree nodes of type T separated by punctuation of type P.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Punctuated<T, P> {
    /// `(T, P)` pairs of item and separator.
    pub pairs: Vec<(T, P)>,
    /// Trailing `T` not followed by a separator.
    pub tail: Option<Box<T>>,
}

impl<T, P> Punctuated<T, P> {
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

impl<T, P, I> Syntax<I> for Punctuated<T, P>
where
    T: Syntax<I>,
    P: Syntax<I>,
    I: Input + Clone,
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
        self.pairs.to_span().union(&self.tail.to_span())
    }
}

/// Ordered choice between two syntax nodes: tries `F` first and falls back to `S`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Or<F, S> {
    /// Parsed by the `F` alternative.
    First(F),
    /// Parsed by the `S` alternative.
    Second(S),
}

impl<I, F, S> Syntax<I> for Or<F, S>
where
    I: Input + Clone,
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

pub use parserc_derive::Syntax;

// The built-in `Syntax` nodes are exercised against the `TokenStream` inputs.
#[cfg(all(test, feature = "input"))]
mod tests {
    use std::marker::PhantomData;

    use super::*;
    use crate::ParseError;
    use crate::input::{AsStr, bytes, chars};

    type Chars = chars::TokenStream<'static>;
    type Bytes = bytes::TokenStream<'static>;

    type A = Char<Chars, 'a'>;
    type B = Char<Chars, 'b'>;
    type Comma = Char<Chars, ','>;
    type Run = Vec<A>;
    type Parens = Delimiter<Char<Chars, '('>, Char<Chars, ')'>, Run>;

    #[test]
    fn char_node_matches_a_char_literal() {
        let mut input = Chars::from("ab");

        let node = A::parse(&mut input).unwrap();

        assert_eq!(node.0.as_str(), "a");
        assert_eq!(node.to_span(), Span::Range(0..1));
        assert_eq!(input.as_str(), "b");
    }

    #[test]
    fn byte_node_matches_a_byte_literal() {
        let mut input = Bytes::from("ab");

        let node = Byte::<Bytes, b'a'>::parse(&mut input).unwrap();

        assert_eq!(node.0.as_str(), "a");
        assert_eq!(input.as_str(), "b");
    }

    #[test]
    fn syntax_input_parse_delegates_to_the_trait() {
        let mut input = Chars::from("ab");

        let node = input.parse::<A>().unwrap();

        assert_eq!(node.to_span(), Span::Range(0..1));
        assert_eq!(input.as_str(), "b");
    }

    #[test]
    fn into_parser_wraps_a_syntax_type_in_a_parser() {
        let mut input = Chars::from("ab");

        let node = A::into_parser().parse(&mut input).unwrap();

        assert_eq!(node.0.as_str(), "a");
    }

    #[test]
    fn phantom_data_consumes_nothing() {
        let mut input = Chars::from("ab");

        let node = <PhantomData<()> as Syntax<Chars>>::parse(&mut input).unwrap();

        assert_eq!(
            <PhantomData<()> as Syntax<Chars>>::to_span(&node),
            Span::None
        );
        assert_eq!(input.as_str(), "ab");
    }

    #[test]
    fn option_node_is_zero_or_one() {
        let mut input = Chars::from("ba");

        let node = Option::<A>::parse(&mut input).unwrap();

        assert!(node.is_none());
        assert_eq!(node.to_span(), Span::None);
        // The failed attempt consumes nothing.
        assert_eq!(input.as_str(), "ba");

        let mut input = Chars::from("ab");
        let node = Option::<A>::parse(&mut input).unwrap();

        assert!(node.is_some());
        assert_eq!(node.to_span(), Span::Range(0..1));
    }

    #[test]
    fn box_node_parses_the_inner_value() {
        let mut input = Chars::from("ab");

        let node = Box::<A>::parse(&mut input).unwrap();

        assert_eq!((*node).0.as_str(), "a");
        assert_eq!(node.to_span(), Span::Range(0..1));
    }

    #[test]
    fn vec_node_repeats_until_failure() {
        let mut input = Chars::from("aab");

        let nodes = Run::parse(&mut input).unwrap();

        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes.to_span(), Span::Range(0..2));
        assert_eq!(input.as_str(), "b");

        let mut input = Chars::from("ba");
        let nodes = Run::parse(&mut input).unwrap();

        assert!(nodes.is_empty());
        assert_eq!(nodes.to_span(), Span::None);
        assert_eq!(input.as_str(), "ba");
    }

    #[test]
    fn delimiter_parses_start_body_and_end() {
        let mut input = Chars::from("(aa)");

        let node = Parens::parse(&mut input).unwrap();

        assert_eq!(node.to_span(), Span::Range(0..4));
        assert!(input.is_empty());
    }

    #[test]
    fn delimiter_requires_a_closing_token() {
        let mut input = Chars::from("(aa");

        let err = Parens::parse(&mut input).unwrap_err();

        // A missing `end` is promoted to a fatal error.
        assert!(err.is_fatal());
    }

    #[test]
    fn delimiter_start_failure_stays_recoverable() {
        let mut input = Chars::from("aa)");

        let err = Parens::parse(&mut input).unwrap_err();

        assert!(!err.is_fatal());
        assert_eq!(input.as_str(), "aa)");
    }

    #[test]
    fn limits_to_caps_the_span_length() {
        assert!(LimitsTo::<Run, 2>::parse(&mut Chars::from("aa")).is_ok());
        assert!(LimitsTo::<Run, 2>::parse(&mut Chars::from("")).is_ok());

        let mut input = Chars::from("aaa");
        let err = LimitsTo::<Run, 2>::parse(&mut input).unwrap_err();

        assert_eq!(
            err,
            Kind::LimitsTo(ControlFlow::Recovable, Span::Range(0..3))
        );
    }

    #[test]
    fn limits_enforces_a_length_window() {
        // Accepted when `LOWER <= length < HIGHER`.
        assert!(Limits::<Run, 1, 3>::parse(&mut Chars::from("a")).is_ok());
        assert!(Limits::<Run, 1, 3>::parse(&mut Chars::from("aa")).is_ok());
        assert!(Limits::<Run, 1, 3>::parse(&mut Chars::from("")).is_err());

        let mut input = Chars::from("aaa");
        let err = Limits::<Run, 1, 3>::parse(&mut input).unwrap_err();

        assert_eq!(err, Kind::Limits(ControlFlow::Recovable, Span::Range(0..3)));
    }

    #[test]
    fn limits_from_enforces_a_minimum_length() {
        assert!(LimitsFrom::<Run, 2>::parse(&mut Chars::from("aa")).is_ok());

        let mut input = Chars::from("a");
        let err = LimitsFrom::<Run, 2>::parse(&mut input).unwrap_err();

        assert_eq!(
            err,
            Kind::LimitsFrom(ControlFlow::Recovable, Span::Range(0..1))
        );
    }

    #[test]
    fn punctuated_collects_pairs_and_an_optional_tail() {
        let mut input = Chars::from("a,a,a");

        let nodes = Punctuated::<A, Comma>::parse(&mut input).unwrap();

        assert_eq!(nodes.len(), 3);
        assert!(!nodes.is_empty());
        assert_eq!(nodes.pairs.len(), 2);
        assert!(nodes.tail.is_some());
        assert_eq!(nodes.to_span(), Span::Range(0..5));

        let mut input = Chars::from("");
        let nodes = Punctuated::<A, Comma>::parse(&mut input).unwrap();

        assert!(nodes.is_empty());
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn or_node_tries_the_first_alternative_first() {
        let mut input = Chars::from("a");
        let node = Or::<A, B>::parse(&mut input).unwrap();

        assert_eq!(node.to_span(), Span::Range(0..1));
        assert!(matches!(node, Or::First(_)));

        let mut input = Chars::from("b");
        let node = Or::<A, B>::parse(&mut input).unwrap();

        assert!(matches!(node, Or::Second(_)));

        let mut input = Chars::from("c");
        let err = Or::<A, B>::parse(&mut input).unwrap_err();

        assert!(!err.is_fatal());
    }

    #[test]
    fn tuple_nodes_parse_in_sequence_without_rewinding() {
        type Pair = (A, B);

        let mut input = Chars::from("ab");

        let node = Pair::parse(&mut input).unwrap();

        assert_eq!(node.to_span(), Span::Range(0..2));
        assert!(input.is_empty());

        let mut input = Chars::from("ax");
        let err = Pair::parse(&mut input).unwrap_err();

        assert!(!err.is_fatal());
        // Tuple fields fail in place: the first `a` stays consumed.
        assert_eq!(input.as_str(), "x");
    }
}

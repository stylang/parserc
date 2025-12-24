//! Parser combinators for tokenizer/lexer.

use std::{fmt::Debug, ops::Range};

use crate::{
    Length, Span,
    errors::{ControlFlow, Kind},
    input::{Find, Input, Item, StartWith},
    parser::Parser,
};

/// A parser match next item, otherwise raise an error.
#[inline]
pub fn next<I>(item: I::Item) -> impl Parser<I, Output = I>
where
    I: Input,
{
    move |input: &mut I| {
        if let Some(next) = input.iter().next() {
            if next == item {
                return Ok(input.split_to(item.len()));
            }

            Err((Kind::Next(ControlFlow::Recovable, input.to_span_at(1))).into())
        } else {
            Err((Kind::Next(ControlFlow::Incomplete, input.to_span())).into())
        }
    }
}

/// A parser match next item by `F`, otherwise raise an error.
#[inline]
pub fn next_if<I, F>(f: F) -> impl Parser<I, Output = I>
where
    I: Input,
    F: FnOnce(I::Item) -> bool,
{
    move |input: &mut I| {
        if let Some(next) = input.iter().next() {
            if f(next) {
                return Ok(input.split_to(next.len()));
            }

            Err((Kind::NextIf(ControlFlow::Recovable, input.to_span_at(1))).into())
        } else {
            Err((Kind::NextIf(ControlFlow::Incomplete, input.to_span_at(1))).into())
        }
    }
}

/// Recogonize a keyword
#[inline]
pub fn keyword<KW, I>(keyword: KW) -> impl Parser<I, Output = I>
where
    I: Input + StartWith<KW> + Clone,
    KW: Debug + Clone + Length,
{
    move |input: &mut I| {
        if let Some(len) = input.starts_with(keyword.clone()) {
            Ok(input.split_to(len))
        } else {
            Err((Kind::Keyword(ControlFlow::Recovable, input.to_span_at(keyword.len()))).into())
        }
    }
}

/// Returns the input slice up to the first occurrence of the keyword.
///
/// If the pattern is never found, returns [`ControlFlow::Incomplete`] error.
#[inline]
pub fn take_until<I, K>(keyword: K) -> impl Parser<I, Output = I>
where
    K: Debug + Clone,
    I: Input + Find<K>,
{
    move |input: &mut I| {
        if let Some(offset) = input.find(keyword.clone()) {
            Ok(input.split_to(offset))
        } else {
            Err(Kind::TakeUntil(
                ControlFlow::Recovable,
                Span::Range(input.start()..input.start()),
            )
            .into())
        }
    }
}

/// Returns the longest input slice (if any) that the predicate `F` returns true.
///
/// This parser will never returns an error.
#[inline]
pub fn take_while<I, F>(mut cond: F) -> impl Parser<I, Output = I>
where
    I: Input,
    F: FnMut(I::Item) -> bool,
{
    move |input: &mut I| {
        let mut iter = input.iter();
        let mut offset = 0;
        loop {
            if let Some(next) = iter.next() {
                if !(cond)(next) {
                    break;
                }

                offset += next.len();
            } else {
                break;
            }
        }

        Ok(input.split_to(offset))
    }
}

/// Returns the longest input slice of length `n` (if any) that the predicate `F` returns true.
///
/// This parser will never returns an error.
#[inline]
pub fn take_while_range_to<I, F>(n: usize, mut cond: F) -> impl Parser<I, Output = I>
where
    I: Input,
    F: FnMut(I::Item) -> bool,
{
    move |input: &mut I| {
        let mut iter = input.iter();
        let mut offset = 0;
        let mut items = 0;
        while let Some(next) = iter.next() {
            if !(cond)(next) {
                break;
            }

            offset += next.len();
            items += 1;

            if items + 1 == n {
                break;
            }
        }

        Ok(input.split_to(offset))
    }
}

/// Returns the longest input slice of at least length `n` (if any) that the predicate `F` returns true.
///
/// This parser will never returns an error.
#[inline]
pub fn take_while_range_from<I, F>(n: usize, mut cond: F) -> impl Parser<I, Output = I>
where
    I: Input,
    F: FnMut(I::Item) -> bool,
{
    move |input: &mut I| {
        let mut iter = input.iter();
        let mut items = 0;
        let mut offset = 0;
        while let Some(next) = iter.next() {
            if !(cond)(next) {
                break;
            }

            offset += next.len();
            items += 1;
        }

        if items < n {
            return Err(Kind::TakeWhileFrom(
                ControlFlow::Recovable,
                Span::Range(input.start()..input.start() + offset),
            )
            .into());
        }

        Ok(input.split_to(offset))
    }
}

/// Returns the longest input slice of length `n` (if any) that the predicate `F` returns true.
///
/// This parser will never returns an error.
#[inline]
pub fn take_while_range<I, F>(range: Range<usize>, mut cond: F) -> impl Parser<I, Output = I>
where
    I: Input,
    F: FnMut(I::Item) -> bool,
{
    move |input: &mut I| {
        let mut iter = input.iter();
        let mut items = 0;
        let mut offset = 0;
        while let Some(next) = iter.next() {
            if !(cond)(next) {
                break;
            }

            offset += next.len();

            items += 1;

            if items + 1 == range.end {
                break;
            }
        }

        if items < range.start {
            return Err(
                Kind::TakeWhileRange(ControlFlow::Recovable, input.to_span_at(offset)).into(),
            );
        }

        Ok(input.split_to(offset))
    }
}

/// Returns the longest input slice (if any) till a predicate is met.
///
/// This parser is a short for `take_while(move |c: I::Item| !cond(c))`.
#[inline(always)]
pub fn take_till<I, F>(mut cond: F) -> impl Parser<I, Output = I>
where
    I: Input,
    F: FnMut(I::Item) -> bool,
{
    take_while(move |c: I::Item| !cond(c))
}

//! Combinators for tokenizer/lexer parsers: match single items, keywords and
//! runs of items.
//!
//! All combinators fail without consuming input: a [`ControlFlow::Recovable`]
//! error means "no match, try something else", while [`ControlFlow::Incomplete`]
//! means "the input ended too early".

use std::{
    fmt::Debug,
    ops::{Bound, RangeBounds},
};

use crate::{
    Length, Span,
    errors::{ControlFlow, Kind},
    input::{Find, Input, Item, StartWith},
    parser::Parser,
};

/// Matches the next item when it equals `item`, returning the consumed input.
///
/// # Errors
///
/// Fails with [`Kind::Next`]: [`ControlFlow::Recovable`] on a mismatch,
/// [`ControlFlow::Incomplete`] at the end of the input.
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

/// Matches the next item when `f` accepts it, returning the consumed input.
///
/// # Errors
///
/// Fails with [`Kind::NextIf`]: [`ControlFlow::Recovable`] when `f` rejects the
/// item, [`ControlFlow::Incomplete`] at the end of the input.
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

/// Recognizes `keyword` at the current position, returning the consumed input.
///
/// # Errors
///
/// Fails with [`Kind::Keyword`] ([`ControlFlow::Recovable`]) when the input does
/// not start with `keyword`.
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

/// Returns the input slice up to the first occurrence of `keyword`, leaving the
/// keyword itself unconsumed for later parsers.
///
/// # Errors
///
/// Fails with [`Kind::TakeUntil`] ([`ControlFlow::Recovable`]) when `keyword` is
/// never found.
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

/// Returns the longest leading slice of items accepted by `cond`.
///
/// Never fails: matches the empty slice when `cond` rejects the first item.
#[inline]
pub fn take_while<I, F>(cond: F) -> impl Parser<I, Output = I>
where
    I: Input,
    F: FnMut(I::Item) -> bool,
{
    take_while_with(.., cond)
}

/// Like [`take_while`], but the number of matched items must fit in `range`.
///
/// # Errors
///
/// Fails with [`Kind::NextIf`] when fewer than `range`'s lower bound items
/// match: [`ControlFlow::Incomplete`] if the input ran out,
/// [`ControlFlow::Recovable`] if `cond` rejected an item.
#[inline]
pub fn take_while_with<I, F, R>(range: R, mut cond: F) -> impl Parser<I, Output = I>
where
    I: Input,
    F: FnMut(I::Item) -> bool,
    R: RangeBounds<usize>,
{
    move |input: &mut I| {
        let min = match range.start_bound() {
            Bound::Included(&min) => min,
            Bound::Excluded(&min) => min.saturating_add(1),
            Bound::Unbounded => 0,
        };
        let max = match range.end_bound() {
            Bound::Included(&max) => max,
            Bound::Excluded(&max) => max.saturating_sub(1),
            Bound::Unbounded => usize::MAX,
        };

        let mut iter = input.iter();
        let mut offset = 0;
        let mut count = 0;
        let mut exhausted = false;

        while count < max {
            if let Some(next) = iter.next() {
                if !(cond)(next) {
                    break;
                }

                offset += next.len();
                count += 1;
            } else {
                exhausted = true;
                break;
            }
        }

        if count < min {
            let flow = if exhausted {
                ControlFlow::Incomplete
            } else {
                ControlFlow::Recovable
            };

            return Err((Kind::NextIf(flow, input.to_span_at(1))).into());
        }

        Ok(input.split_to(offset))
    }
}

/// Returns the longest leading slice of items rejected by `cond`.
///
/// Shorthand for [`take_while`] with a negated predicate.
#[inline(always)]
pub fn take_till<I, F>(mut cond: F) -> impl Parser<I, Output = I>
where
    I: Input,
    F: FnMut(I::Item) -> bool,
{
    take_while(move |c: I::Item| !cond(c))
}

// The combinators are exercised through the `TokenStream` input types,
// which are gated behind the `input` feature.
#[cfg(all(test, feature = "input"))]
mod tests {
    use std::ops::Bound;

    use super::*;
    use crate::input::{AsStr, Input, bytes, chars};
    use crate::{ControlFlow, Kind, Span, parser::Parser};

    type Bytes = bytes::TokenStream<'static>;
    type Chars = chars::TokenStream<'static>;

    #[test]
    fn next_matches_the_expected_item() {
        let mut input = Bytes::from("abc");

        let taken = next::<Bytes>(b'a').parse(&mut input).unwrap();

        assert_eq!(taken.as_str(), "a");
        assert_eq!(taken.to_span(), Span::Range(0..1));
        assert_eq!(input.as_str(), "bc");
        assert_eq!(input.start(), 1);
    }

    #[test]
    fn next_mismatch_reports_recovable_error() {
        let mut input = Bytes::from("abc");

        let err = next::<Bytes>(b'x').parse(&mut input).unwrap_err();

        assert_eq!(err, Kind::Next(ControlFlow::Recovable, Span::Range(0..1)));
        // A failed parser consumes nothing.
        assert_eq!(input.as_str(), "abc");
    }

    #[test]
    fn next_on_empty_input_reports_incomplete_error() {
        let mut input = Bytes::from("");

        let err = next::<Bytes>(b'a').parse(&mut input).unwrap_err();

        assert_eq!(err, Kind::Next(ControlFlow::Incomplete, Span::Range(0..0)));
    }

    #[test]
    fn next_counts_multibyte_chars_as_a_single_item() {
        let mut input = Chars::from("é!");

        let taken = next::<Chars>('é').parse(&mut input).unwrap();

        assert_eq!(taken.as_str(), "é");
        assert_eq!(taken.to_span(), Span::Range(0..2));
        assert_eq!(input.as_str(), "!");
    }

    #[test]
    fn next_if_accepts_items_matching_the_predicate() {
        let mut input = Bytes::from("ab");

        let taken = next_if::<Bytes, _>(|c| c.is_ascii_alphabetic())
            .parse(&mut input)
            .unwrap();

        assert_eq!(taken.as_str(), "a");
        assert_eq!(input.as_str(), "b");
    }

    #[test]
    fn next_if_rejection_reports_recovable_error() {
        let mut input = Bytes::from("1a");

        let err = next_if::<Bytes, _>(|c| c.is_ascii_alphabetic())
            .parse(&mut input)
            .unwrap_err();

        assert_eq!(err, Kind::NextIf(ControlFlow::Recovable, Span::Range(0..1)));
        assert_eq!(input.as_str(), "1a");
    }

    #[test]
    fn next_if_on_empty_input_reports_incomplete_error() {
        let mut input = Bytes::from("");

        let err = next_if::<Bytes, _>(|_| true).parse(&mut input).unwrap_err();

        assert_eq!(
            err,
            Kind::NextIf(ControlFlow::Incomplete, Span::Range(0..0))
        );
    }

    #[test]
    fn keyword_matches_a_prefix() {
        let mut input = Chars::from("if x");

        let taken = keyword::<_, Chars>("if").parse(&mut input).unwrap();

        assert_eq!(taken.as_str(), "if");
        assert_eq!(input.as_str(), " x");
    }

    #[test]
    fn keyword_mismatch_reports_recovable_error() {
        let mut input = Bytes::from("let");

        let err = keyword::<_, Bytes>("if").parse(&mut input).unwrap_err();

        // The error span covers the expected keyword width.
        assert_eq!(
            err,
            Kind::Keyword(ControlFlow::Recovable, Span::Range(0..2))
        );
        assert_eq!(input.as_str(), "let");
    }

    #[test]
    fn keyword_on_empty_input_reports_recovable_error() {
        let mut input = Bytes::from("");

        let err = keyword::<_, Bytes>("if").parse(&mut input).unwrap_err();

        assert_eq!(
            err,
            Kind::Keyword(ControlFlow::Recovable, Span::Range(0..0))
        );
    }

    #[test]
    fn take_until_stops_before_the_keyword() {
        let mut input = Bytes::from("hello world");

        let taken = take_until::<Bytes, _>("world").parse(&mut input).unwrap();

        assert_eq!(taken.as_str(), "hello ");
        // The keyword itself stays in the input for later parsers.
        assert_eq!(input.as_str(), "world");
        assert_eq!(input.start(), 6);
    }

    #[test]
    fn take_until_at_the_keyword_start_yields_an_empty_slice() {
        let mut input = Bytes::from("abc");

        let taken = take_until::<Bytes, _>("abc").parse(&mut input).unwrap();

        assert!(taken.is_empty());
        assert_eq!(input.as_str(), "abc");
    }

    #[test]
    fn take_until_missing_keyword_reports_recovable_error() {
        let mut input = Bytes::from("hello");

        let err = take_until::<Bytes, _>("world")
            .parse(&mut input)
            .unwrap_err();

        assert_eq!(
            err,
            Kind::TakeUntil(ControlFlow::Recovable, Span::Range(0..0))
        );
        assert_eq!(input.as_str(), "hello");
    }

    #[test]
    fn take_while_consumes_the_longest_matching_run() {
        let mut input = Bytes::from("aaab");

        let taken = take_while::<Bytes, _>(|c| c == b'a')
            .parse(&mut input)
            .unwrap();

        assert_eq!(taken.as_str(), "aaa");
        assert_eq!(input.as_str(), "b");
    }

    #[test]
    fn take_while_counts_multibyte_chars_as_a_single_item() {
        let mut input = Chars::from("ééa");

        let taken = take_while::<Chars, _>(|c| c == 'é')
            .parse(&mut input)
            .unwrap();

        assert_eq!(taken.as_str(), "éé");
        assert_eq!(taken.to_span(), Span::Range(0..4));
        assert_eq!(input.as_str(), "a");
    }

    #[test]
    fn take_while_may_match_an_empty_slice() {
        let mut input = Bytes::from("abc");

        let taken = take_while::<Bytes, _>(|c| c == b'x')
            .parse(&mut input)
            .unwrap();

        assert!(taken.is_empty());
        assert_eq!(input.as_str(), "abc");
    }

    #[test]
    fn take_while_with_never_takes_more_than_the_upper_bound() {
        let mut input = Bytes::from("aaaa");

        let taken = take_while_with::<Bytes, _, _>(1..=2, |c| c == b'a')
            .parse(&mut input)
            .unwrap();

        assert_eq!(taken.as_str(), "aa");
        assert_eq!(input.as_str(), "aa");
    }

    #[test]
    fn take_while_with_reports_recovable_when_the_predicate_stops_early() {
        let mut input = Bytes::from("ab");

        let err = take_while_with::<Bytes, _, _>(2..=3, |c| c == b'a')
            .parse(&mut input)
            .unwrap_err();

        assert_eq!(err, Kind::NextIf(ControlFlow::Recovable, Span::Range(0..1)));
        assert_eq!(input.as_str(), "ab");
    }

    #[test]
    fn take_while_with_reports_incomplete_when_the_input_is_exhausted() {
        let mut input = Bytes::from("a");

        let err = take_while_with::<Bytes, _, _>(2..=3, |c| c == b'a')
            .parse(&mut input)
            .unwrap_err();

        assert_eq!(
            err,
            Kind::NextIf(ControlFlow::Incomplete, Span::Range(0..1))
        );
    }

    #[test]
    fn take_while_with_exclusive_bounds_count_items() {
        let mut input = Bytes::from("aaaa");
        let bounds = (Bound::Excluded(1), Bound::Excluded(3));

        let taken = take_while_with::<Bytes, _, _>(bounds, |c| c == b'a')
            .parse(&mut input)
            .unwrap();

        // `1..3` read as a length range is exactly two items.
        assert_eq!(taken.as_str(), "aa");
    }

    #[test]
    fn take_till_stops_at_the_first_matching_item() {
        let mut input = Bytes::from("hello world");

        let taken = take_till::<Bytes, _>(|c| c == b' ')
            .parse(&mut input)
            .unwrap();

        assert_eq!(taken.as_str(), "hello");
        assert_eq!(input.as_str(), " world");
    }

    #[test]
    fn take_till_takes_the_whole_input_when_the_predicate_never_matches() {
        let mut input = Bytes::from("abc");

        let taken = take_till::<Bytes, _>(|_| false).parse(&mut input).unwrap();

        assert_eq!(taken.as_str(), "abc");
        assert!(input.is_empty());
    }
}

use std::{fmt::Debug, num::NonZeroUsize, ops::RangeInclusive};

use sourceinput::{Find, Item, Length, SplitTo, StartWith, ToSpan};

use crate::{ControlFlow, Kind, ParseError, Parser};

/// A parser match next item, otherwise raise an error.
#[inline]
pub fn next<I>(item: I::Item) -> impl Parser<I, Output = I>
where
    I: SplitTo + Clone + ToSpan,
    I::Error: ParseError,
{
    next_if(move |i| i == item)
        .map_err(|err: I::Error| Kind::Next(err.control_flow(), err.to_span()).into())
}

/// A parser match next item by `F`, otherwise raise an error.
#[inline]
pub fn next_if<I, F>(f: F) -> impl Parser<I, Output = I>
where
    I: SplitTo + Clone,
    I::Error: ParseError,
    F: FnOnce(I::Item) -> bool,
{
    move |input: &mut I| {
        if let Some(next) = input.iter().next() {
            if f(next) {
                return Ok(input.split_to(next.len()));
            }

            Err((Kind::NextIf(ControlFlow::Recovable, input.to_span_with(1))).into())
        } else {
            Err((Kind::NextIf(ControlFlow::Incomplete, input.to_span_with(1))).into())
        }
    }
}

/// Recogonize a keyword
#[inline]
pub fn keyword<KW, I>(keyword: KW) -> impl Parser<I, Output = I>
where
    I: SplitTo + StartWith<KW> + Clone,
    I::Error: ParseError,
    KW: Length + Clone,
{
    move |input: &mut I| {
        if input.start_with(keyword.clone()) {
            Ok(input.split_to(keyword.len()))
        } else {
            Err((Kind::Keyword(ControlFlow::Recovable, input.to_span_with(keyword.len()))).into())
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
    I: Find<K> + SplitTo,
    I::Error: ParseError,
{
    move |input: &mut I| {
        if let Some(offset) = input.find(keyword.clone()) {
            Ok(input.split_to(offset))
        } else {
            Err(Kind::TakeUntil(ControlFlow::Recovable, input.to_span()).into())
        }
    }
}

/// Returns the longest input slice (if any) that the predicate `F` returns true.
///
/// This parser will never returns an error.
#[inline]
pub fn take_while<I, F>(mut cond: F) -> impl Parser<I, Output = I>
where
    I: SplitTo,
    I::Error: ParseError,
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

/// Returns the longest input slice (if any) till a predicate is met.
///
/// This parser is a short for `take_while(move |c: I::Item| !cond(c))`.
#[inline(always)]
pub fn take_till<I, F>(mut cond: F) -> impl Parser<I, Output = I>
where
    I: SplitTo,
    I::Error: ParseError,
    F: FnMut(I::Item) -> bool,
{
    take_while(move |c: I::Item| !cond(c))
}

/// Returns the longest input slice of length `n` (if any) that the predicate `F` returns true.
///
/// This parser will never returns an error.
#[inline]
pub fn take_while_n<I, F>(n: NonZeroUsize, mut cond: F) -> impl Parser<I, Output = I>
where
    I: SplitTo,
    I::Error: ParseError,
    F: FnMut(I::Item) -> bool,
{
    let n = n.get();

    move |input: &mut I| {
        let mut offset = 0;
        let mut items = 0usize;

        let mut iter = input.iter();

        while let Some(next) = iter.next() {
            if !(cond)(next) {
                break;
            }

            offset += next.len();
            items += 1;

            // Safety: n > 0
            if items == n {
                break;
            }
        }

        Ok(input.split_to(offset))
    }
}

/// Returns the longest input slice of at least length `n` (if any) that the predicate `F` returns true.
///
/// Returns an error [`TakeWhileAtLeastN`](Kind::TakeWhileAtLeastN) if fewer than `n` items are parsed.
#[inline]
pub fn take_while_at_least_n<I, F>(n: NonZeroUsize, mut cond: F) -> impl Parser<I, Output = I>
where
    I: SplitTo,
    I::Error: ParseError,
    F: FnMut(I::Item) -> bool,
{
    let n = n.get();

    move |input: &mut I| {
        let mut offset = 0;
        let mut items = 0usize;

        let mut iter = input.iter();

        while let Some(next) = iter.next() {
            if !(cond)(next) {
                break;
            }

            offset += next.len();
            items += 1;
        }

        if items < n {
            return Err(Kind::TakeWhileAtLeastN(
                ControlFlow::Recovable,
                input.to_span_with(offset),
            )
            .into());
        }

        Ok(input.split_to(offset))
    }
}

/// Returns the longest input slice of length in `range` (if any) that the predicate `F` returns true.
///
/// Returns an error [`TakeWhileRange`](Kind::TakeWhileRange) if fewer than `lowerbound` items are parsed.
#[inline]
pub fn take_while_range<I, F>(
    range: RangeInclusive<usize>,
    mut cond: F,
) -> impl Parser<I, Output = I>
where
    I: SplitTo,
    I::Error: ParseError,
    F: FnMut(I::Item) -> bool,
{
    assert!(!range.is_empty());

    move |input: &mut I| {
        let mut offset = 0;
        let mut items = 0usize;

        let mut iter = input.iter();

        while let Some(next) = iter.next() {
            // Safety: range.end > 0
            if !(items < *range.end()) {
                break;
            }

            if !(cond)(next) {
                break;
            }

            offset += next.len();
            items += 1;
        }

        if items < *range.start() {
            return Err(
                Kind::TakeWhileRange(ControlFlow::Recovable, input.to_span_with(offset)).into(),
            );
        }

        Ok(input.split_to(offset))
    }
}

#[cfg(test)]
mod tests {

    use std::num::NonZeroUsize;

    use sourceinput::Span;

    use crate::{
        Kind, Parser,
        combinators::{
            keyword, next, take_till, take_until, take_while_at_least_n, take_while_n,
            take_while_range,
        },
    };

    type Chars<'a> = sourceinput::Chars<'a, Kind>;

    #[test]
    fn test_next() {
        let mut input = Chars::begin("await");

        assert_eq!(next('a').parse(&mut input), Ok((0, "a").into()));
        assert_eq!(
            next('a').parse(&mut input),
            Err(Kind::Next(crate::ControlFlow::Recovable, Span::from(1..2)))
        );

        assert_eq!(next('w').parse(&mut input), Ok((1, "w").into()));
        assert_eq!(next('a').parse(&mut input), Ok((2, "a").into()));
        assert_eq!(next('i').parse(&mut input), Ok((3, "i").into()));
        assert_eq!(next('t').parse(&mut input), Ok((4, "t").into()));

        assert_eq!(
            next('a').parse(&mut input),
            Err(Kind::Next(crate::ControlFlow::Incomplete, Span::from(5..5)))
        );
    }

    #[test]
    fn test_keyword() {
        assert_eq!(
            keyword("await").parse(&mut Chars::begin("await~~")),
            Ok((0, "await").into())
        );
        assert_eq!(
            keyword(b"await").parse(&mut Chars::begin("await~~")),
            Ok((0, "await").into())
        );

        assert_eq!(
            keyword(b"await").parse(&mut Chars::begin("~await~~")),
            Err(Kind::Keyword(
                crate::ControlFlow::Recovable,
                Span::from(0..5)
            ))
        );

        assert_eq!(
            keyword(b"await").parse(&mut Chars::begin("~")),
            Err(Kind::Keyword(
                crate::ControlFlow::Recovable,
                Span::from(0..1)
            ))
        );
    }

    #[test]
    fn test_take_until() {
        assert_eq!(
            take_until(b"await").parse(&mut Chars::begin("~!!!await")),
            Ok((0, "~!!!").into())
        );

        assert_eq!(
            take_until(b"await").parse(&mut Chars::begin("~!!!")),
            Err(Kind::TakeUntil(
                crate::ControlFlow::Recovable,
                Span::from(0..4)
            ))
        );
    }

    #[test]
    fn test_take_till() {
        assert_eq!(
            take_till(|c| c == 't').parse(&mut Chars::begin("~!!!await")),
            Ok((0, "~!!!awai").into())
        );

        assert_eq!(
            take_till(|c| c == 't').parse(&mut Chars::begin("~!!!awai")),
            Ok((0, "~!!!awai").into())
        );
    }

    #[test]
    fn test_take_while_n() {
        assert_eq!(
            take_while_n(NonZeroUsize::new(4).unwrap(), |c: char| c.is_ascii_digit())
                .parse(&mut Chars::begin("123456789")),
            Ok((0, "1234").into())
        );

        assert_eq!(
            take_while_n(NonZeroUsize::new(4).unwrap(), |c: char| c.is_ascii_digit())
                .parse(&mut Chars::begin("12")),
            Ok((0, "12").into())
        );

        assert_eq!(
            take_while_n(NonZeroUsize::new(4).unwrap(), |c: char| c.is_ascii_digit())
                .parse(&mut Chars::begin("")),
            Ok((0, "").into())
        );
    }

    #[test]
    fn test_take_while_at_least_n() {
        assert_eq!(
            take_while_at_least_n(NonZeroUsize::new(4).unwrap(), |c: char| c.is_ascii_digit())
                .parse(&mut Chars::begin("123456789~")),
            Ok((0, "123456789").into())
        );

        assert_eq!(
            take_while_at_least_n(NonZeroUsize::new(4).unwrap(), |c: char| c.is_ascii_digit())
                .parse(&mut Chars::begin("123")),
            Err(Kind::TakeWhileAtLeastN(
                crate::ControlFlow::Recovable,
                Span::from(0..3)
            ))
        );
    }

    #[test]
    fn test_take_while_range() {
        assert_eq!(
            take_while_range(0..=0, |c: char| c.is_ascii_digit())
                .parse(&mut Chars::begin("123456789~")),
            Ok((0, "").into())
        );

        assert_eq!(
            take_while_range(2..=2, |c: char| c.is_ascii_digit())
                .parse(&mut Chars::begin("123456789~")),
            Ok((0, "12").into())
        );

        assert_eq!(
            take_while_range(0..=1, |c: char| c.is_ascii_digit())
                .parse(&mut Chars::begin("123456789~")),
            Ok((0, "1").into())
        );

        assert_eq!(
            take_while_range(0..=4, |c: char| c.is_ascii_digit())
                .parse(&mut Chars::begin("123456789~")),
            Ok((0, "1234").into())
        );

        assert_eq!(
            take_while_range(0..=4, |c: char| c.is_ascii_digit())
                .parse(&mut Chars::begin("~123456789~")),
            Ok((0, "").into())
        );

        assert_eq!(
            take_while_range(1..=4, |c: char| c.is_ascii_digit())
                .parse(&mut Chars::begin("~123456789~")),
            Err(Kind::TakeWhileRange(
                crate::ControlFlow::Recovable,
                Span::from(0..0)
            ))
        );
    }
}

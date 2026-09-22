//! Core [`Parser`] trait and its built-in combinators.
//!
//! A parser consumes an [`Input`] stream and produces either a value or an
//! [`Input::Error`]. Parsers are composed by chaining the methods of [`Parser`],
//! and every closure `FnOnce(&mut I) -> Result<O, I::Error>` already implements
//! the trait.

use crate::{
    errors::{ControlFlow, ParseError},
    input::Input,
};

/// A parser combinator.
///
/// A parser consumes part of an [`Input`] stream and produces an
/// [`output`](Parser::Output) value or an error. Combinators chain parsers
/// together, and any closure `FnOnce(&mut I) -> Result<Self::Output, I::Error>`
/// implements this trait.
///
/// Errors come in two flavors: `non-fatal` errors ([`ControlFlow::Recovable`]
/// and [`ControlFlow::Incomplete`]) may be retried by backtracking combinators
/// such as [`ok`](Parser::ok) and [`or`](Parser::or), while
/// [`fatal`](ControlFlow::Fatal) errors abort the whole parsing process.
pub trait Parser<I>
where
    I: Input,
{
    /// The value produced on success.
    type Output;

    /// Consumes `self` and parses `input`, which is advanced past the consumed items on success.
    ///
    /// On error the input may be left partially consumed; combinators that backtrack
    /// ([`ok`](Parser::ok), [`or`](Parser::or)) take care of restoring it themselves.
    fn parse(self, input: &mut I) -> Result<Self::Output, I::Error>;

    /// Creates a parser that turns a `non-fatal` failure into a `None` value.
    ///
    /// A success is wrapped in `Some`; a `non-fatal` error rewinds `input` to
    /// where it was before the call and yields `None`; a
    /// [`fatal`](ControlFlow::Fatal) error is still propagated.
    #[inline]
    fn ok(self) -> impl Parser<I, Output = Option<Self::Output>>
    where
        I: Clone,
        Self: Sized,
    {
        IsOk(self)
    }

    /// Creates a parser that applies `f` to the output on success, leaving errors untouched.
    #[inline]
    fn map<F, O>(self, f: F) -> impl Parser<I, Output = O>
    where
        F: FnOnce(Self::Output) -> O,
        Self: Sized,
    {
        Map(self, f)
    }

    /// Creates a parser that applies `f` to the error on failure, leaving outputs untouched.
    #[inline]
    fn map_err<F>(self, f: F) -> impl Parser<I, Output = Self::Output>
    where
        F: FnOnce(I::Error) -> I::Error,
        Self: Sized,
    {
        MapErr(self, f)
    }

    /// Creates a parser that promotes every error to a [`fatal`](ControlFlow::Fatal) one,
    /// so backtracking combinators such as [`ok`](Parser::ok) and [`or`](Parser::or)
    /// can no longer retry it.
    #[inline]
    fn fatal(self) -> impl Parser<I, Output = Self::Output>
    where
        Self: Sized,
    {
        Fatal(self)
    }

    /// Creates a parser that boxes its output. Shorthand for [`map`](Parser::map) with `Box::new`.
    #[inline]
    fn boxed(self) -> impl Parser<I, Output = Box<Self::Output>>
    where
        Self: Sized,
    {
        self.map(|v| Box::new(v))
    }

    /// Creates a parser that falls back to `parser` when this one fails with a `non-fatal` error.
    ///
    /// Both alternatives start from the same input position, and the first
    /// success wins. A [`fatal`](ControlFlow::Fatal) error from either side is
    /// propagated immediately.
    #[inline]
    fn or<R>(self, parser: R) -> impl Parser<I, Output = Self::Output>
    where
        I: Clone,
        R: Parser<I, Output = Self::Output>,
        Self: Sized,
    {
        Or(self, parser)
    }
}

/// Every `FnOnce(&mut I) -> Result<O, I::Error>` closure implements [`Parser`].
impl<O, I, F> Parser<I> for F
where
    I: Input,
    F: FnOnce(&mut I) -> Result<O, I::Error>,
{
    type Output = O;

    #[inline]
    fn parse(self, input: &mut I) -> Result<Self::Output, I::Error> {
        self(input)
    }
}

/// Backtracking wrapper behind [`Parser::ok`].
struct IsOk<P>(P);

impl<P, I> Parser<I> for IsOk<P>
where
    I: Input + Clone,
    P: Parser<I>,
{
    type Output = Option<P::Output>;

    #[inline]
    fn parse(self, input: &mut I) -> Result<Self::Output, I::Error> {
        // Snapshot the input so a `non-fatal` error can rewind to this position.
        let snapshot = input.clone();
        match self.0.parse(input) {
            Ok(t) => Ok(Some(t)),
            Err(err) if err.control_flow() == ControlFlow::Fatal => Err(err),
            Err(_) => {
                *input = snapshot;
                Ok(None)
            }
        }
    }
}

/// Output-mapping wrapper behind [`Parser::map`].
struct Map<P, F>(P, F);

impl<P, I, F, O> Parser<I> for Map<P, F>
where
    I: Input,
    P: Parser<I>,
    F: FnOnce(P::Output) -> O,
{
    type Output = O;

    #[inline]
    fn parse(self, input: &mut I) -> Result<Self::Output, I::Error> {
        self.0.parse(input).map(|output| (self.1)(output))
    }
}

/// Error-mapping wrapper behind [`Parser::map_err`].
struct MapErr<P, F>(P, F);

impl<P, I, F> Parser<I> for MapErr<P, F>
where
    I: Input,
    P: Parser<I>,
    F: FnOnce(I::Error) -> I::Error,
{
    type Output = P::Output;

    #[inline]
    fn parse(self, input: &mut I) -> Result<Self::Output, I::Error> {
        self.0.parse(input).map_err(|output| (self.1)(output))
    }
}

/// Error-promoting wrapper behind [`Parser::fatal`].
struct Fatal<P>(P);

impl<P, I> Parser<I> for Fatal<P>
where
    I: Input,
    P: Parser<I>,
{
    type Output = P::Output;

    #[inline]
    fn parse(self, input: &mut I) -> Result<Self::Output, I::Error> {
        match self.0.parse(input) {
            Err(err) => Err(err.into_fatal()),
            r => r,
        }
    }
}

/// Ordered choice (try the left parser, fall back to the right) behind [`Parser::or`].
struct Or<L, R>(L, R);

impl<L, R, I, O> Parser<I> for Or<L, R>
where
    I: Input + Clone,
    L: Parser<I, Output = O>,
    R: Parser<I, Output = O>,
{
    type Output = O;

    #[inline]
    fn parse(self, input: &mut I) -> Result<Self::Output, I::Error> {
        let mut try_input = input.clone();
        if let Some(v) = self.0.ok().parse(&mut try_input)? {
            *input = try_input;
            return Ok(v);
        }

        self.1.parse(input)
    }
}

// The `Parser` trait is exercised through hand-rolled parsers on `TokenStream`.
#[cfg(all(test, feature = "input"))]
mod tests {
    use super::*;
    use crate::Kind;
    use crate::input::{AsStr, Input, chars};

    type Chars = chars::TokenStream<'static>;

    /// Consumes exactly one char on success.
    fn one_char(input: &mut Chars) -> Result<Chars, Kind> {
        Ok(input.split_to(1))
    }

    /// Consumes one char and then fails with a `non-fatal` error.
    fn eat_then_fail(input: &mut Chars) -> Result<Chars, Kind> {
        input.split_to(1);

        Err(Kind::Next(ControlFlow::Recovable, input.to_span()))
    }

    /// Fails with a `non-fatal` error without consuming anything.
    fn fail_recoverable(input: &mut Chars) -> Result<Chars, Kind> {
        Err(Kind::Next(ControlFlow::Recovable, input.to_span()))
    }

    /// Fails with a `fatal` error without consuming anything.
    fn fail_fatal(input: &mut Chars) -> Result<Chars, Kind> {
        Err(Kind::Next(ControlFlow::Fatal, input.to_span()))
    }

    #[test]
    fn closures_implement_parser() {
        let mut input = Chars::from("xy");

        let taken = one_char.parse(&mut input).unwrap();

        assert_eq!(taken.as_str(), "x");
        assert_eq!(input.as_str(), "y");
    }

    #[test]
    fn map_transforms_the_output() {
        let mut input = Chars::from("xy");

        let taken = one_char
            .map(|taken| taken.as_str().to_owned())
            .parse(&mut input)
            .unwrap();

        assert_eq!(taken, "x");
    }

    #[test]
    fn map_err_rewrites_the_error() {
        let mut input = Chars::from("xy");

        let err = fail_recoverable
            .map_err(|err| err.into_fatal())
            .parse(&mut input)
            .unwrap_err();

        assert!(err.is_fatal());
    }

    #[test]
    fn ok_wraps_success_in_some() {
        let mut input = Chars::from("xy");

        let taken = one_char.ok().parse(&mut input).unwrap();

        assert_eq!(taken.unwrap().as_str(), "x");
    }

    #[test]
    fn ok_turns_recoverable_failure_into_none() {
        let mut input = Chars::from("xy");

        let taken = fail_recoverable.ok().parse(&mut input).unwrap();

        assert!(taken.is_none());
        assert_eq!(input.as_str(), "xy");
    }

    #[test]
    fn ok_rewinds_the_input_after_a_failure() {
        let mut input = Chars::from("xy");

        let taken = eat_then_fail.ok().parse(&mut input).unwrap();

        assert!(taken.is_none());
        // The partially consumed char is given back.
        assert_eq!(input.as_str(), "xy");
    }

    #[test]
    fn ok_propagates_fatal_errors() {
        let mut input = Chars::from("xy");

        let err = fail_fatal.ok().parse(&mut input).unwrap_err();

        assert!(err.is_fatal());
    }

    #[test]
    fn fatal_promotes_recoverable_errors() {
        let mut input = Chars::from("xy");

        let err = fail_recoverable.fatal().parse(&mut input).unwrap_err();

        assert!(err.is_fatal());
    }

    #[test]
    fn boxed_boxes_the_output() {
        let mut input = Chars::from("xy");

        let taken = one_char.boxed().parse(&mut input).unwrap();

        assert_eq!(taken.as_str(), "x");
    }

    #[test]
    fn or_prefers_the_left_parser() {
        let mut input = Chars::from("xy");

        let taken = one_char.or(eat_then_fail).parse(&mut input).unwrap();

        assert_eq!(taken.as_str(), "x");
    }

    #[test]
    fn or_rewinds_before_trying_the_right_parser() {
        let mut input = Chars::from("xy");

        let taken = eat_then_fail.or(one_char).parse(&mut input).unwrap();

        // The left parser consumed `x` before failing, the right one still sees it.
        assert_eq!(taken.as_str(), "x");
        assert_eq!(input.as_str(), "y");
    }

    #[test]
    fn or_propagates_fatal_errors_without_retrying() {
        let mut input = Chars::from("xy");

        let err = fail_fatal.or(one_char).parse(&mut input).unwrap_err();

        assert!(err.is_fatal());
        assert_eq!(input.as_str(), "xy");
    }
}

//! Traits for parser combinators.

use crate::{
    errors::{ControlFlow, ParseError},
    input::Input,
};

/// A parsing combinator should implement this trait.
pub trait Parser<I>
where
    I: Input,
{
    type Output;

    /// Consumes itself and parses the input stream to generate the `output` product.
    fn parse(self, input: &mut I) -> Result<Self::Output, I::Error>;

    /// Creates a new parser that converts `non-fatal` error into `None` value.
    #[inline]
    fn ok(self) -> impl Parser<I, Output = Option<Self::Output>>
    where
        I: Clone,
        Self: Sized,
    {
        IsOk(self)
    }

    /// On success, use func `F` to convert origin output to type `O`
    #[inline]
    fn map<F, O>(self, f: F) -> impl Parser<I, Output = O>
    where
        F: FnOnce(Self::Output) -> O,
        Self: Sized,
    {
        Map(self, f)
    }

    /// On failed, use func `F` to convert origin error
    #[inline]
    fn map_err<F>(self, f: F) -> impl Parser<I, Output = Self::Output>
    where
        F: FnOnce(I::Error) -> I::Error,
        Self: Sized,
    {
        MapErr(self, f)
    }

    /// Creates a parser that convert all `non-fatal` error into [`fatal`](ControlFlow::Fatal) error.
    #[inline]
    fn fatal(self) -> impl Parser<I, Output = Self::Output>
    where
        Self: Sized,
    {
        Fatal(self)
    }

    /// Map output into `Box<Self::Output>`, this func is short for code `Parser::map(|v|Box::new(v))`
    #[inline]
    fn boxed(self) -> impl Parser<I, Output = Box<Self::Output>>
    where
        Self: Sized,
    {
        self.map(|v| Box::new(v))
    }

    /// Executre another `Parser` if this one returns a `non-fatal` error.
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

/// Implement [`Parser`] for all `FnOnce(I) -> Result<O, I, E>`
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

struct IsOk<P>(P);

impl<P, I> Parser<I> for IsOk<P>
where
    I: Input + Clone,
    P: Parser<I>,
{
    type Output = Option<P::Output>;

    #[inline]
    fn parse(self, input: &mut I) -> Result<Self::Output, I::Error> {
        let snapshot = input.clone();

        // for retrospective analysis, we clone the input stream.
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

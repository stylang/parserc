use parserc::{
    ControlFlow, Kind, Span,
    chars::{self, CharsInput},
    syntax::{InputSyntaxExt, Syntax},
};

type TokenStream<'a> = chars::TokenStream<'a, Kind>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Syntax)]
#[parserc(take_while = |c: char| c.is_ascii_alphabetic())]
struct Ident<I>(pub I)
where
    I: CharsInput;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Syntax)]
enum T<I>
where
    I: CharsInput,
{
    A(
        #[parserc(left_recursion,map_err = ParseError::into_fatal)] Box<T<I>>,
        Ident<I>,
    ),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Syntax)]
struct T1<I>
where
    I: CharsInput,
{
    #[parserc(left_recursion,map_err = ParseError::into_fatal)]
    pub t1: Box<T1<I>>,
    pub ident: Ident<I>,
}

#[test]
fn test_derive() {
    assert_eq!(
        TokenStream::from("hello world").parse(),
        Ok(Ident(TokenStream::from("hello")))
    );
}

#[test]
fn test_left_recursion() {
    assert_eq!(
        T::parse(&mut TokenStream::from("")),
        Err(Kind::LeftRecursion(ControlFlow::Fatal, Span::Range(0..0)))
    );

    assert_eq!(
        T1::parse(&mut TokenStream::from("")),
        Err(Kind::LeftRecursion(ControlFlow::Fatal, Span::Range(0..0)))
    );
}

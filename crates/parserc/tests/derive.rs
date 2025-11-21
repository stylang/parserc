use parserc::{
    Kind,
    chars::{self, CharsInput},
    syntax::InputSyntaxExt,
};

use parserc_derive::Syntax;

type TokenStream<'a> = chars::TokenStream<'a, Kind>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Syntax)]
#[parserc(take_while = |c: char| c.is_ascii_alphabetic())]
struct Ident<I>(pub I)
where
    I: CharsInput;

#[test]
fn test_derive() {
    assert_eq!(
        TokenStream::from("hello world").parse(),
        Ok(Ident(TokenStream::from("hello")))
    );
}

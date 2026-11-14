// The Rust the generator wrote for the spec in `data/spec.rs`.
//
// This file is machine-made: `cargo test -p unsyn --test dump -- --nocapture`
// prints what `Codegen` produces for that spec, and the result is kept here so
// that the emitted types are compiled and run rather than only read back as
// tokens. `tests/generated.rs` compares the two whole, and says so when they
// fall apart.
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WS<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for WS<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| {
                ((::core::matches!(c, ' ') || ::core::matches!(c, '\t'))
                    || ::core::matches!(c, '\n'))
            })
            .parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "WS",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LETTER<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for LETTER<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| {
                (::core::matches!(c, 'a'..='z') || ::core::matches!(c, 'A'..='Z'))
            })
            .parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "LETTER",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DIGIT<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for DIGIT<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '0'..='9')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "DIGIT",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WORD<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for WORD<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            input.parse::<LETTER<I>>()?;
            ::parserc::take_while(|c: char| {
                (((::core::matches!(c, 'a'..='z') || ::core::matches!(c, 'A'..='Z'))
                    || ::core::matches!(c, '0'..='9'))
                    || ::core::matches!(c, '_'))
            })
            .parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "WORD",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LET<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for LET<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::keyword("let").parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "LET",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FN<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for FN<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::keyword("fn").parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "FN",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct KEYWORDS<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for KEYWORDS<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            |input: &mut I| -> ::core::result::Result<(), <I as ::parserc::Input>::Error> {
                input.parse::<LET<I>>()?;
                ::core::result::Result::Ok(())
            }
            .or(
                |input: &mut I| -> ::core::result::Result<(), <I as ::parserc::Input>::Error> {
                    input.parse::<FN<I>>()?;
                    ::core::result::Result::Ok(())
                },
            )
            .parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "KEYWORDS",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IDENT<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for IDENT<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            let __start = input.clone();
            input.parse::<WORD<I>>()?;
            if {
                let mut __probe = __start.clone();
                (|__probe: &mut I| -> ::core::result::Result<(), <I as ::parserc::Input>::Error> {
                    __probe.parse::<KEYWORDS<I>>()?;
                    ::core::result::Result::Ok(())
                })
                .parse(&mut __probe)
                .is_ok()
            } {
                return Err(::parserc::Kind::Token(
                    "IDENT",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "IDENT",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OPEN<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for OPEN<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '(')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "OPEN",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CLOSE<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for CLOSE<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, ')')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "CLOSE",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct COMMA<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for COMMA<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, ',')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "COMMA",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SEMI<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for SEMI<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, ';')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "SEMI",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ARROW<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for ARROW<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::keyword("->").parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "ARROW",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SHARP<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for SHARP<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '#')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "SHARP",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LEFT<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for LEFT<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '[')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "LEFT",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RIGHT<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for RIGHT<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, ']')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "RIGHT",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TILDE<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for TILDE<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '~')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "TILDE",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LETTER_OR_ANY<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for LETTER_OR_ANY<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::take_while_with(1.., |_: char| true).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "LETTER_OR_ANY",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct QUOTE<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for QUOTE<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '\'')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "QUOTE",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BSLASH<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for BSLASH<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '\\')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "BSLASH",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MATCHER<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for MATCHER<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            crate::backend::word_start(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "MATCHER",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SLASH<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for SLASH<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '/')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "SLASH",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct STAR<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for STAR<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '*')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "STAR",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UNDER<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for UNDER<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '_')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "UNDER",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Plus<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for Plus<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '+')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "Plus",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Minus<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for Minus<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '-')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "Minus",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Underscore<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for Underscore<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '_')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "Underscore",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TEXT11<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for TEXT11<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| !(::core::matches!(c, '\\') || ::core::matches!(c, '\'')))
                .parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "TEXT11",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TEXT12<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for TEXT12<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| (::core::matches!(c, 'n') || ::core::matches!(c, 't')))
                .parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "TEXT12",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Any<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for Any<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|_: char| true).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "Any",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Commented11<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for Commented11<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| !::core::matches!(c, '*')).parse(input)?;
            if input.len() == __from.len() {
                return Err(::parserc::Kind::Token(
                    "Commented11",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            let __taken = __from.len() - input.len();
            Ok(Self(__from.split_to(__taken)))
        }
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        self.0.to_span()
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ITEM<I>(
    pub KEYWORDS<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub IDENT<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub ARROW<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub EXPR<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub SEMI<I>,
);
impl<I> ::parserc::syntax::Syntax<I> for ITEM<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<KEYWORDS<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<IDENT<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<ARROW<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<EXPR<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<SEMI<I>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
            .union(&self.3.to_span())
            .union(&self.4.to_span())
            .union(&self.5.to_span())
            .union(&self.6.to_span())
            .union(&self.7.to_span())
            .union(&self.8.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EXPR<I>(
    pub ::std::boxed::Box<PRIMARY<I>>,
    pub ::std::vec::Vec<(::std::vec::Vec<WS<I>>, ::std::boxed::Box<EXPR1<I>>)>,
);
impl<I> ::parserc::syntax::Syntax<I> for EXPR<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<::std::boxed::Box<PRIMARY<I>>>()?,
            input.parse::<::std::vec::Vec<(::std::vec::Vec<WS<I>>, ::std::boxed::Box<EXPR1<I>>)>>(
            )?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PRIMARY<I> {
    PRIMARYIDENT(IDENT<I>),
    PRIMARYLITERAL(LITERAL<I>),
    PRIMARYGROUP(::std::boxed::Box<GROUP<I>>),
    PRIMARYNEGATED(::std::boxed::Box<NEGATED<I>>),
}
impl<I> ::parserc::syntax::Syntax<I> for PRIMARY<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::PRIMARYIDENT(input.parse::<IDENT<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::PRIMARYLITERAL(input.parse::<LITERAL<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::PRIMARYGROUP(
                    input.parse::<::std::boxed::Box<GROUP<I>>>()?,
                ))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::PRIMARYNEGATED(
                    input.parse::<::std::boxed::Box<NEGATED<I>>>()?,
                ))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        ::core::result::Result::Err(
            ::parserc::Kind::Syntax(
                "PRIMARY",
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into(),
        )
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        match self {
            Self::PRIMARYIDENT(__0) => ::parserc::Span::None.union(&__0.to_span()),
            Self::PRIMARYLITERAL(__0) => ::parserc::Span::None.union(&__0.to_span()),
            Self::PRIMARYGROUP(__0) => ::parserc::Span::None.union(&__0.to_span()),
            Self::PRIMARYNEGATED(__0) => ::parserc::Span::None.union(&__0.to_span()),
        }
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GROUP<I>(
    pub OPEN<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub ::std::boxed::Box<EXPR<I>>,
    pub ::std::vec::Vec<WS<I>>,
    pub CLOSE<I>,
);
impl<I> ::parserc::syntax::Syntax<I> for GROUP<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<OPEN<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<::std::boxed::Box<EXPR<I>>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<CLOSE<I>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
            .union(&self.3.to_span())
            .union(&self.4.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NEGATED<I>(
    pub TILDE<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub ::std::boxed::Box<PRIMARY<I>>,
);
impl<I> ::parserc::syntax::Syntax<I> for NEGATED<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<TILDE<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<::std::boxed::Box<PRIMARY<I>>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BINOP<I> {
    BINOPARROW(ARROW<I>),
    BINOPPlus(Plus<I>),
    BINOPMinus(Minus<I>),
}
impl<I> ::parserc::syntax::Syntax<I> for BINOP<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::BINOPARROW(input.parse::<ARROW<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::BINOPPlus(input.parse::<Plus<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::BINOPMinus(input.parse::<Minus<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        ::core::result::Result::Err(
            ::parserc::Kind::Syntax(
                "BINOP",
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into(),
        )
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        match self {
            Self::BINOPARROW(__0) => ::parserc::Span::None.union(&__0.to_span()),
            Self::BINOPPlus(__0) => ::parserc::Span::None.union(&__0.to_span()),
            Self::BINOPMinus(__0) => ::parserc::Span::None.union(&__0.to_span()),
        }
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LITERAL<I> {
    LITERALNUMBER(NUMBER<I>),
    LITERALTEXT(TEXT<I>),
}
impl<I> ::parserc::syntax::Syntax<I> for LITERAL<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::LITERALNUMBER(input.parse::<NUMBER<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::LITERALTEXT(input.parse::<TEXT<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        ::core::result::Result::Err(
            ::parserc::Kind::Syntax(
                "LITERAL",
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into(),
        )
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        match self {
            Self::LITERALNUMBER(__0) => ::parserc::Span::None.union(&__0.to_span()),
            Self::LITERALTEXT(__0) => ::parserc::Span::None.union(&__0.to_span()),
        }
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NUMBER<I>(
    pub DIGIT<I>,
    pub ::std::vec::Vec<(::std::vec::Vec<WS<I>>, NUMBER1<I>)>,
);
impl<I> ::parserc::syntax::Syntax<I> for NUMBER<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<DIGIT<I>>()?,
            input.parse::<::std::vec::Vec<(::std::vec::Vec<WS<I>>, NUMBER1<I>)>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TEXT<I>(
    pub QUOTE<I>,
    pub ::std::vec::Vec<(::std::vec::Vec<WS<I>>, TEXT1<I>)>,
    pub ::std::vec::Vec<WS<I>>,
    pub QUOTE<I>,
);
impl<I> ::parserc::syntax::Syntax<I> for TEXT<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<QUOTE<I>>()?,
            input.parse::<::std::vec::Vec<(::std::vec::Vec<WS<I>>, TEXT1<I>)>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<QUOTE<I>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
            .union(&self.3.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PARAMS<I>(
    pub OPEN<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub ::core::option::Option<PARAMS1<I>>,
    pub ::std::vec::Vec<WS<I>>,
    pub CLOSE<I>,
);
impl<I> ::parserc::syntax::Syntax<I> for PARAMS<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<OPEN<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<::core::option::Option<PARAMS1<I>>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<CLOSE<I>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
            .union(&self.3.to_span())
            .union(&self.4.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LIST<I>(
    pub OPEN<I>,
    pub ::std::vec::Vec<(::std::vec::Vec<WS<I>>, IDENT<I>)>,
    pub ::std::vec::Vec<WS<I>>,
    pub CLOSE<I>,
);
impl<I> ::parserc::syntax::Syntax<I> for LIST<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<OPEN<I>>()?,
            input.parse::<::std::vec::Vec<(::std::vec::Vec<WS<I>>, IDENT<I>)>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<CLOSE<I>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
            .union(&self.3.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TUPLE<I>(
    pub OPEN<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub IDENT<I>,
    pub ::std::vec::Vec<(::std::vec::Vec<WS<I>>, TUPLE1<I>)>,
    pub ::std::vec::Vec<WS<I>>,
    pub CLOSE<I>,
);
impl<I> ::parserc::syntax::Syntax<I> for TUPLE<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<OPEN<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<IDENT<I>>()?,
            {
                let __run =
                    input.parse::<::std::vec::Vec<(::std::vec::Vec<WS<I>>, TUPLE1<I>)>>()?;
                if __run.is_empty() {
                    return ::core::result::Result::Err(
                        ::parserc::Kind::Syntax(
                            "TUPLE",
                            ::parserc::ControlFlow::Recovable,
                            input.to_span_at(1),
                        )
                        .into(),
                    );
                }
                __run
            },
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<CLOSE<I>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
            .union(&self.3.to_span())
            .union(&self.4.to_span())
            .union(&self.5.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SHAPE<I>(pub SHARP<I>, pub SHAPE1<I>);
impl<I> ::parserc::syntax::Syntax<I> for SHAPE<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<SHARP<I>>()?,
            input.parse::<SHAPE1<I>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ANYONE<I>(
    pub ANYONE1<I>,
    pub ::std::vec::Vec<(::std::vec::Vec<WS<I>>, ANYONE2<I>)>,
);
impl<I> ::parserc::syntax::Syntax<I> for ANYONE<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<ANYONE1<I>>()?,
            input.parse::<::std::vec::Vec<(::std::vec::Vec<WS<I>>, ANYONE2<I>)>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Joined<I>(
    pub (DIGIT<I>, UNDER<I>),
    pub ::std::vec::Vec<WS<I>>,
    pub DIGIT<I>,
);
impl<I> ::parserc::syntax::Syntax<I> for Joined<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<(DIGIT<I>, UNDER<I>)>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<DIGIT<I>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Guarded<I>(pub Guarded1<I>);
impl<I> ::parserc::syntax::Syntax<I> for Guarded<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(input.parse::<Guarded1<I>>()?))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None.union(&self.0.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Commented<I>(
    pub SLASH<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub STAR<I>,
    pub ::std::vec::Vec<(::std::vec::Vec<WS<I>>, Commented1<I>)>,
    pub ::std::vec::Vec<WS<I>>,
    pub STAR<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub SLASH<I>,
);
impl<I> ::parserc::syntax::Syntax<I> for Commented<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<SLASH<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<STAR<I>>()?,
            input.parse::<::std::vec::Vec<(::std::vec::Vec<WS<I>>, Commented1<I>)>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<STAR<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<SLASH<I>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
            .union(&self.3.to_span())
            .union(&self.4.to_span())
            .union(&self.5.to_span())
            .union(&self.6.to_span())
            .union(&self.7.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Either<I>(
    pub Either1<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub LEFT<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub IDENT<I>,
);
impl<I> ::parserc::syntax::Syntax<I> for Either<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<Either1<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<LEFT<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<IDENT<I>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
            .union(&self.3.to_span())
            .union(&self.4.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dotted<I>(pub Dotted1<I>);
impl<I> ::parserc::syntax::Syntax<I> for Dotted<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(input.parse::<Dotted1<I>>()?))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None.union(&self.0.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EXPR1<I>(
    pub BINOP<I>,
    pub ::std::vec::Vec<WS<I>>,
    pub ::std::boxed::Box<PRIMARY<I>>,
);
impl<I> ::parserc::syntax::Syntax<I> for EXPR1<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<BINOP<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<::std::boxed::Box<PRIMARY<I>>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NUMBER1<I> {
    NUMBER1DIGIT(DIGIT<I>),
    NUMBER1Underscore(Underscore<I>),
}
impl<I> ::parserc::syntax::Syntax<I> for NUMBER1<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::NUMBER1DIGIT(input.parse::<DIGIT<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::NUMBER1Underscore(input.parse::<Underscore<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        ::core::result::Result::Err(
            ::parserc::Kind::Syntax(
                "NUMBER1",
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into(),
        )
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        match self {
            Self::NUMBER1DIGIT(__0) => ::parserc::Span::None.union(&__0.to_span()),
            Self::NUMBER1Underscore(__0) => ::parserc::Span::None.union(&__0.to_span()),
        }
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TEXT1<I> {
    TEXT1Tilde(TEXT11<I>),
    TEXT1BSLASH(BSLASH<I>, ::std::vec::Vec<WS<I>>, TEXT12<I>),
}
impl<I> ::parserc::syntax::Syntax<I> for TEXT1<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::TEXT1Tilde(input.parse::<TEXT11<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::TEXT1BSLASH(
                    input.parse::<BSLASH<I>>()?,
                    input.parse::<::std::vec::Vec<WS<I>>>()?,
                    input.parse::<TEXT12<I>>()?,
                ))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        ::core::result::Result::Err(
            ::parserc::Kind::Syntax(
                "TEXT1",
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into(),
        )
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        match self {
            Self::TEXT1Tilde(__0) => ::parserc::Span::None.union(&__0.to_span()),
            Self::TEXT1BSLASH(__0, __1, __2) => ::parserc::Span::None
                .union(&__0.to_span())
                .union(&__1.to_span())
                .union(&__2.to_span()),
        }
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PARAMS11<I>(pub COMMA<I>, pub ::std::vec::Vec<WS<I>>, pub IDENT<I>);
impl<I> ::parserc::syntax::Syntax<I> for PARAMS11<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<COMMA<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<IDENT<I>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PARAMS1<I>(
    pub IDENT<I>,
    pub ::std::vec::Vec<(::std::vec::Vec<WS<I>>, PARAMS11<I>)>,
);
impl<I> ::parserc::syntax::Syntax<I> for PARAMS1<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<IDENT<I>>()?,
            input.parse::<::std::vec::Vec<(::std::vec::Vec<WS<I>>, PARAMS11<I>)>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TUPLE1<I>(pub COMMA<I>, pub ::std::vec::Vec<WS<I>>, pub IDENT<I>);
impl<I> ::parserc::syntax::Syntax<I> for TUPLE1<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(
            input.parse::<COMMA<I>>()?,
            input.parse::<::std::vec::Vec<WS<I>>>()?,
            input.parse::<IDENT<I>>()?,
        ))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None
            .union(&self.0.to_span())
            .union(&self.1.to_span())
            .union(&self.2.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SHAPE1<I>(pub ::std::vec::Vec<(::std::vec::Vec<WS<I>>, DIGIT<I>)>);
impl<I> ::parserc::syntax::Syntax<I> for SHAPE1<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        let mut __items = ::std::vec::Vec::new();
        loop {
            if __items.len() >= 4 {
                break;
            }
            let ::core::option::Option::Some(__next) =
                input.parse::<::core::option::Option<(::std::vec::Vec<WS<I>>, DIGIT<I>)>>()?
            else {
                break;
            };
            __items.push(__next);
        }
        if __items.len() < 4 {
            return ::core::result::Result::Err(
                ::parserc::Kind::Syntax(
                    "SHAPE1",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into(),
            );
        }
        ::core::result::Result::Ok(Self(__items))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None.union(&self.0.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ANYONE1<I> {
    ANYONE1MATCHER(MATCHER<I>),
    ANYONE1Any(Any<I>),
}
impl<I> ::parserc::syntax::Syntax<I> for ANYONE1<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::ANYONE1MATCHER(input.parse::<MATCHER<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::ANYONE1Any(input.parse::<Any<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        ::core::result::Result::Err(
            ::parserc::Kind::Syntax(
                "ANYONE1",
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into(),
        )
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        match self {
            Self::ANYONE1MATCHER(__0) => ::parserc::Span::None.union(&__0.to_span()),
            Self::ANYONE1Any(__0) => ::parserc::Span::None.union(&__0.to_span()),
        }
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ANYONE2<I> {
    ANYONE2MATCHER(MATCHER<I>),
    ANYONE2Any(Any<I>),
}
impl<I> ::parserc::syntax::Syntax<I> for ANYONE2<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::ANYONE2MATCHER(input.parse::<MATCHER<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::ANYONE2Any(input.parse::<Any<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        ::core::result::Result::Err(
            ::parserc::Kind::Syntax(
                "ANYONE2",
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into(),
        )
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        match self {
            Self::ANYONE2MATCHER(__0) => ::parserc::Span::None.union(&__0.to_span()),
            Self::ANYONE2Any(__0) => ::parserc::Span::None.union(&__0.to_span()),
        }
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Guarded1<I>(pub IDENT<I>);
impl<I> ::parserc::syntax::Syntax<I> for Guarded1<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        let __taken = input.parse::<IDENT<I>>()?;
        {
            let mut __probe = input.clone();
            let __fits =
                (|__probe: &mut I| -> ::core::result::Result<(), <I as ::parserc::Input>::Error> {
                    __probe.parse::<Minus<I>>()?;
                    ::core::result::Result::Ok(())
                })
                .ok()
                .parse(&mut __probe)?;
            if __fits.is_none() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Syntax(
                        "Guarded1",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
        }
        ::core::result::Result::Ok(Self(__taken))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None.union(&self.0.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Commented1<I> {
    Commented1Tilde(Commented11<I>),
    Commented1UNDER(UNDER<I>),
}
impl<I> ::parserc::syntax::Syntax<I> for Commented1<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::Commented1Tilde(input.parse::<Commented11<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::Commented1UNDER(input.parse::<UNDER<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        ::core::result::Result::Err(
            ::parserc::Kind::Syntax(
                "Commented1",
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into(),
        )
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        match self {
            Self::Commented1Tilde(__0) => ::parserc::Span::None.union(&__0.to_span()),
            Self::Commented1UNDER(__0) => ::parserc::Span::None.union(&__0.to_span()),
        }
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Either1<I> {
    Either1IDENT(IDENT<I>),
    Either1DIGIT(DIGIT<I>),
}
impl<I> ::parserc::syntax::Syntax<I> for Either1<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::Either1IDENT(input.parse::<IDENT<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        if let ::core::option::Option::Some(__alt) =
            (|input: &mut I| -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
                ::core::result::Result::Ok(Self::Either1DIGIT(input.parse::<DIGIT<I>>()?))
            })
            .ok()
            .parse(input)?
        {
            return ::core::result::Result::Ok(__alt);
        }
        ::core::result::Result::Err(
            ::parserc::Kind::Syntax(
                "Either1",
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into(),
        )
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        match self {
            Self::Either1IDENT(__0) => ::parserc::Span::None.union(&__0.to_span()),
            Self::Either1DIGIT(__0) => ::parserc::Span::None.union(&__0.to_span()),
        }
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dotted1<I>(pub ::std::vec::Vec<(::std::vec::Vec<WS<I>>, IDENT<I>)>);
impl<I> ::parserc::syntax::Syntax<I> for Dotted1<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        let mut __items = ::std::vec::Vec::new();
        loop {
            if __items.len() >= 3 {
                break;
            }
            let ::core::option::Option::Some(__next) =
                input.parse::<::core::option::Option<(::std::vec::Vec<WS<I>>, IDENT<I>)>>()?
            else {
                break;
            };
            __items.push(__next);
        }
        if __items.len() < 2 {
            return ::core::result::Result::Err(
                ::parserc::Kind::Syntax(
                    "Dotted1",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into(),
            );
        }
        ::core::result::Result::Ok(Self(__items))
    }
    fn to_span(&self) -> ::parserc::Span {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::parserc::Span::None.union(&self.0.to_span())
    }
}

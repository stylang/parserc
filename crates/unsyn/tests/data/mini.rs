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
            ::parserc::take_while_with(1.., |c: char| {
                ((::core::matches!(c, ' ') || ::core::matches!(c, '\t'))
                    || ::core::matches!(c, '\n'))
            })
            .parse(input)?;
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
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "LETTER",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "DIGIT",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "WORD",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "LET",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "FN",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "KEYWORDS",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
                let mut __ahead = __start.clone();
                let mut __text = __ahead.split_to(input.start() - __start.start());
                (|__probe: &mut I| -> ::core::result::Result<(), <I as ::parserc::Input>::Error> {
                    __probe.parse::<KEYWORDS<I>>()?;
                    ::core::result::Result::Ok(())
                })
                .parse(&mut __text)
                .is_ok()
                    && __text.is_empty()
            } {
                return Err(::parserc::Kind::Token(
                    "IDENT",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "IDENT",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
pub struct REACH<I>(pub I);
impl<I> ::parserc::syntax::Syntax<I> for REACH<I>
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
            input.parse::<LETTER<I>>()?;
            if {
                let mut __ahead = __start.clone();
                let mut __text = __ahead.split_to(input.start() - __start.start());
                (|__probe: &mut I| -> ::core::result::Result<(), <I as ::parserc::Input>::Error> {
                    ::parserc::keyword("abcd").parse(__probe)?;
                    ::core::result::Result::Ok(())
                })
                .parse(&mut __text)
                .is_ok()
                    && __text.is_empty()
            } {
                return Err(::parserc::Kind::Token(
                    "REACH",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into());
            }
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "REACH",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
pub struct OPEN<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '(')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "OPEN",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
pub struct CLOSE<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, ')')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "CLOSE",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
pub struct COMMA<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, ',')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "COMMA",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
pub struct SEMI<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, ';')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "SEMI",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
pub struct ARROW<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::keyword("->").parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "ARROW",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
pub struct SHARP<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '#')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "SHARP",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
pub struct LEFT<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '[')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "LEFT",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
pub struct RIGHT<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, ']')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "RIGHT",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
pub struct TILDE<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '~')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "TILDE",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "LETTER_OR_ANY",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "QUOTE",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "BSLASH",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "MATCHER",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
pub struct SLASH<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '/')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "SLASH",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "STAR",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
pub struct UNDER<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '_')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "UNDER",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
pub struct Plus<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '+')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "Plus",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
pub struct Minus<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '-')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "Minus",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
pub struct Underscore<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| ::core::matches!(c, '_')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "Underscore",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
pub struct TEXT1<I>(pub I);
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
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| !(::core::matches!(c, '\\') || ::core::matches!(c, '\'')))
                .parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "TEXT1",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
pub struct TEXT2<I>(
    pub ::core::option::Option<WS<I>>,
    pub I,
    pub ::core::option::Option<WS<I>>,
);
impl<I> ::parserc::syntax::Syntax<I> for TEXT2<I>
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
            let __leading = input.parse::<::core::option::Option<WS<I>>>()?;
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| (::core::matches!(c, 'n') || ::core::matches!(c, 't')))
                .parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "TEXT2",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
            }
            let __taken = __from.len() - input.len();
            let __own = __from.split_to(__taken);
            let __trailing = input.parse::<::core::option::Option<WS<I>>>()?;
            Ok(Self(__leading, __own, __trailing))
        }
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
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "Any",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
pub struct Commented1<I>(pub I);
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
        {
            let mut __from = input.clone();
            ::parserc::next_if(|c: char| !::core::matches!(c, '*')).parse(input)?;
            if input.len() == __from.len() {
                return ::core::result::Result::Err(
                    ::parserc::Kind::Token(
                        "Commented1",
                        ::parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into(),
                );
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
    pub ::core::option::Option<WS<I>>,
    pub IDENT<I>,
    pub ::parserc::syntax::Delimiter<ARROW<I>, SEMI<I>, EXPR<I>>,
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
            input.parse::<::core::option::Option<WS<I>>>()?,
            input.parse::<IDENT<I>>()?,
            input.parse::<::parserc::syntax::Delimiter<ARROW<I>, SEMI<I>, EXPR<I>>>()?,
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
pub struct EXPR<I>(
    pub ::std::boxed::Box<PRIMARY<I>>,
    pub ::core::option::Option<WS<I>>,
    pub  ::std::vec::Vec<(
        ::core::option::Option<WS<I>>,
        BINOP<I>,
        ::core::option::Option<WS<I>>,
        ::std::boxed::Box<PRIMARY<I>>,
    )>,
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
            input.parse::<::core::option::Option<WS<I>>>()?,
            input.parse::<::std::vec::Vec<(
                ::core::option::Option<WS<I>>,
                BINOP<I>,
                ::core::option::Option<WS<I>>,
                ::std::boxed::Box<PRIMARY<I>>,
            )>>()?,
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
    pub ::parserc::syntax::Delimiter<OPEN<I>, CLOSE<I>, ::std::boxed::Box<EXPR<I>>>,
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
        ::core::result::Result::Ok(Self(input.parse::<::parserc::syntax::Delimiter<
            OPEN<I>,
            CLOSE<I>,
            ::std::boxed::Box<EXPR<I>>,
        >>()?))
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
pub struct NEGATED<I>(pub TILDE<I>, pub ::std::boxed::Box<PRIMARY<I>>);
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
    pub ::core::option::Option<WS<I>>,
    pub  ::std::vec::Vec<(
        ::core::option::Option<WS<I>>,
        ::parserc::syntax::Or<DIGIT<I>, Underscore<I>>,
    )>,
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
            input.parse::<::core::option::Option<WS<I>>>()?,
            input.parse::<::std::vec::Vec<(
                ::core::option::Option<WS<I>>,
                ::parserc::syntax::Or<DIGIT<I>, Underscore<I>>,
            )>>()?,
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
pub struct TEXT<I>(
    pub QUOTE<I>,
    pub ::core::option::Option<WS<I>>,
    pub  ::std::vec::Vec<(
        ::core::option::Option<WS<I>>,
        ::parserc::syntax::Or<TEXT1<I>, (BSLASH<I>, TEXT2<I>)>,
    )>,
    pub ::core::option::Option<WS<I>>,
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
            input.parse::<::core::option::Option<WS<I>>>()?,
            input.parse::<::std::vec::Vec<(
                ::core::option::Option<WS<I>>,
                ::parserc::syntax::Or<TEXT1<I>, (BSLASH<I>, TEXT2<I>)>,
            )>>()?,
            input.parse::<::core::option::Option<WS<I>>>()?,
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
            .union(&self.4.to_span())
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PARAMS<I>(
    pub  ::parserc::syntax::Delimiter<
        OPEN<I>,
        CLOSE<I>,
        ::core::option::Option<(
            IDENT<I>,
            ::core::option::Option<WS<I>>,
            ::std::vec::Vec<(COMMA<I>, IDENT<I>)>,
        )>,
    >,
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
        ::core::result::Result::Ok(Self(input.parse::<::parserc::syntax::Delimiter<
            OPEN<I>,
            CLOSE<I>,
            ::core::option::Option<(
                IDENT<I>,
                ::core::option::Option<WS<I>>,
                ::std::vec::Vec<(COMMA<I>, IDENT<I>)>,
            )>,
        >>()?))
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
pub struct ARGS<I>(
    pub  ::parserc::syntax::Delimiter<
        OPEN<I>,
        CLOSE<I>,
        ::parserc::syntax::Punctuated<IDENT<I>, COMMA<I>>,
    >,
);
impl<I> ::parserc::syntax::Syntax<I> for ARGS<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(input.parse::<::parserc::syntax::Delimiter<
            OPEN<I>,
            CLOSE<I>,
            ::parserc::syntax::Punctuated<IDENT<I>, COMMA<I>>,
        >>()?))
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
pub struct PAIRED<I>(pub COMMA<I>, pub SEMI<I>);
impl<I> ::parserc::syntax::Syntax<I> for PAIRED<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(input.parse::<COMMA<I>>()?, input.parse::<SEMI<I>>()?))
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
pub struct RUNDED<I>(
    pub TILDE<I>,
    pub ::parserc::syntax::Delimiter<LEFT<I>, RIGHT<I>, IDENT<I>>,
);
impl<I> ::parserc::syntax::Syntax<I> for RUNDED<I>
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
            input.parse::<::parserc::syntax::Delimiter<LEFT<I>, RIGHT<I>, IDENT<I>>>()?,
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
pub struct LIST<I>(
    pub  ::parserc::syntax::Delimiter<
        OPEN<I>,
        CLOSE<I>,
        ::std::vec::Vec<(::core::option::Option<WS<I>>, IDENT<I>)>,
    >,
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
        ::core::result::Result::Ok(Self(input.parse::<::parserc::syntax::Delimiter<
            OPEN<I>,
            CLOSE<I>,
            ::std::vec::Vec<(::core::option::Option<WS<I>>, IDENT<I>)>,
        >>()?))
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
pub struct SPACED<I>(
    pub  ::parserc::syntax::Delimiter<
        OPEN<I>,
        CLOSE<I>,
        (
            IDENT<I>,
            ::std::vec::Vec<(::core::option::Option<WS<I>>, IDENT<I>)>,
        ),
    >,
);
impl<I> ::parserc::syntax::Syntax<I> for SPACED<I>
where
    I: ::unsyn::runtime::GeneratedInput,
{
    fn parse(input: &mut I) -> ::core::result::Result<Self, <I as ::parserc::Input>::Error> {
        #[allow(unused_imports)]
        use ::parserc::{
            AsStr, Find, Input, Parser, StartWith,
            syntax::{Syntax, SyntaxInput},
        };
        ::core::result::Result::Ok(Self(input.parse::<::parserc::syntax::Delimiter<
            OPEN<I>,
            CLOSE<I>,
            (
                IDENT<I>,
                ::std::vec::Vec<(::core::option::Option<WS<I>>, IDENT<I>)>,
            ),
        >>()?))
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
pub struct TUPLE<I>(
    pub  ::parserc::syntax::Delimiter<
        OPEN<I>,
        CLOSE<I>,
        (
            IDENT<I>,
            ((COMMA<I>, IDENT<I>), ::std::vec::Vec<(COMMA<I>, IDENT<I>)>),
        ),
    >,
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
        ::core::result::Result::Ok(Self(input.parse::<::parserc::syntax::Delimiter<
            OPEN<I>,
            CLOSE<I>,
            (
                IDENT<I>,
                ((COMMA<I>, IDENT<I>), ::std::vec::Vec<(COMMA<I>, IDENT<I>)>),
            ),
        >>()?))
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
    pub ::parserc::syntax::Or<MATCHER<I>, Any<I>>,
    pub ::core::option::Option<WS<I>>,
    pub  ::std::vec::Vec<(
        ::core::option::Option<WS<I>>,
        ::parserc::syntax::Or<MATCHER<I>, Any<I>>,
    )>,
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
            input.parse::<::parserc::syntax::Or<MATCHER<I>, Any<I>>>()?,
            input.parse::<::core::option::Option<WS<I>>>()?,
            input.parse::<::std::vec::Vec<(
                ::core::option::Option<WS<I>>,
                ::parserc::syntax::Or<MATCHER<I>, Any<I>>,
            )>>()?,
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
pub struct Joined<I>(pub (DIGIT<I>, UNDER<I>), pub DIGIT<I>);
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
    pub STAR<I>,
    pub ::core::option::Option<WS<I>>,
    pub  ::std::vec::Vec<(
        ::core::option::Option<WS<I>>,
        ::parserc::syntax::Or<Commented1<I>, UNDER<I>>,
    )>,
    pub ::core::option::Option<WS<I>>,
    pub STAR<I>,
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
            input.parse::<STAR<I>>()?,
            input.parse::<::core::option::Option<WS<I>>>()?,
            input.parse::<::std::vec::Vec<(
                ::core::option::Option<WS<I>>,
                ::parserc::syntax::Or<Commented1<I>, UNDER<I>>,
            )>>()?,
            input.parse::<::core::option::Option<WS<I>>>()?,
            input.parse::<STAR<I>>()?,
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
    }
}
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Either<I>(
    pub ::parserc::syntax::Or<IDENT<I>, DIGIT<I>>,
    pub LEFT<I>,
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
            input.parse::<::parserc::syntax::Or<IDENT<I>, DIGIT<I>>>()?,
            input.parse::<LEFT<I>>()?,
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
pub struct SHAPE1<I>(pub ::std::vec::Vec<(::core::option::Option<WS<I>>, DIGIT<I>)>);
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
            let ::core::option::Option::Some(__next) = input
                .parse::<::core::option::Option<(::core::option::Option<WS<I>>, DIGIT<I>)>>()?
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
        if !{
            let mut __probe = input.clone();
            (|__probe: &mut I| -> ::core::result::Result<(), <I as ::parserc::Input>::Error> {
                __probe.parse::<Minus<I>>()?;
                ::core::result::Result::Ok(())
            })
            .ok()
            .parse(&mut __probe)?
            .is_some()
        } {
            return ::core::result::Result::Err(
                ::parserc::Kind::Syntax(
                    "Guarded1",
                    ::parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into(),
            );
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
pub struct Dotted1<I>(pub ::std::vec::Vec<(::core::option::Option<WS<I>>, IDENT<I>)>);
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
            let ::core::option::Option::Some(__next) = input
                .parse::<::core::option::Option<(::core::option::Option<WS<I>>, IDENT<I>)>>()?
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

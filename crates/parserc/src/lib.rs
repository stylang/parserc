//! Parser combinator framework for the `rust` language.
//!
//! `parserc` builds parsers by composing small single-purpose parsers —
//! *combinators*. A parser consumes an [`Input`] stream and returns either a
//! parsed value or an error whose [`Span`] points into the source code, ready
//! for diagnostics.
//!
//! # Crate layout
//!
//! - [`Parser`] — the [`Parser`] trait and its combinators:
//!   [`map`](Parser::map), [`map_err`](Parser::map_err), [`ok`](Parser::ok),
//!   [`or`](Parser::or), [`fatal`](Parser::fatal), [`boxed`](Parser::boxed).
//! - Tokenizer combinators — [`next`], [`next_if`], [`keyword`], [`take_until`],
//!   [`take_while`], [`take_while_with`], [`take_till`].
//! - [`Input`] — the [`Input`] trait and the ready-to-use streams
//!   [`chars::TokenStream`] (yields [`char`] items) and [`bytes::TokenStream`]
//!   (yields [`u8`] items).
//! - [`ControlFlow`] / [`Kind`] / [`ParseError`] — the error model.
//! - [`Span`] / [`BeforeSpan`] — source code regions.
//! - [`syntax`] (feature `syntax`) — AST building blocks ([`Syntax`](syntax::Syntax)
//!   trait, [`syntax::Delimiter`], [`syntax::Punctuated`], …) and the derive macro.
//!
//! # Error model
//!
//! Every error carries a [`ControlFlow`] code:
//!
//! - [`ControlFlow::Recovable`] — the parser did not match; an alternative may
//!   still succeed.
//! - [`ControlFlow::Incomplete`] — the input ended before the parser could finish.
//! - [`ControlFlow::Fatal`] — parsing cannot continue; backtracking combinators
//!   ([`ok`](Parser::ok), [`or`](Parser::or)) propagate it instead of retrying.
//!
//! Parsers fail without consuming input; combinators that try alternatives
//! rewind the input themselves before retrying.
//!
//! # Quick start
//!
//! ```
//! use parserc::{AsStr, Parser, chars::TokenStream, keyword, take_while, take_while_with};
//!
//! type Chars = TokenStream<'static>;
//!
//! let mut input = Chars::from("let answer = 42");
//!
//! // Match a keyword, skip whitespace, then read an identifier.
//! let kw = keyword::<_, Chars>("let").parse(&mut input).unwrap();
//! let ws = take_while::<Chars, _>(|c| c.is_whitespace()).parse(&mut input).unwrap();
//! let name = take_while_with::<Chars, _, _>(1.., |c| c.is_ascii_alphabetic())
//!     .map(|taken| taken.as_str().to_owned())
//!     .parse(&mut input)
//!     .unwrap();
//!
//! assert_eq!(kw.as_str(), "let");
//! assert_eq!(ws.as_str(), " ");
//! assert_eq!(name, "answer");
//! // The unparsed remainder is still available on the input.
//! assert_eq!(input.as_str(), " = 42");
//! ```
//!
//! # Alternatives and backtracking
//!
//! [`or`](Parser::or) runs the right parser only when the left one fails with a
//! `non-fatal` error, and [`fatal`](Parser::fatal) forbids such retries:
//!
//! ```
//! use parserc::{AsStr, ParseError, Parser, chars::TokenStream, keyword};
//!
//! type Chars = TokenStream<'static>;
//!
//! let mut input = Chars::from("const x = 1");
//! let kw = keyword::<_, Chars>("let")
//!     .or(keyword::<_, Chars>("const"))
//!     .parse(&mut input)
//!     .unwrap();
//! assert_eq!(kw.as_str(), "const");
//!
//! // A plain failure is recoverable ...
//! let mut input = Chars::from("x");
//! let err = keyword::<_, Chars>("let").parse(&mut input).unwrap_err();
//! assert!(!err.is_fatal());
//!
//! // ... while `fatal` stops every backtracking combinator.
//! let err = keyword::<_, Chars>("let").fatal().parse(&mut input).unwrap_err();
//! assert!(err.is_fatal());
//! ```
//!
//! # Feature flags
//!
//! - `input` (default) — the [`chars`] and [`bytes`] [`Input`] implementations.
//! - `syntax` (default) — the [`syntax`] module and the
//!   [`Syntax`](syntax::Syntax) derive macro.
//! - `serde` (default) — `serde` support for [`Span`] and the error types.
#![cfg_attr(docsrs, feature(doc_cfg))]

mod input;
pub use input::*;

mod errors;
pub use errors::*;

mod span;
pub use span::*;

mod parser;
pub use parser::*;

mod c;
pub use c::*;

#[cfg(feature = "syntax")]
#[cfg_attr(docsrs, doc(cfg(feature = "syntax")))]
pub mod syntax;

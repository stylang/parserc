//! A DSL for specifying concrete syntax trees.
//!
//! A `unsyn` source file is a list of node definitions. Each one names a node
//! of the grammar and gives the pattern it matches:
//!
//! - `whitespace` declares the trivia of the language: at most one per file.
//! - `lexer` declares a terminal: a pattern over characters.
//! - `syntax` declares a non-terminal: a pattern over other nodes.
//!
//! This crate is the implementation of that language, in four stages, each one
//! dependent on the previous one:
//!
//! - [`input`] holds the [`Input`](parserc::Input) the parsers share.
//! - [`lexical`] recognises the tokens: identifiers, keywords, punctuation,
//!   delimiters, literals and the several comment and documentation forms.
//! - [`syntax`] applies the context-free grammar of the language and produces a
//!   [`Crate`](syntax::Crate), the CST of the source file.
//! - [`semantics`] gives that CST meaning: it resolves the references between
//!   nodes, validates their patterns and rejects the definitions that can start
//!   with themselves.
//!
//! ```
//! use parserc::syntax::Syntax;
//! use unsyn::{input::TokenStream, semantics::analyze, syntax::Crate};
//!
//! let source = r#"
//!     whitespace W -> U+0020 | U+0009;
//!     lexer XID_Start -> <XID_Start>;
//!     lexer XID_Continue -> <XID_Continue>;
//!     lexer IDENTIFIER -> ( XID_Start | '_' ) XID_Continue*;
//!     syntax PROGRAM -> IDENTIFIER;
//! "#;
//!
//! let mut input = TokenStream::from(source);
//! let file = Crate::parse(&mut input).expect("the source should parse");
//!
//! let module = analyze(&file).expect("the source should be well-formed");
//!
//! assert_eq!(module.symbols().len(), 5);
//! assert_eq!(module.definition("IDENTIFIER").unwrap().kind().keyword(), "lexer");
//!
//! // `<Name>` is a call of a matcher the backend provides, not a node.
//! assert_eq!(module.matchers(), ["XID_Start", "XID_Continue"]);
//! ```
//!
//! The spec of the language, written in the language itself, lives in
//! `bootstrap.uns` next to this crate and is checked by the tests of
//! [`semantics`].

pub mod errors;
pub mod input;
pub mod lexical;
pub mod semantics;
pub mod syntax;

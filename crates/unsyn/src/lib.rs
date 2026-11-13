//! A DSL for specifying concrete syntax trees.
//!
//! A `unsyn` source file is a list of node definitions. Each one names a node
//! of the grammar and gives the pattern it matches:
//!
//! - `whitespace` declares the trivia of the language: at most one per file.
//!   The node it declares is the natural separator of the language, which every
//!   `syntax` rule takes between its operands as greedily as the text offers,
//!   so a rule of nodes has no need to name it. A rule that names it as one of
//!   its operands says the separator twice: the mention is given no field of
//!   its own, and a lint is written about it instead.
//! - `lexer` declares a terminal: a pattern over characters.
//! - `syntax` declares a non-terminal: a pattern over other nodes.
//!
//! Every operand that matches consumes at least one character: `~` asks its
//! question of the character here and then takes it, and the repetitions, the
//! options and the bounds that may match nothing at all never match twice at
//! one position. What an `except` or a `followed` asserts about the text is the
//! exception the DSL makes, being tested and then left where it was. The
//! recursion checks count on this, since whatever consumes is what stands
//! between a definition and itself.
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
//!   nodes, validates their patterns, rejects the definitions that can start
//!   with themselves, and reports what a definition says twice as a lint.
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
//!
//! // A set whose members say nothing twice earns no lint.
//! assert!(module.lints().is_empty());
//! ```
//!
//! The spec of the language, written in the language itself, lives in
//! `bootstrap.uns` next to this crate and is checked by the tests of
//! [`semantics`], and [`codegen`] is what turns a analyzed file into the Rust of
//! the tree it describes.

pub mod codegen;
pub mod errors;
pub mod input;
pub mod lexical;
pub mod semantics;
pub mod syntax;

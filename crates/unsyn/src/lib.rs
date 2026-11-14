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
//! # Two phases, two features
//!
//! The crate does two jobs that have nothing to do with each other, and the
//! features of `Cargo.toml` say so:
//!
//! - `runtime`, the [`runtime`] module, is what a grammar the generator writes
//!   is *compiled against*: the [`GeneratedInput`](runtime::GeneratedInput)
//!   every generated type is bounded by, and the one question the DSL asks of
//!   itself, about where a keyword ends. What a `<NAME>` in a pattern calls is
//!   deliberately not here: such a name is the caller's link, and unsyn ships no
//!   answer to it.
//!   It knows nothing of the DSL, and needs nothing but a character stream.
//! - `codegen`, the same-named feature, is the DSL itself: the reader, the
//!   analyzer and the generator, in the stages its own module document lays
//!   out, over `quote` and `proc_macro2`.
//!
//! The spec of the language, written in the language itself, lives in
//! `bootstrap.uns` next to this crate, and `tests/bootstrap.rs` runs the reader
//! and the analyzer over it, from source text to the module a tree is generated
//! from.
//!
//! A crate that keeps a generated file and never produces one therefore takes
//! the runtime alone, and leaves `quote` and `proc-macro2` out of its tree:
//!
//! ```toml
//! [dependencies]
//! unsyn = { version = "0.13", default-features = false, features = ["runtime"] }
//! ```
//!
//! A build without the `runtime` feature has pages that point at items it does
//! not hold: the two above point at each other's phase, and the generator's own
//! documentation names the types a file it writes will call. Those links are
//! allowed to break in that case rather than written twice, since nothing about
//! the build changes, only what the documentation can show.
#![cfg_attr(not(feature = "runtime"), allow(rustdoc::broken_intra_doc_links))]

#[cfg(feature = "codegen")]
pub mod codegen;
#[cfg(feature = "codegen")]
pub mod errors;
#[cfg(feature = "codegen")]
pub mod input;
#[cfg(feature = "codegen")]
pub mod lexical;
#[cfg(feature = "runtime")]
pub mod runtime;
#[cfg(feature = "codegen")]
pub mod semantics;
#[cfg(feature = "codegen")]
pub mod syntax;

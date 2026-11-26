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
//! one position. What an `except` asserts about the text is the exception the
//! DSL makes, being tested over the run the operand took and then left where it
//! was. The recursion checks count on this, since whatever consumes is what
//! stands between a definition and itself.
//!
//! # Where the trivia goes
//!
//! A run of trivia lies between two words of the language, and since the node
//! that declares it is greedy and takes each run whole, the run is one node of
//! the tree and never part of two. Where it is filed is settled by the shape of
//! the productions, never by what the characters of the described language
//! happen to be:
//!
//! - no operand keeps a run for itself. A terminal holds its own text and
//!   nothing else, and a `lexer` token is read out of the text it is handed, so
//!   a run taken there would end up inside that other token;
//! - between two operands of a sequence stands a field of the sequence, one
//!   optional run of the trivia, laid beside the operands rather than around
//!   either of them;
//! - an option and a repetition open each of their matches with the run that
//!   stands before them, so no field lies before such an operand as well — that
//!   leading run is what tells two matches of one repetition apart.
//!
//! The one exception is a repetition of a rule that can itself match the trivia,
//! such as the items of a file: a run laid before every item would be taken by
//! the item meant to keep it, so such a list carries no leading run, and the
//! trivia stands among the items as an item of its own.
//!
//! None of this changes what a grammar accepts: the first operand that reaches a
//! run keeps it, and keeps all of it. What it decides is where in the tree the
//! run is filed away, which is why a rule over nodes names the trivia of its own
//! language at most once.
//!
//! The mapping from a production to a Rust type, and the rules for naming what
//! the mapping produces, are written down once in `README.md` beside this
//! crate's sources. The documents of [`read`], [`semantic`] and [`codegen`] go on
//! from there to how each is implemented.
//!
//! # One job, one feature
//!
//! The crate does one job: the DSL itself — the reader, the analyzer and the
//! generator, in the stages [`codegen`] lays out, over `quote` and `proc_macro2`.
//! There is no feature for a runtime, because there is no runtime to
//! ship. Two facts about a generated file make that plain:
//!
//! - what a `<NAME>` in a pattern reaches is the caller's link, and unsyn ships
//!   no answer to it; where a word of the described language ends is the spec's
//!   business, and unsyn assumes no boundary for it.
//! - a generated file today asks only [`parserc`] — the character stream it
//!   reads, the combinators it runs, and the
//!   [`Syntax`](parserc::syntax::Syntax) its nodes derive.
//!
//! A placeholder module for answers a later phase might want would hold nothing
//! a generated file needs, and the generator must not reach into it either: what
//! it emits is decided by the tree a spec compiles to and by what the caller
//! configured, not by items unsyn happens to keep.
//!
//! The spec of the language, written in the language itself, lives in
//! `unsyn.us` at the root of this crate — and this crate is built from it. The
//! front end (`lexer`, the nodes, the visitor and the error a file reports) is not
//! a hand-written file: it is what the generator writes for the spec, into
//! `src/generated`, where it is committed. [`unsyn-gen`] is the run that makes it,
//! and there is no build script: a crate that generates itself from the tree it is
//! in needs no earlier release of itself, so a change to what the generator *writes*
//! lands in the same release as the change that made it.
//!
//! [`unsyn-gen`]: https://github.com/stylang/stylang/blob/main/crates/unsyn/src/bin/unsyn-gen.rs
//!
//! That is also why the front end carries no `impl` block and no hand-picked
//! name: every type, field and variant below is what the productions say. What
//! the crate adds by hand is the reading of that tree
//! ([`read`]), what it denotes ([`semantic`]), and what becomes of it
//! ([`codegen`]).
//!
//! A crate that keeps a generated file and never produces one therefore needs no
//! unsyn at all: `parserc`, with the two features a generated file names, and
//! nothing else. `quote` and `proc-macro2` stay out of its tree, since only the
//! writing of a spec reaches for them:
//!
//! ```toml
//! [dependencies]
//! parserc = { version = "0.14", features = ["input", "syntax"] }
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod codegen;
pub mod decode;
pub mod error;
pub mod input;
pub mod read;
pub mod scan;
pub mod semantic;

// The front end of the language, in the code the generator wrote for `unsyn.us`.
// These are not hand-maintained files: the spec at the root of this crate is the
// source of every type below, and `src/bin/unsyn-gen.rs` is what writes them,
// into `src/generated`, by the generator this crate ships. A
// `use super::input::UnsynInput` inside one of them is this crate's own `input`,
// and a `super::lexer` is this crate's own `lexer` — mounted as siblings of the
// crate root, the generated files are the siblings the generator writes them for.
//
// Only the error's own file is not mounted here: it lives inside `error`, as
// `error::generated`, so that the generated error and the wrapper around it are
// one module's two parts rather than two modules with nearly one name.
//
// No allowance is written on these modules. What the generator has to say about a
// shape it cannot name is said by the code itself, on the items that need it, so a
// warning here is a defect of the generator rather than a debt of the crate.
#[path = "generated/lexer.rs"]
pub mod lexer;

#[path = "generated/syntax.rs"]
pub mod syntax;

#[path = "generated/visit.rs"]
pub mod visit;

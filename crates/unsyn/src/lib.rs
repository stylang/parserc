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
//! The `compiler` module's own documents say how the rule is decided and what the
//! generated types look like once it has been.
//!
//! # Two phases, two features
//!
//! The crate does two jobs that have nothing to do with each other, and the
//! features of `Cargo.toml` say so:
//!
//! - `runtimes`, the [`runtimes`] module, is the side a grammar the generator
//!   writes belongs to: what a generated file is *compiled against*, and
//!   nothing else. It is deliberately not what a generated file calls: what a
//!   `<NAME>` in a pattern reaches is the caller's link, and unsyn ships no
//!   answer to it; where a word of the described language ends is the spec's
//!   business, and unsyn assumes no boundary for it. A generated file today
//!   asks only [`parserc`] — the character stream it reads, the combinators it
//!   runs, and the [`Syntax`](parserc::syntax::Syntax) its nodes derive — so
//!   this module holds no item that a generated file needs, and stays a place
//!   for the answers a later phase of the language will want to ship without
//!   the generator having to reach into `compiler` for them.
//! - `compiler`, the same-named feature, is the DSL itself: the reader, the
//!   analyzer and the generator, in the stages its own module document lays
//!   out, over `quote` and `proc_macro2`.
//!
//! The spec of the language, written in the language itself, lives in
//! `unsyn.us` at the root of this crate, and `tests/unsyn_us.rs` runs the
//! reader, the analyzer and the generator over it, from source text to the code
//! of a parser and back to reading with it.
//!
//! A crate that keeps a generated file and never produces one therefore needs
//! no unsyn at all: `parserc`, with the two features a generated file names,
//! and nothing else. `quote` and `proc-macro2` stay out of its tree, since only
//! the writing of a spec reaches for them:
//!
//! ```toml
//! [dependencies]
//! parserc = { version = "0.13", features = ["input", "syntax"] }
//! ```
//!
//! A build without the `compiler` feature has pages that point at items it does
//! not hold: the two phases point at each other, and the generator's own
//! documentation names the types a file it writes will call. Those links are
//! allowed to break in that case rather than written twice, since nothing about
//! the build changes, only what the documentation can show.
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "compiler")]
pub mod compiler;

#[cfg(feature = "runtimes")]
pub mod runtimes;

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
//! that declares it is greedy and takes each run whole, that run belongs to
//! exactly one of the two. Which one is settled by the shape of the productions,
//! never by what the characters of the described language happen to be:
//!
//! - a terminal whose pattern denotes finitely many fixed words — `'<'`, `';'`,
//!   `'->'`, `'lexer'` — *owns* the runs on each side of the text it matched, as
//!   the punctuation of a hand-written parser does;
//! - the node `whitespace` declares is the trivia itself, and owns nothing;
//! - a terminal another pattern reaches by name owns nothing, since its text is
//!   read into the text of the rule that reaches it, and a run taken there would
//!   end up inside that other token;
//! - a terminal that could match a character the trivia matches owns nothing,
//!   since the run before it would eat what it stands there to take.
//!
//! Between two operands neither of which owns the run before it, the sequence
//! that reads them carries one optional run of the trivia. An operand that may
//! match nothing — an option, a repetition, a bound — never owns a run, so it is
//! always content to let its neighbour keep it.
//!
//! None of this changes what a grammar accepts: the first operand that reaches a
//! run keeps it, and keeps all of it. What it decides is where in the tree the
//! run is filed away, which is why a rule over nodes names the trivia of its own
//! language at most once.
//!
//! The `codegen` module's own documents say how the rule is decided and what the
//! generated types look like once it has been.
//!
//! # Two phases, two features
//!
//! The crate does two jobs that have nothing to do with each other, and the
//! features of `Cargo.toml` say so:
//!
//! - `runtime`, the [`runtime`] module, is what a grammar the generator writes
//!   is *compiled against*: the [`GeneratedInput`](runtime::GeneratedInput)
//!   every generated type is bounded by, and nothing else. Two things are
//!   deliberately not here. What a `<NAME>` in a pattern calls is the caller's
//!   link, and unsyn ships no answer to it. Where a word of the described
//!   language ends is the spec's business, and unsyn assumes no boundary for it.
//!   The module knows nothing of the DSL, and needs nothing but a character
//!   stream.
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

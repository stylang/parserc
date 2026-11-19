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

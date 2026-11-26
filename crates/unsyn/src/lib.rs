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
//! crate's sources. The `compiler` module's documents go on from there to how
//! each is implemented.
//!
//! # One job, one feature
//!
//! The crate does one job, and the `compiler` feature says so: the DSL itself —
//! the reader, the analyzer and the generator, in the stages its own module
//! document lays out, over `quote` and `proc_macro2`.
//!
//! There is no second feature for a runtime, because there is no runtime to
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
//! front end (`lexer`, the nodes and the visitor) is not a hand-written file: it
//! is what a build writes for the spec, into `OUT_DIR`, by the released
//! generator. [`build.rs`](https://github.com/stylang/stylang/blob/main/crates/unsyn/build.rs)
//! says how, and why the generator run there is the published one rather than
//! the one in `src/` — a crate cannot compile the code that decides its own
//! shape, so a change to what a build *writes* lands in the release after the one
//! whose build ran the previous generator.
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
//!
//! One consequence of the build-dependency is worth knowing before typing a
//! command: two packages named `unsyn` are in the graph — this one, and the
//! pinned one the build runs — so a `-p unsyn` filter is ambiguous and cargo asks
//! for a version, `cargo test -p unsyn@0.14.1`.
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod codegen;
pub mod decode;
pub mod error;
pub mod input;
pub mod read;
pub mod scan;
pub mod semantic;

// The front end of the language, in the code a build wrote for `unsyn.us`. These
// are not hand-maintained files: the spec at the root of this crate is the source
// of every type below, and `build.rs` says how the released generator is the one
// that wrote them. A `use super::input::UnsynInput` inside one of them is this
// crate's own `input`, and a `super::lexer` is this crate's own `lexer` — the
// siblings here are the siblings a build writes.
//
// `include!` rather than `#[path]` because a `path` attribute takes a literal
// alone, and the directory a build writes into is known only once the build has
// run.
//
// Only the error's own file is not mounted here: it lives inside `error`, as
// `error::generated`, so that the generated error and the wrapper around it are
// one module's two parts rather than two modules with nearly one name.
//
// No allowance is written on these modules. The released generator's output has
// three known defects — an error import in a file that reports no refusal, a
// character class spelled as its range where `char` names it, and field types
// nested as tuples because the spec gave those groups no left side. The first two
// are fixed in `src/codegen.rs`, each pinned by a test; the third is now carried
// by the generated code itself, on the items that need it. Until this crate is
// built by a generator holding those fixes, the output compiled here still warns,
// and that warning is the honest record of a release being one behind its own
// generator — silencing it here would hide it without fixing anything.
pub mod lexer {
    include!(concat!(env!("OUT_DIR"), "/unsyn/lexer.rs"));
}

pub mod syntax {
    include!(concat!(env!("OUT_DIR"), "/unsyn/syntax.rs"));
}

pub mod visit {
    include!(concat!(env!("OUT_DIR"), "/unsyn/visit.rs"));
}

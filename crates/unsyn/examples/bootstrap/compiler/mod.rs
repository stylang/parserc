//! The crate's own pipeline, over the front end the generator wrote.
//!
//! This tree mirrors [`src/compiler`](../../../src/compiler) module for module.
//! Only the three slots the front end fills differ: they name the generated
//! file instead of the hand-written sources, so the analysis and the generation
//! are handed the tree the generator's own parser built. No `cfg` appears in
//! those files and none is planned — a switch there would be an admission that
//! the two trees are two.
//!
//! Which modules are reached by path and which are re-exported is not a matter
//! of taste. A module that names a node of the front end has to be compiled
//! again here, or `super::syntax` inside it would still mean the hand tree.
//! A module that names nothing of it — the stream, the reports, the decoding of
//! an escape — is the *same trait* or the *same enum* in both halves of the
//! loop only if it is compiled once, so those are re-exported from the crate
//! rather than copied. Copying `input` here would make the harness's
//! `UnsynInput` a second trait, and the file a build wrote, which imports the
//! crate's own, would satisfy nothing.
//!
//! See [`BOOTSTRAP_LOG.md`](../../../BOOTSTRAP_LOG.md) for how far this is
//! expected to compile today: the front-end slots are honest, the stages that
//! make the analysis agree with them are the work.

/// The nodes, the parsers and the visitor a build wrote for `unsyn.us`.
///
/// One file, because a build writes one file. The slots below are what let that
/// one file stand in for the parts the hand front end was cut into.
#[path = "../../../tests/generated/unsyn_us.rs"]
pub mod generated;

/// A terminal, in the generated telling. The split of terminals from nodes
/// carries nothing a tree can tell; the hand file is cut that way, so the
/// slots are cut that way.
pub mod lexer {
    pub use super::generated::*;
}

/// A node of the tree, in the same telling.
pub mod syntax {
    pub use super::generated::*;
}

/// The descents of the visitor, over the nodes this harness names.
///
/// Reached by path like `read`/`semantic`/`codegen`: the walk names a node of
/// the front end at every step, so one compilation in the crate would pin it to
/// the hand tree and its shape differences would never be measured here.
#[path = "../../../src/compiler/visit.rs"]
pub mod visit;

// The reports say nothing about which tree they were handed, and must be
// compiled exactly once: the stream the generated file names carries
// `Error = ::unsyn::compiler::errors::UnsynError` written into it in absolute
// path, so a second copy here would be a different enum that the generated
// `UnsynInput` refuses to accept. `errors` therefore stays a re-export.
//
// The decoding is the other case. It names nodes of the front end — the literal
// whose digits it reads — so a single compilation pins it to the hand tree and
// every call the recompiled analysis and generation make of it mismatches, same
// name and different owner. It is reached by path so it follows the slot, and it
// names no error type at all, so no second enum comes of it.
pub use ::unsyn::compiler::errors;

#[path = "../../../src/compiler/decode.rs"]
pub mod decode;

/// The stream the generated file names: the trait it emitted, and its alias.
pub mod input {
    pub use super::generated::*;
}

#[path = "../../../src/compiler/read.rs"]
pub mod read;

#[path = "../../../src/compiler/semantic.rs"]
pub mod semantic;

#[path = "../../../src/compiler/codegen.rs"]
pub mod codegen;

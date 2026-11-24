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

/// The descents of the visitor the generated file ships.
pub mod visit {
    pub use super::generated::*;
}

// The error and the decoding say nothing about which tree they were handed:
// one compilation in the crate, the same types here and in the generated
// file. The *stream* is different — the file writes the trait itself now, so
// this slot names what it wrote, and the analysis recompiled here is written
// against that same one trait. Copying `input` would have made a second.
pub use ::unsyn::compiler::{decode, errors};

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

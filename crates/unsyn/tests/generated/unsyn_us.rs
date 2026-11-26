//! The code of `unsyn.us`, as the generator writes it, in the files a build
//! makes of it.
//!
//! This is the keeping, not the code: the four files below are what
//! [`to_dir`](unsyn::codegen::Builder::to_dir) writes, brought under one module
//! so that a test can name the tree as `generated::Stmt` rather than by which of
//! them holds it. `error.rs` is absent because this build was told the error
//! stands in the crate — see
//! [`Builder::error_at`](unsyn::codegen::Builder::error_at) — and a file that
//! reaches a neighbour by `super::` needs the same parent these four share.
//!
//! Each file is generated: see `tests/unsyn_us.rs`. Do not edit them.
#[path = "unsyn_us/input.rs"]
pub mod input;

#[path = "unsyn_us/lexer.rs"]
pub mod lexer;

#[path = "unsyn_us/syntax.rs"]
pub mod syntax;

#[path = "unsyn_us/visit.rs"]
pub mod visit;

pub use input::*;
pub use lexer::*;
pub use syntax::*;
pub use visit::*;

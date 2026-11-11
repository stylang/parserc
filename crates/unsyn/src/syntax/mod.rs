//! The concrete syntax tree of the `unsyn` language.
//!
//! Where [`crate::lexical`] is what a source file is written with, this module is
//! what a source file says: [`Crate`] for the file, [`Stmt`] for one
//! definition of it, and [`Expr`] for the pattern a definition gives its
//! node.
//!
//! The types here are the grammar, in the order of the precedence it reads with:
//! an expression is alternatives joined by `|`, an alternative is a sequence of
//! operands, and an operand is a single thing with at most one suffix. A node of
//! any of them keeps the tokens it was read from, so the tree is a view of the
//! source rather than a summary of it: every span in a file can be found again
//! from the tree alone.
//!
//! These are the types the DSL itself describes, so `bootstrap.uns` beside this
//! crate is a definition of them, checked by the tests of `tests/bootstrap.rs`.

mod expr;
pub use expr::*;

mod item;
pub use item::*;

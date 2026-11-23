//! Reading a tree that a build wrote.
//!
//! [`lexer`](crate::compiler::lexer) and [`syntax`](crate::compiler::syntax) hold
//! the tree and nothing else: types, their fields, and the parsers the derive
//! builds from them. No `impl` block of ours stands beside those types, because
//! [`unsyn`](crate) is a generator for *any* language and can not know what a
//! build of some other file will want to ask of its nodes. Everything the passes
//! here do read — walking a sequence, counting the members of a set, saying which
//! kind of definition a statement declares — lives in this module as a plain
//! function taking a node.
//!
//! The line is drawn by who owns the knowledge, not by how small the function is:
//!
//! - a shape a production writes, such as `X+` becoming a non-empty run, is read
//!   here and could as well be read at the call site;
//! - a meaning a word carries, such as `lexer` declaring a terminal, is a fact of
//!   *this* DSL and appears nowhere in the tree — see [`Decl`].
//!
//! A generated file will therefore ship types, parsers and a visitor and no
//! method of its own, and the analysis above it will read it exactly as this one
//! does.

use parserc::syntax::Syntax;

use super::input::UnsynInput;
use super::lexer::LeadingKeywords;
use super::syntax::{Expr, ExprNoTopAlt, OneOrMore, Set, SetItem, Stmt};

// ---------------------------------------------------------------------------
// The shapes a production writes
// ---------------------------------------------------------------------------

/// The operands of a `X+` sequence, in the order they were written.
#[inline]
pub fn elements<I, X>(run: &OneOrMore<I, X>) -> impl Iterator<Item = &X>
where
    I: UnsynInput,
    X: Syntax<I>,
{
    std::iter::once(&run.first).chain(run.rest.iter().map(|(_, element)| element))
}

/// The alternatives of an expression, in the order they were written.
///
/// The `|` that introduced an alternative is trivia of the tree rather than part
/// of the alternative, so it is dropped here.
#[inline]
pub fn branches<'a, I: UnsynInput>(
    expr: &'a Expr<I>,
) -> impl Iterator<Item = &'a OneOrMore<I, ExprNoTopAlt<I>>> + 'a {
    std::iter::once(&expr.first).chain(expr.rest.iter().map(|(_, _, _, branch)| branch))
}

/// Whether the expression offers exactly one way of being read.
#[inline]
pub fn is_single<I: UnsynInput>(expr: &Expr<I>) -> bool {
    expr.rest.is_empty()
}

/// The members of a set, in the order they were written.
///
/// The commas between them are trivia of the same kind the bars of a choice are,
/// and the one the closing bracket may keep is the last of the run rather than a
/// member after it.
#[inline]
pub fn members<'a, I: UnsynInput>(set: &'a Set<I>) -> impl Iterator<Item = &'a SetItem<I>> + 'a {
    set.items
        .iter()
        .map(|(_, item, _, _)| item)
        .chain(set.tail.iter().map(|(_, item)| item))
}

// ---------------------------------------------------------------------------
// What a word of the DSL means
// ---------------------------------------------------------------------------

/// The kind of node a definition declares, which is the word it opens with.
///
/// Which of the three words means which is a fact about this DSL and about no
/// other: a tree says only that some alternative of [`LeadingKeywords`] was read,
/// and this is where that stops being a shape and becomes an intention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decl {
    /// `whitespace`: the trivia the language separates its nodes with.
    Whitespace,
    /// `lexer`: a terminal, a pattern over characters.
    Lexer,
    /// `syntax`: a node of the tree, a pattern over other nodes.
    Syntax,
}

/// The kind of node a definition declares.
///
/// The word the definition opens with is the only thing that says so, and it is
/// the variant of [`LeadingKeywords`] that was read — the three words of the spec
/// are the three alternatives of that rule.
#[inline]
pub fn decl<I: UnsynInput>(stmt: &Stmt<I>) -> Decl {
    match stmt.keyword {
        LeadingKeywords::Lexer(_) => Decl::Lexer,
        LeadingKeywords::Syntax(_) => Decl::Syntax,
        LeadingKeywords::Whitespace(_) => Decl::Whitespace,
    }
}

//! Reading the tree in source order, one method per node type.
//!
//! [`Visit`] is the walk: it enters every node of a file, tokens and whitespace
//! included, in the order the file was written. A pass implements the methods it
//! cares about and lets the defaults carry it down, exactly as `syn::Visit`
//! works, so nothing here reaches past the node it was handed: no parent
//! pointer, no path of names to walk down, and no erased handle — every
//! signature names a concrete type of [`crate::syntax`] or
//! [`crate::lexical`].
//!
//! ## The one list, and why it has to stay one
//!
//! A bootstrapped parser is a second tree: the same language written with the
//! types this crate generates from `bootstrap.uns`. The two trees share no Rust
//! type, so no `PartialEq` compares them and no pass can be written against both
//! at once. What they must share is *this interface*: one method per node, the
//! same method names, the same descent. Then a pass written for one tree reads
//! the same language in the other, and the equivalence the bootstrap is judged
//! by is a judgement about the language rather than about which types happened to
//! be written for it.
//!
//! So the list below is the contract, and it is derived rather than chosen:
//!
//! * every node type of the tree gets one method, named for the type in
//!   snake_case, and *only* node types get one;
//! * every method's default body is one line, `walk::walk_x(self, node)`, and
//!   every `walk_x` says what that node holds and in which order — so the pair
//!   (method, walk) is the whole of the knowledge a walk has of a node, and a
//!   generator that can name a type can emit both halves;
//! * the order a walk enters the parts of a node is the order they stand in the
//!   file, which for [`parserc::syntax::Delimiter`] is `start`, `body`, `end`
//!   and not the order of its three fields;
//! * the leaf is [`Visit::visit_input`], which says the text of one token where
//!   it stands, and no method is allowed to skip a token.
//!
//! Which node types exist is not a fact about this module, so it is not written
//! down here either. `tests/visit.rs` reads the interface out of the source of this
//! module and the node types out of the tree of [`crate::syntax`] and
//! [`crate::lexical`], and says so when the two stop being one list; the same file
//! walks a source that uses every form the language has, and checks that every
//! method is entered once and that the walk reads every token of that file on its
//! way through them.
//!
//! ## What is deliberately not here
//!
//! `syn` also generates `VisitMut` and `Fold`. Those are for a pass that
//! rewrites a tree, and no pass here does: the analyses report and the generator
//! emits. They will come from the same list when a pass needs them, which is why
//! the list is kept in the form a generator can read.

pub mod walk;

use crate::{
    input::UnsynInput,
    lexical::{
        S,
        comments::{InnerBlockDoc, InnerLineDoc, OuterBlockDoc, OuterLineDoc},
        ident::Ident,
        keyword::{Concat, Except, Followed, Lexer, Syntax, Whitespace},
        lit::{AsciiEscape, LitDec, LitStr, LitUnicode, QuoteEscape, StrSegment, UnicodeEscape},
        punct::{
            ArrowRight, BraceEnd, BraceStart, BracketEnd, BracketStart, Comma, Gt, Lt, Minus, Or,
            ParenEnd, ParenStart, Plus, Question, Semi, Star, Tilde,
        },
    },
    syntax::{
        Crate, Expr, ExprNoTopAlt, ExprNoTopAlts, ExprWithSuffix, ExprWithoutSuffix, InnerDoc,
        Item, LeadingKey, OuterDoc, Range, Repeat, SetItem, Stmt,
    },
};

/// The walk itself: one method per node type, and defaults that descend.
///
/// A method's default body is the descent into the node, written as
/// [`walk::walk_crate`] and its siblings. Overriding a method and *not* calling
/// the matching `walk_*` function prunes the tree below that node, which is how
/// a pass that only wants the statements of a file never enters an expression.
pub trait Visit<'ast, I: UnsynInput> {
    /// A whole source file: the documents that open it, then its items.
    fn visit_crate(&mut self, node: &'ast Crate<I>) {
        walk::walk_crate(self, node);
    }

    /// One item of a file: a document, a run of whitespace, or a definition.
    fn visit_item(&mut self, node: &'ast Item<I>) {
        walk::walk_item(self, node);
    }

    /// A document that describes the file, in either of its two spellings.
    fn visit_inner_doc(&mut self, node: &'ast InnerDoc<I>) {
        walk::walk_inner_doc(self, node);
    }

    /// A document that describes the statement after it.
    fn visit_outer_doc(&mut self, node: &'ast OuterDoc<I>) {
        walk::walk_outer_doc(self, node);
    }

    /// One definition: `lexer TEXT -> …;`.
    fn visit_stmt(&mut self, node: &'ast Stmt<I>) {
        walk::walk_stmt(self, node);
    }

    /// The keyword a definition opens with, which is what says what it defines.
    fn visit_leading_key(&mut self, node: &'ast LeadingKey<I>) {
        walk::walk_leading_key(self, node);
    }

    /// The pattern a definition gives its node: the alternatives joined by `|`.
    fn visit_expr(&mut self, node: &'ast Expr<I>) {
        walk::walk_expr(self, node);
    }

    /// The operands of one alternative, written one after another.
    fn visit_expr_no_top_alts(&mut self, node: &'ast ExprNoTopAlts<I>) {
        walk::walk_expr_no_top_alts(self, node);
    }

    /// One operand of an alternative, with or without a suffix.
    fn visit_expr_no_top_alt(&mut self, node: &'ast ExprNoTopAlt<I>) {
        walk::walk_expr_no_top_alt(self, node);
    }

    /// An operand with the one suffix on it: `*`, `?`, `+`, `{…}`, `except`,
    /// `followed` or `concat`.
    fn visit_expr_with_suffix(&mut self, node: &'ast ExprWithSuffix<I>) {
        walk::walk_expr_with_suffix(self, node);
    }

    /// An operand that stands alone: a set, a complement, a call, a group, a
    /// literal or a reference.
    fn visit_expr_without_suffix(&mut self, node: &'ast ExprWithoutSuffix<I>) {
        walk::walk_expr_without_suffix(self, node);
    }

    /// One member of a set: a literal, a reference, or a range.
    fn visit_set_item(&mut self, node: &'ast SetItem<I>) {
        walk::walk_set_item(self, node);
    }

    /// The `a-b` bounds of a set member written with a `-`.
    fn visit_range(&mut self, node: &'ast Range<I>) {
        walk::walk_range(self, node);
    }

    /// The `{…}` on an operand, braces and all, counting how many times it may
    /// match.
    fn visit_repeat(&mut self, node: &'ast Repeat<I>) {
        walk::walk_repeat(self, node);
    }

    /// An identifier: the name of a definition, of a node referred to, or of an
    /// external called.
    fn visit_ident(&mut self, node: &'ast Ident<I>) {
        walk::walk_ident(self, node);
    }

    /// A run of whitespace, which is a token because a sequence is told apart
    /// by nothing else.
    fn visit_s(&mut self, node: &'ast S<I>) {
        walk::walk_s(self, node);
    }

    /// `'…'`, one string or one character of a set.
    fn visit_lit_str(&mut self, node: &'ast LitStr<I>) {
        walk::walk_lit_str(self, node);
    }

    /// One item of the content of a string: plain text or an escape.
    fn visit_str_segment(&mut self, node: &'ast StrSegment<I>) {
        walk::walk_str_segment(self, node);
    }

    /// An escape of the `\n` and `\x41` forms.
    fn visit_ascii_escape(&mut self, node: &'ast AsciiEscape<I>) {
        walk::walk_ascii_escape(self, node);
    }

    /// A `\u{…}` escape: its three tokens, the digits between the two runs of
    /// punctuation.
    fn visit_unicode_escape(&mut self, node: &'ast UnicodeEscape<I>) {
        walk::walk_unicode_escape(self, node);
    }

    /// An escaped `'` inside a string.
    fn visit_quote_escape(&mut self, node: &'ast QuoteEscape<I>) {
        walk::walk_quote_escape(self, node);
    }

    /// `U+0041`, a code point written in full.
    fn visit_lit_unicode(&mut self, node: &'ast LitUnicode<I>) {
        walk::walk_lit_unicode(self, node);
    }

    /// `12`, a count written in full.
    fn visit_lit_dec(&mut self, node: &'ast LitDec<I>) {
        walk::walk_lit_dec(self, node);
    }

    /// A `//!` document.
    fn visit_inner_line_doc(&mut self, node: &'ast InnerLineDoc<I>) {
        walk::walk_inner_line_doc(self, node);
    }

    /// A `/*!` document.
    fn visit_inner_block_doc(&mut self, node: &'ast InnerBlockDoc<I>) {
        walk::walk_inner_block_doc(self, node);
    }

    /// A `///` document.
    fn visit_outer_line_doc(&mut self, node: &'ast OuterLineDoc<I>) {
        walk::walk_outer_line_doc(self, node);
    }

    /// A `/**` document.
    fn visit_outer_block_doc(&mut self, node: &'ast OuterBlockDoc<I>) {
        walk::walk_outer_block_doc(self, node);
    }

    // Every one of the tokens below holds the same three things: the whitespace
    // before it, which may be absent, its own text, and the whitespace after it,
    // which may be absent. Each still gets a method and a walk of its own,
    // because a generator names a type and emits the pair; nothing here is
    // allowed to depend on knowing that two of them are shaped alike.
    #[rustfmt::skip]
    fn visit_plus(&mut self, node: &'ast Plus<I>) { walk::walk_plus(self, node) }
    #[rustfmt::skip]
    fn visit_minus(&mut self, node: &'ast Minus<I>) { walk::walk_minus(self, node) }
    #[rustfmt::skip]
    fn visit_star(&mut self, node: &'ast Star<I>) { walk::walk_star(self, node) }
    #[rustfmt::skip]
    fn visit_question(&mut self, node: &'ast Question<I>) { walk::walk_question(self, node) }
    #[rustfmt::skip]
    fn visit_tilde(&mut self, node: &'ast Tilde<I>) { walk::walk_tilde(self, node) }
    #[rustfmt::skip]
    fn visit_or(&mut self, node: &'ast Or<I>) { walk::walk_or(self, node) }
    #[rustfmt::skip]
    fn visit_paren_start(&mut self, node: &'ast ParenStart<I>) { walk::walk_paren_start(self, node) }
    #[rustfmt::skip]
    fn visit_paren_end(&mut self, node: &'ast ParenEnd<I>) { walk::walk_paren_end(self, node) }
    #[rustfmt::skip]
    fn visit_bracket_start(&mut self, node: &'ast BracketStart<I>) { walk::walk_bracket_start(self, node) }
    #[rustfmt::skip]
    fn visit_bracket_end(&mut self, node: &'ast BracketEnd<I>) { walk::walk_bracket_end(self, node) }
    #[rustfmt::skip]
    fn visit_brace_start(&mut self, node: &'ast BraceStart<I>) { walk::walk_brace_start(self, node) }
    #[rustfmt::skip]
    fn visit_brace_end(&mut self, node: &'ast BraceEnd<I>) { walk::walk_brace_end(self, node) }
    #[rustfmt::skip]
    fn visit_lt(&mut self, node: &'ast Lt<I>) { walk::walk_lt(self, node) }
    #[rustfmt::skip]
    fn visit_gt(&mut self, node: &'ast Gt<I>) { walk::walk_gt(self, node) }
    #[rustfmt::skip]
    fn visit_comma(&mut self, node: &'ast Comma<I>) { walk::walk_comma(self, node) }
    #[rustfmt::skip]
    fn visit_arrow_right(&mut self, node: &'ast ArrowRight<I>) { walk::walk_arrow_right(self, node) }
    #[rustfmt::skip]
    fn visit_semi(&mut self, node: &'ast Semi<I>) { walk::walk_semi(self, node) }

    // A keyword owns the whitespace after it and never the whitespace before it,
    // because a definition starts at its own keyword.
    #[rustfmt::skip]
    fn visit_whitespace(&mut self, node: &'ast Whitespace<I>) { walk::walk_whitespace(self, node) }
    #[rustfmt::skip]
    fn visit_lexer(&mut self, node: &'ast Lexer<I>) { walk::walk_lexer(self, node) }
    #[rustfmt::skip]
    fn visit_syntax(&mut self, node: &'ast Syntax<I>) { walk::walk_syntax(self, node) }
    #[rustfmt::skip]
    fn visit_followed(&mut self, node: &'ast Followed<I>) { walk::walk_followed(self, node) }
    #[rustfmt::skip]
    fn visit_concat(&mut self, node: &'ast Concat<I>) { walk::walk_concat(self, node) }
    #[rustfmt::skip]
    fn visit_except(&mut self, node: &'ast Except<I>) { walk::walk_except(self, node) }

    /// The text of one token where it stands: the leaf of the walk.
    ///
    /// Every token of a file is visited exactly once, in order, so a pass that
    /// collects these has the file again, and a pass that compares two
    /// collections has compared what the two trees *read* rather than what they
    /// chose to store.
    fn visit_input(&mut self, input: &'ast I);
}
